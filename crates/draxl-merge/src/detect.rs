use crate::context::TreeContext;
use crate::explain::{
    binding_rename_vs_initializer_change_conflict, non_convergent_replay_conflict,
    parameter_type_vs_body_interpretation_change_conflict, replay_failure_conflict,
    same_node_write_conflict, same_ranked_position_conflict, same_scalar_path_write_conflict,
    same_single_slot_write_conflict,
};
use crate::model::{Conflict, ConflictReport, ReplayFailure, ReplayOrder, ReplayStage};
use crate::semantic::{extract_semantic_changes, SemanticChange, SemanticOwner, SemanticRegion};
use draxl_ast::File;
use draxl_patch::{apply_op, PatchDest, PatchOp, RankedDest, SlotRef};
use draxl_printer::canonicalize_file;
use draxl_validate::validate_file;

/// Checks both hard and semantic conflicts against the same base.
pub fn check_conflicts(base: &File, left: &[PatchOp], right: &[PatchOp]) -> ConflictReport {
    let hard = check_hard_conflicts(base, left, right);
    if hard.has_conflicts() {
        return hard;
    }

    ConflictReport {
        conflicts: classify_semantic_conflicts(base, left, right),
    }
}

/// Checks whether two patch streams have hard conflicts against the same base.
pub fn check_hard_conflicts(base: &File, left: &[PatchOp], right: &[PatchOp]) -> ConflictReport {
    let left_then_right = replay(base, ReplayOrder::LeftThenRight, left, right);
    let right_then_left = replay(base, ReplayOrder::RightThenLeft, right, left);

    if let (Ok(left_then_right), Ok(right_then_left)) = (&left_then_right, &right_then_left) {
        if canonicalize_file(left_then_right).without_spans()
            == canonicalize_file(right_then_left).without_spans()
        {
            return ConflictReport::default();
        }
    }

    let mut conflicts = classify_pairwise_conflicts(left, right);

    if conflicts.is_empty() {
        match (&left_then_right, &right_then_left) {
            (Ok(_), Ok(_)) => conflicts.push(non_convergent_replay_conflict(left, right)),
            (Err(failure), Ok(_)) | (Ok(_), Err(failure)) => {
                conflicts.push(replay_failure_conflict(failure, left, right));
            }
            (Err(left_failure), Err(right_failure)) => {
                conflicts.push(replay_failure_conflict(left_failure, left, right));
                if left_failure != right_failure {
                    conflicts.push(replay_failure_conflict(right_failure, left, right));
                }
            }
        }
    }

    ConflictReport { conflicts }
}

fn classify_pairwise_conflicts(left: &[PatchOp], right: &[PatchOp]) -> Vec<Conflict> {
    let mut conflicts = Vec::new();

    for (left_index, left_op) in left.iter().enumerate() {
        for (right_index, right_op) in right.iter().enumerate() {
            if let Some(dest) = same_ranked_dest(left_op, right_op) {
                conflicts.push(same_ranked_position_conflict(
                    left_index,
                    left_op,
                    right_index,
                    right_op,
                    &dest,
                ));
                continue;
            }

            if let Some(slot) = same_single_slot_dest(left_op, right_op) {
                conflicts.push(same_single_slot_write_conflict(
                    left_index,
                    left_op,
                    right_index,
                    right_op,
                    &slot,
                ));
                continue;
            }

            if let Some((node_id, segments)) = same_scalar_path(left_op, right_op) {
                conflicts.push(same_scalar_path_write_conflict(
                    left_index,
                    left_op,
                    right_index,
                    right_op,
                    &node_id,
                    &segments,
                ));
                continue;
            }

            if let Some(node_id) = same_node_target(left_op, right_op) {
                conflicts.push(same_node_write_conflict(
                    left_index,
                    left_op,
                    right_index,
                    right_op,
                    &node_id,
                ));
            }
        }
    }

    conflicts
}

fn classify_semantic_conflicts(base: &File, left: &[PatchOp], right: &[PatchOp]) -> Vec<Conflict> {
    let context = TreeContext::build(base);
    let left_changes = extract_semantic_changes(left, &context);
    let right_changes = extract_semantic_changes(right, &context);
    let mut conflicts = Vec::new();

    for left_change in &left_changes {
        for right_change in &right_changes {
            if let Some(conflict) =
                semantic_conflict_for_pair(left_change, left, right_change, right)
            {
                conflicts.push(conflict);
            }
        }
    }

    conflicts
}

