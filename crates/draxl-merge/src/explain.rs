use crate::model::{
    Conflict, ConflictClass, ConflictCode, ConflictSide, ReplayFailure, ReplayOrder, ReplayStage,
};
use crate::render::{node_label, path_label, ranked_dest_label, summarize_side};
use draxl_patch::{PatchOp, RankedDest, SlotRef};

pub(crate) fn same_node_write_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    node_id: &str,
) -> Conflict {
    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::SameNodeWrite,
        summary: format!("both patch streams write the same node target `@{node_id}`"),
        detail: format!(
            "The left and right patch streams both modify the same node shell `@{node_id}`. \
             That prevents deterministic auto-merge because the final AST depends on which rewrite wins."
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some(
            "combine these edits into one agreed rewrite for the shared node target".to_owned(),
        ),
    }
}

pub(crate) fn same_scalar_path_write_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    node_id: &str,
    segments: &[String],
) -> Conflict {
    let path = path_label(node_id, segments);
    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::SameScalarPathWrite,
        summary: format!("both patch streams write the same scalar path `{path}`"),
        detail: format!(
            "The left and right patch streams both write `{path}`. \
             That is a hard conflict because there is no single deterministic value unless the updates are reconciled."
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some("choose one final value for the shared scalar path".to_owned()),
    }
}

pub(crate) fn same_single_slot_write_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    slot: &SlotRef,
) -> Conflict {
    let target = crate::render::slot_ref_label(slot);
    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::SameSingleSlotWrite,
        summary: format!("both patch streams write the same single-child slot `{target}`"),
        detail: format!(
            "The left and right patch streams both assign the single-child slot `{target}`. \
             That makes the occupant ambiguous, so the merge cannot be treated as deterministically clean."
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some("pick one final occupant for the slot or merge the occupant edits manually".to_owned()),
    }
}

pub(crate) fn same_ranked_position_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    dest: &RankedDest,
) -> Conflict {
    let target = ranked_dest_label(dest);
    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::SameRankedPosition,
        summary: format!("both patch streams target the same ranked position `{target}`"),
        detail: format!(
            "The left and right patch streams both write `{target}`. \
             That creates a hard conflict because the same ranked destination cannot host two independent writes cleanly."
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some("assign distinct ranks or combine the inserted content into one change".to_owned()),
    }
}

pub(crate) fn replay_failure_conflict(
    failure: &ReplayFailure,
    left: &[PatchOp],
    right: &[PatchOp],
) -> Conflict {
    let (summary, detail) = replay_failure_text(failure);
    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::ReplayFailure,
        summary,
        detail,
        left: replay_relevant_sides(failure, left, true),
        right: replay_relevant_sides(failure, right, false),
        remediation: Some(
            "rebase the later patch stream onto the earlier one and regenerate its patch ops"
                .to_owned(),
        ),
    }
}

pub(crate) fn non_convergent_replay_conflict(left: &[PatchOp], right: &[PatchOp]) -> Conflict {
    let left_sides = left
        .iter()
        .enumerate()
        .map(|(index, op)| summarize_side(index, op))
        .collect::<Vec<_>>();
    let right_sides = right
        .iter()
        .enumerate()
        .map(|(index, op)| summarize_side(index, op))
        .collect::<Vec<_>>();

    Conflict {
        class: ConflictClass::Hard,
        code: ConflictCode::NonConvergentReplay,
        summary: "the two replay orders produce different final ASTs".to_owned(),
        detail: format!(
            "Both replay orders succeeded, but they did not converge to the same canonical AST. \
             That is a hard conflict because auto-merge would depend on replay order.\n\nleft stream:\n{}\n\nright stream:\n{}",
            render_side_list(&left_sides),
            render_side_list(&right_sides),
        ),
        left: left_sides,
        right: right_sides,
        remediation: Some(
            "rewrite the overlapping patch ops so both replay orders converge to one result"
                .to_owned(),
        ),
    }
}

pub(crate) fn binding_rename_vs_initializer_change_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    let_id: &str,
    binding_id: &str,
) -> Conflict {
    Conflict {
        class: ConflictClass::Semantic,
        code: ConflictCode::BindingRenameVsInitializerChange,
        summary: format!(
            "one side renames binding `{}` while the other changes initializer meaning in `{}`",
            node_label(binding_id),
            node_label(let_id)
        ),
        detail: format!(
            "These edits are structurally mergeable, but they should be reviewed together. \
             The left patch stream renames the binding `{}` in let statement `{}`, while the right patch stream changes the initializer subtree of the same let. \
             That means the merged code may keep a name whose meaning has shifted.",
            node_label(binding_id),
            node_label(let_id)
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some(
            "review the binding name against the new initializer meaning and rename or rewrite them together"
                .to_owned(),
        ),
    }
}

pub(crate) fn call_callee_vs_argument_change_conflict(
    left_index: usize,
    left_op: &PatchOp,
    right_index: usize,
    right_op: &PatchOp,
    call_id: &str,
) -> Conflict {
    Conflict {
        class: ConflictClass::Semantic,
        code: ConflictCode::CallCalleeVsArgumentChange,
        summary: format!(
            "one side changes the callee while the other changes argument meaning in `{}`",
            node_label(call_id)
        ),
        detail: format!(
            "These edits are structurally mergeable, but they should be reviewed together. \
             The left patch stream changes the callee region of call `{}`, while the right patch stream changes an argument region of the same call. \
             That means the merged code may pair a new call contract with an argument value that still follows the old representation.",
            node_label(call_id)
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some(
            "review the call contract against the merged argument representation and update them together"
                .to_owned(),
        ),
    }
}

fn replay_failure_text(failure: &ReplayFailure) -> (String, String) {
    let order = match failure.order {
        ReplayOrder::LeftThenRight => "left then right",
        ReplayOrder::RightThenLeft => "right then left",
    };

    let stage = match failure.stage {
        ReplayStage::LeftOp(index) => format!("while applying left op {index}"),
        ReplayStage::RightOp(index) => format!("while applying right op {index}"),
        ReplayStage::Validation => "during post-replay validation".to_owned(),
    };

    (
        format!("replay failed {stage} in the `{order}` order"),
        format!(
            "The hard-conflict checker could not complete the `{order}` replay {stage}. \
             The underlying failure was:\n{}",
            failure.message
        ),
    )
}

fn replay_relevant_sides(
    failure: &ReplayFailure,
    ops: &[PatchOp],
    is_left: bool,
) -> Vec<ConflictSide> {
    match failure.stage {
        ReplayStage::LeftOp(index) if is_left => ops
            .get(index)
            .map(|op| vec![summarize_side(index, op)])
            .unwrap_or_default(),
        ReplayStage::RightOp(index) if !is_left => ops
            .get(index)
            .map(|op| vec![summarize_side(index, op)])
            .unwrap_or_default(),
        ReplayStage::Validation => ops
            .iter()
            .enumerate()
            .map(|(index, op)| summarize_side(index, op))
            .collect(),
        _ => Vec::new(),
    }
}

fn render_side_list(sides: &[ConflictSide]) -> String {
    sides
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}
