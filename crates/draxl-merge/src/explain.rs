use crate::model::{
    Conflict, ConflictClass, ConflictCode, ConflictOwner, ConflictRegion, ConflictSide,
    ReplayFailure, ReplayOrder, ReplayStage,
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
        owner: None,
        left_regions: Vec::new(),
        right_regions: Vec::new(),
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
    left_region: ConflictRegion,
    right_index: usize,
    right_op: &PatchOp,
    right_region: ConflictRegion,
    let_id: &str,
    binding_id: &str,
) -> Conflict {
    Conflict {
        class: ConflictClass::Semantic,
        code: ConflictCode::BindingRenameVsInitializerChange,
        owner: Some(ConflictOwner::Binding {
            let_id: let_id.to_owned(),
            binding_id: binding_id.to_owned(),
        }),
        left_regions: vec![left_region],
        right_regions: vec![right_region],
        summary: format!(
            "one side renames binding `{}` while the other changes initializer meaning in `{}`",
            node_label(binding_id),
            node_label(let_id)
        ),
        detail: format!(
            "These edits are structurally mergeable, but they should be reviewed together. \
             One patch stream renames the binding `{}` in let statement `{}`, while the other changes the initializer subtree of the same let. \
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

pub(crate) fn parameter_type_vs_body_interpretation_change_conflict(
    left_index: usize,
    left_op: &PatchOp,
    left_region: ConflictRegion,
    right_index: usize,
    right_op: &PatchOp,
    right_region: ConflictRegion,
    fn_id: &str,
    param_id: &str,
    param_name: &str,
) -> Conflict {
    Conflict {
        class: ConflictClass::Semantic,
        code: ConflictCode::ParameterTypeVsBodyInterpretationChange,
        owner: Some(ConflictOwner::Parameter {
            fn_id: fn_id.to_owned(),
            param_id: param_id.to_owned(),
            param_name: param_name.to_owned(),
        }),
        left_regions: vec![left_region],
        right_regions: vec![right_region],
        summary: format!(
            "one side changes parameter contract `{}` while the other changes body interpretation in `{}`",
            node_label(param_id),
            node_label(fn_id)
        ),
        detail: format!(
            "These edits are structurally mergeable, but they should be reviewed together. \
             One patch stream changes the type contract for parameter `{}` in function `{}`, while the other changes body logic that still interprets that parameter. \
             That means the merged code may keep body behavior that no longer matches the parameter contract.",
            node_label(param_id),
            node_label(fn_id)
        ),
        left: vec![summarize_side(left_index, left_op)],
        right: vec![summarize_side(right_index, right_op)],
        remediation: Some(
            "review the parameter contract against the merged body logic and update them together"
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
