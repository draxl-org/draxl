use crate::context::{LetRegion, TreeContext};
use crate::explain::{
    binding_rename_vs_initializer_change_conflict, non_convergent_replay_conflict,
    parameter_type_vs_body_interpretation_change_conflict, replay_failure_conflict,
    same_node_write_conflict, same_ranked_position_conflict, same_scalar_path_write_conflict,
    same_single_slot_write_conflict,
};
use crate::model::{Conflict, ConflictReport, ReplayFailure, ReplayOrder, ReplayStage};
use draxl_ast::{Expr, File, Stmt};
use draxl_patch::{
    apply_op, PatchDest, PatchNode, PatchOp, PatchValue, RankedDest, SlotOwner, SlotRef,
};
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
        let left_param_contract = parameter_type_change_target(left_op, &context);

        for (right_index, right_op) in right.iter().enumerate() {
            let right_rename = binding_rename_target(right_op, &context);
            let right_meaning = let_initializer_change_target(right_op, &context);
            let right_param_contract = parameter_type_change_target(right_op, &context);

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

            if let Some(contract) = &left_param_contract {
                if parameter_body_interpretation_change_for_param(right_op, &context, contract) {
                    conflicts.push(parameter_type_vs_body_interpretation_change_conflict(
                        left_index,
                        left_op,
                        right_index,
                        right_op,
                        &contract.fn_id,
                        &contract.param_id,
                    ));
                }
            }

            if let Some(contract) = &right_param_contract {
                if parameter_body_interpretation_change_for_param(left_op, &context, contract) {
                    conflicts.push(parameter_type_vs_body_interpretation_change_conflict(
                        right_index,
                        right_op,
                        left_index,
                        left_op,
                        &contract.fn_id,
                        &contract.param_id,
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
struct ParameterTypeChangeTarget {
    fn_id: String,
    param_id: String,
    param_name: String,
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

fn parameter_type_change_target(
    op: &PatchOp,
    context: &TreeContext,
) -> Option<ParameterTypeChangeTarget> {
    match op {
        PatchOp::Put { slot, .. } => param_type_slot_target(slot, context),
        PatchOp::Move {
            target_id,
            dest: PatchDest::Slot(slot),
        } => param_type_slot_target(slot, context)
            .or_else(|| node_in_param_type_region(target_id, context)),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => node_in_param_type_region(target_id, context),
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_param_type_region(&path.node_id, context)
        }
        _ => None,
    }
}

fn param_type_slot_target(
    slot: &SlotRef,
    context: &TreeContext,
) -> Option<ParameterTypeChangeTarget> {
    if slot.slot != "ty" {
        return None;
    }

    let SlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    let node = context.node(owner_id)?;
    let fn_id = node.enclosing_fn.clone()?;
    let param_name = node.param_name.clone()?;

    Some(ParameterTypeChangeTarget {
        fn_id,
        param_id: owner_id.clone(),
        param_name,
    })
}

fn node_in_param_type_region(
    node_id: &str,
    context: &TreeContext,
) -> Option<ParameterTypeChangeTarget> {
    let node = context.node(node_id)?;
    if !node.param_type_region {
        return None;
    }

    let param_id = node.enclosing_param.clone()?;
    let param = context.node(&param_id)?;

    Some(ParameterTypeChangeTarget {
        fn_id: node.enclosing_fn.clone()?,
        param_id,
        param_name: param.param_name.clone()?,
    })
}

fn parameter_body_interpretation_change_for_param(
    op: &PatchOp,
    context: &TreeContext,
    contract: &ParameterTypeChangeTarget,
) -> bool {
    match op {
        PatchOp::Replace {
            target_id,
            replacement,
        } => {
            node_in_fn_body(target_id, context, &contract.fn_id)
                && patch_node_mentions_name(replacement, &contract.param_name)
        }
        PatchOp::Put { slot, node } => {
            slot_in_fn_body(slot, context, &contract.fn_id)
                && patch_node_mentions_name(node, &contract.param_name)
        }
        _ => false,
    }
}

fn node_in_fn_body(node_id: &str, context: &TreeContext, fn_id: &str) -> bool {
    let Some(node) = context.node(node_id) else {
        return false;
    };
    node.in_fn_body && node.enclosing_fn.as_deref() == Some(fn_id)
}

fn slot_in_fn_body(slot: &SlotRef, context: &TreeContext, fn_id: &str) -> bool {
    let SlotOwner::Node(owner_id) = &slot.owner else {
        return false;
    };
    node_in_fn_body(owner_id, context, fn_id)
}

fn patch_node_mentions_name(node: &PatchNode, name: &str) -> bool {
    match node {
        PatchNode::Expr(expr) => expr_mentions_name(expr, name),
        PatchNode::Stmt(stmt) => stmt_mentions_name(stmt, name),
        _ => false,
    }
}

fn stmt_mentions_name(stmt: &Stmt, name: &str) -> bool {
    match stmt {
        Stmt::Let(node) => expr_mentions_name(&node.value, name),
        Stmt::Expr(node) => expr_mentions_name(&node.expr, name),
        Stmt::Item(_) | Stmt::Doc(_) | Stmt::Comment(_) => false,
    }
}

fn expr_mentions_name(expr: &Expr, name: &str) -> bool {
    match expr {
        Expr::Path(node) => node.path.segments.len() == 1 && node.path.segments[0] == name,
        Expr::Lit(_) => false,
        Expr::Group(node) => expr_mentions_name(&node.expr, name),
        Expr::Binary(node) => {
            expr_mentions_name(&node.lhs, name) || expr_mentions_name(&node.rhs, name)
        }
        Expr::Unary(node) => expr_mentions_name(&node.expr, name),
        Expr::Call(node) => {
            expr_mentions_name(&node.callee, name)
                || node.args.iter().any(|arg| expr_mentions_name(arg, name))
        }
        Expr::Match(node) => {
            expr_mentions_name(&node.scrutinee, name)
                || node
                    .arms
                    .iter()
                    .any(|arm| expr_mentions_name(&arm.body, name))
                || node
                    .arms
                    .iter()
                    .filter_map(|arm| arm.guard.as_ref())
                    .any(|guard| expr_mentions_name(guard, name))
        }
        Expr::Block(block) => block
            .stmts
            .iter()
            .any(|stmt| stmt_mentions_name(stmt, name)),
    }
}