fn semantic_conflict_for_pair(
    left_change: &SemanticChange,
    left_ops: &[PatchOp],
    right_change: &SemanticChange,
    right_ops: &[PatchOp],
) -> Option<Conflict> {
    if left_change.owner != right_change.owner {
        return None;
    }

    let left_op = left_ops.get(left_change.op_index)?;
    let right_op = right_ops.get(right_change.op_index)?;

    match (&left_change.owner, left_change.region, right_change.region) {
        (
            SemanticOwner::Binding { let_id, binding_id },
            SemanticRegion::BindingName,
            SemanticRegion::BindingInitializer,
        )
        | (
            SemanticOwner::Binding { let_id, binding_id },
            SemanticRegion::BindingInitializer,
            SemanticRegion::BindingName,
        ) => Some(binding_rename_vs_initializer_change_conflict(
            left_change.op_index,
            left_op,
            right_change.op_index,
            right_op,
            let_id,
            binding_id,
        )),
        (
            SemanticOwner::Parameter {
                fn_id, param_id, ..
            },
            SemanticRegion::ParameterTypeContract,
            SemanticRegion::ParameterBodyInterpretation,
        )
        | (
            SemanticOwner::Parameter {
                fn_id, param_id, ..
            },
            SemanticRegion::ParameterBodyInterpretation,
            SemanticRegion::ParameterTypeContract,
        ) => Some(parameter_type_vs_body_interpretation_change_conflict(
            left_change.op_index,
            left_op,
            right_change.op_index,
            right_op,
            fn_id,
            param_id,
        )),
        _ => None,
    }
}

fn replay(
    base: &File,
    order: ReplayOrder,
    first: &[PatchOp],
    second: &[PatchOp],
) -> Result<File, ReplayFailure> {
    let mut file = base.clone();
    apply_sequence(&mut file, order, first, first_stage)?;
    apply_sequence(&mut file, order, second, second_stage)?;
    validate_file(&file).map_err(|errors| ReplayFailure {
        order,
        stage: ReplayStage::Validation,
        message: format_validation_errors(&errors),
    })?;
    Ok(file)
}

fn apply_sequence(
    file: &mut File,
    order: ReplayOrder,
    ops: &[PatchOp],
    stage_for_index: impl Fn(usize) -> ReplayStage,
) -> Result<(), ReplayFailure> {
    for (index, op) in ops.iter().cloned().enumerate() {
        apply_op(file, op).map_err(|error| ReplayFailure {
            order,
            stage: stage_for_index(index),
            message: error.to_string(),
        })?;
    }
    Ok(())
}

fn same_node_target(left: &PatchOp, right: &PatchOp) -> Option<String> {
    match (node_target(left), node_target(right)) {
        (Some(left), Some(right)) if left == right => Some(left.to_owned()),
        _ => None,
    }
}

fn same_scalar_path(left: &PatchOp, right: &PatchOp) -> Option<(String, Vec<String>)> {
    match (path_target(left), path_target(right)) {
        (Some((left_node, left_segments)), Some((right_node, right_segments)))
            if left_node == right_node && left_segments == right_segments =>
        {
            Some((left_node.to_owned(), left_segments.to_vec()))
        }
        _ => None,
    }
}

fn same_single_slot_dest(left: &PatchOp, right: &PatchOp) -> Option<SlotRef> {
    match (single_slot_target(left), single_slot_target(right)) {
        (Some(left), Some(right)) if left == right => Some(left.clone()),
        _ => None,
    }
}

fn same_ranked_dest(left: &PatchOp, right: &PatchOp) -> Option<RankedDest> {
    match (ranked_target(left), ranked_target(right)) {
        (Some(left), Some(right)) if left == right => Some(left.clone()),
        _ => None,
    }
}

fn node_target(op: &PatchOp) -> Option<&str> {
    match op {
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => Some(target_id),
        _ => None,
    }
}

fn path_target(op: &PatchOp) -> Option<(&str, &[String])> {
    match op {
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            Some((&path.node_id, &path.segments))
        }
        _ => None,
    }
}

fn single_slot_target(op: &PatchOp) -> Option<&SlotRef> {
    match op {
        PatchOp::Put { slot, .. } => Some(slot),
        PatchOp::Move {
            dest: PatchDest::Slot(slot),
            ..
        } => Some(slot),
        _ => None,
    }
}

fn ranked_target(op: &PatchOp) -> Option<&RankedDest> {
    match op {
        PatchOp::Insert { dest, .. } => Some(dest),
        PatchOp::Move {
            dest: PatchDest::Ranked(dest),
            ..
        } => Some(dest),
        _ => None,
    }
}

fn first_stage(index: usize) -> ReplayStage {
    ReplayStage::LeftOp(index)
}

fn second_stage(index: usize) -> ReplayStage {
    ReplayStage::RightOp(index)
}

fn format_validation_errors(errors: &[draxl_validate::ValidationError]) -> String {
    let mut message = String::from("validation failed:");
    for error in errors {
        message.push_str("\n- ");
        message.push_str(&error.message);
    }
    message
}
