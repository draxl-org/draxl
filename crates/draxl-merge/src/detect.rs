use crate::context::{CallRegion, LetRegion, TreeContext};
use crate::explain::{
    binding_rename_vs_initializer_change_conflict, call_callee_vs_argument_change_conflict,
    non_convergent_replay_conflict, replay_failure_conflict, same_node_write_conflict,
    same_ranked_position_conflict, same_scalar_path_write_conflict,
    same_single_slot_write_conflict,
};
use crate::model::{Conflict, ConflictReport, ReplayFailure, ReplayOrder, ReplayStage};
use draxl_ast::File;
use draxl_patch::{apply_op, PatchDest, PatchOp, PatchValue, RankedDest, SlotOwner, SlotRef};
use draxl_printer::canonicalize_file;
use draxl_validate::validate_file;

/// Checks both hard and semantic conflicts against the same base.
pub fn check_conflicts(base: &File, left: &[PatchOp], right: &[PatchOp]) -> ConflictReport {
    let hard = check_hard_conflicts(base, left, right);
    if hard.has_conflicts() {
        return hard;
    }

    let semantic_conflicts = classify_semantic_conflicts(base, left, right);
    ConflictReport {
        conflicts: semantic_conflicts,
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
    let mut conflicts = Vec::new();

    for (left_index, left_op) in left.iter().enumerate() {
        let left_rename = binding_rename_target(left_op, &context);
        let left_meaning = let_initializer_change_target(left_op, &context);

        for (right_index, right_op) in right.iter().enumerate() {
            let right_rename = binding_rename_target(right_op, &context);
            let right_meaning = let_initializer_change_target(right_op, &context);
            let left_callee = call_callee_change_target(left_op, &context);
            let left_argument = call_argument_change_target(left_op, &context);
            let right_callee = call_callee_change_target(right_op, &context);
            let right_argument = call_argument_change_target(right_op, &context);

            if let (Some(rename), Some(meaning)) = (&left_rename, &right_meaning) {
                if rename.let_id == meaning.let_id {
                    conflicts.push(binding_rename_vs_initializer_change_conflict(
                        left_index,
                        left_op,
                        right_index,
                        right_op,
                        &rename.let_id,
                        &rename.binding_id,
                    ));
                }
            }

            if let (Some(meaning), Some(rename)) = (&left_meaning, &right_rename) {
                if rename.let_id == meaning.let_id {
                    conflicts.push(binding_rename_vs_initializer_change_conflict(
                        right_index,
                        right_op,
                        left_index,
                        left_op,
                        &rename.let_id,
                        &rename.binding_id,
                    ));
                }
            }

            if let (Some(callee), Some(argument)) = (&left_callee, &right_argument) {
                if callee.call_id == argument.call_id {
                    conflicts.push(call_callee_vs_argument_change_conflict(
                        left_index,
                        left_op,
                        right_index,
                        right_op,
                        &callee.call_id,
                    ));
                }
            }

            if let (Some(argument), Some(callee)) = (&left_argument, &right_callee) {
                if callee.call_id == argument.call_id {
                    conflicts.push(call_callee_vs_argument_change_conflict(
                        right_index,
                        right_op,
                        left_index,
                        left_op,
                        &callee.call_id,
                    ));
                }
            }
        }
    }

    conflicts
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct BindingRenameTarget {
    let_id: String,
    binding_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LetInitializerChangeTarget {
    let_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CallRegionChangeTarget {
    call_id: String,
}

fn binding_rename_target(op: &PatchOp, context: &TreeContext) -> Option<BindingRenameTarget> {
    match op {
        PatchOp::Set { path, value } if path.segments.as_slice() == ["name"] => {
            let PatchValue::Ident(_) = value else {
                return None;
            };
            let node = context.node(&path.node_id)?;
            if !node.is_let_binding {
                return None;
            }
            Some(BindingRenameTarget {
                let_id: node.enclosing_let.clone()?,
                binding_id: path.node_id.clone(),
            })
        }
        _ => None,
    }
}

fn let_initializer_change_target(
    op: &PatchOp,
    context: &TreeContext,
) -> Option<LetInitializerChangeTarget> {
    match op {
        PatchOp::Put { slot, .. } => init_slot_target(slot, context),
        PatchOp::Move {
            target_id,
            dest: PatchDest::Slot(slot),
        } => init_slot_target(slot, context).or_else(|| node_in_init_region(target_id, context)),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => node_in_init_region(target_id, context),
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_init_region(&path.node_id, context)
        }
        _ => None,
    }
}

fn init_slot_target(slot: &SlotRef, context: &TreeContext) -> Option<LetInitializerChangeTarget> {
    if slot.slot != "init" {
        return None;
    }

    let SlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    let node = context.node(owner_id)?;
    if !node.is_let_stmt {
        return None;
    }

    Some(LetInitializerChangeTarget {
        let_id: owner_id.clone(),
    })
}

fn node_in_init_region(node_id: &str, context: &TreeContext) -> Option<LetInitializerChangeTarget> {
    let node = context.node(node_id)?;
    if node.let_region != Some(LetRegion::Init) {
        return None;
    }
    Some(LetInitializerChangeTarget {
        let_id: node.enclosing_let.clone()?,
    })
}

fn call_callee_change_target(
    op: &PatchOp,
    context: &TreeContext,
) -> Option<CallRegionChangeTarget> {
    match op {
        PatchOp::Put { slot, .. } => callee_slot_target(slot, context),
        PatchOp::Move {
            target_id,
            dest: PatchDest::Slot(slot),
        } => callee_slot_target(slot, context)
            .or_else(|| node_in_call_region(target_id, context, CallRegion::Callee)),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => {
            node_in_call_region(target_id, context, CallRegion::Callee)
        }
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_call_region(&path.node_id, context, CallRegion::Callee)
        }
        _ => None,
    }
}

fn call_argument_change_target(
    op: &PatchOp,
    context: &TreeContext,
) -> Option<CallRegionChangeTarget> {
    match op {
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => {
            node_in_call_region(target_id, context, CallRegion::Arg)
        }
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_call_region(&path.node_id, context, CallRegion::Arg)
        }
        _ => None,
    }
}

fn callee_slot_target(slot: &SlotRef, context: &TreeContext) -> Option<CallRegionChangeTarget> {
    if slot.slot != "callee" {
        return None;
    }

    let SlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    let node = context.node(owner_id)?;
    if !node.is_call_expr {
        return None;
    }

    Some(CallRegionChangeTarget {
        call_id: owner_id.clone(),
    })
}

fn node_in_call_region(
    node_id: &str,
    context: &TreeContext,
    region: CallRegion,
) -> Option<CallRegionChangeTarget> {
    let node = context.node(node_id)?;
    if node.call_region != Some(region) {
        return None;
    }
    Some(CallRegionChangeTarget {
        call_id: node.enclosing_call.clone()?,
    })
}
