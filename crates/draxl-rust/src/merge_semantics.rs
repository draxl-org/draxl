use crate::merge_context::{LetRegion, TreeContext};
use draxl_ast::{Expr, Stmt};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticOwner {
    Binding {
        let_id: String,
        binding_id: String,
    },
    Parameter {
        fn_id: String,
        param_id: String,
        param_name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticRegion {
    BindingName,
    BindingInitializer,
    ParameterTypeContract,
    ParameterBodyInterpretation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticChange {
    pub owner: SemanticOwner,
    pub region: SemanticRegion,
    pub op_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticPatchNode {
    Expr(Expr),
    Stmt(Stmt),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticSlotOwner {
    File,
    Node(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticSlotRef {
    pub owner: SemanticSlotOwner,
    pub slot: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemanticOp {
    Set {
        node_id: String,
        path: Vec<String>,
        ident_value: bool,
    },
    Clear {
        node_id: String,
    },
    Put {
        slot: SemanticSlotRef,
        node: Option<SemanticPatchNode>,
    },
    Replace {
        target_id: String,
        replacement: Option<SemanticPatchNode>,
    },
    Delete {
        target_id: String,
    },
    Move {
        target_id: String,
        dest_slot: Option<SemanticSlotRef>,
    },
    Other,
}

pub fn extract_semantic_changes(ops: &[SemanticOp], context: &TreeContext) -> Vec<SemanticChange> {
    let mut changes = Vec::new();

    for (op_index, op) in ops.iter().enumerate() {
        changes.extend(semantic_changes_for_op(op_index, op, context));
    }

    changes
}

fn semantic_changes_for_op(
    op_index: usize,
    op: &SemanticOp,
    context: &TreeContext,
) -> Vec<SemanticChange> {
    let mut changes = Vec::new();

    if let Some(owner) = binding_name_owner(op, context) {
        changes.push(SemanticChange {
            owner,
            region: SemanticRegion::BindingName,
            op_index,
        });
    }

    if let Some(owner) = binding_initializer_owner(op, context) {
        changes.push(SemanticChange {
            owner,
            region: SemanticRegion::BindingInitializer,
            op_index,
        });
    }

    if let Some(owner) = parameter_type_contract_owner(op, context) {
        changes.push(SemanticChange {
            owner,
            region: SemanticRegion::ParameterTypeContract,
            op_index,
        });
    }

    changes.extend(parameter_body_interpretation_changes(op_index, op, context));

    changes
}

fn binding_name_owner(op: &SemanticOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        SemanticOp::Set {
            node_id,
            path,
            ident_value,
        } if path.as_slice() == ["name"] => {
            if !ident_value {
                return None;
            }
            let node = context.node(node_id)?;
            if !node.is_let_binding {
                return None;
            }
            binding_owner_for_let(node.enclosing_let.as_deref()?, context)
        }
        _ => None,
    }
}

fn binding_initializer_owner(op: &SemanticOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        SemanticOp::Put { slot, .. } => init_slot_binding_owner(slot, context),
        SemanticOp::Move {
            target_id,
            dest_slot: Some(slot),
        } => init_slot_binding_owner(slot, context)
            .or_else(|| node_in_init_region_binding_owner(target_id, context)),
        SemanticOp::Replace { target_id, .. }
        | SemanticOp::Delete { target_id }
        | SemanticOp::Move { target_id, .. } => {
            node_in_init_region_binding_owner(target_id, context)
        }
        SemanticOp::Set { node_id, .. } | SemanticOp::Clear { node_id } => {
            node_in_init_region_binding_owner(node_id, context)
        }
        _ => None,
    }
}

fn init_slot_binding_owner(slot: &SemanticSlotRef, context: &TreeContext) -> Option<SemanticOwner> {
    if slot.slot != "init" {
        return None;
    }

    let SemanticSlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    let node = context.node(owner_id)?;
    if !node.is_let_stmt {
        return None;
    }

    binding_owner_for_let(owner_id, context)
}

fn node_in_init_region_binding_owner(
    node_id: &str,
    context: &TreeContext,
) -> Option<SemanticOwner> {
    let node = context.node(node_id)?;
    if node.let_region != Some(LetRegion::Init) {
        return None;
    }

    binding_owner_for_let(node.enclosing_let.as_deref()?, context)
}

fn binding_owner_for_let(let_id: &str, context: &TreeContext) -> Option<SemanticOwner> {
    Some(SemanticOwner::Binding {
        let_id: let_id.to_owned(),
        binding_id: context.binding_id_for_let(let_id)?.to_owned(),
    })
}

fn parameter_type_contract_owner(op: &SemanticOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        SemanticOp::Put { slot, .. } => param_type_slot_owner(slot, context),
        SemanticOp::Move {
            target_id,
            dest_slot: Some(slot),
        } => param_type_slot_owner(slot, context)
            .or_else(|| node_in_param_type_region_owner(target_id, context)),
        SemanticOp::Replace { target_id, .. }
        | SemanticOp::Delete { target_id }
        | SemanticOp::Move { target_id, .. } => node_in_param_type_region_owner(target_id, context),
        SemanticOp::Set { node_id, .. } | SemanticOp::Clear { node_id } => {
            node_in_param_type_region_owner(node_id, context)
        }
        _ => None,
    }
}

fn param_type_slot_owner(slot: &SemanticSlotRef, context: &TreeContext) -> Option<SemanticOwner> {
    if slot.slot != "ty" {
        return None;
    }

    let SemanticSlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    parameter_owner_from_param_id(owner_id, context)
}

fn node_in_param_type_region_owner(node_id: &str, context: &TreeContext) -> Option<SemanticOwner> {
    let node = context.node(node_id)?;
    if !node.param_type_region {
        return None;
    }

    parameter_owner_from_param_id(node.enclosing_param.as_deref()?, context)
}

fn parameter_owner_from_param_id(param_id: &str, context: &TreeContext) -> Option<SemanticOwner> {
    let node = context.node(param_id)?;
    Some(SemanticOwner::Parameter {
        fn_id: node.enclosing_fn.clone()?,
        param_id: param_id.to_owned(),
        param_name: node.param_name.clone()?,
    })
}

fn parameter_body_interpretation_changes(
    op_index: usize,
    op: &SemanticOp,
    context: &TreeContext,
) -> Vec<SemanticChange> {
    let (fn_id, node) = match op {
        SemanticOp::Replace {
            target_id,
            replacement,
        } => {
            let Some(fn_id) = function_body_owner_id_for_node(target_id, context) else {
                return Vec::new();
            };
            (fn_id, replacement)
        }
        SemanticOp::Put { slot, node } => {
            let Some(fn_id) = function_body_owner_id_for_slot(slot, context) else {
                return Vec::new();
            };
            (fn_id, node)
        }
        _ => return Vec::new(),
    };

    context
        .params_in_fn(&fn_id)
        .iter()
        .filter(|param| patch_node_mentions_name(node.as_ref(), &param.name))
        .map(|param| SemanticChange {
            owner: SemanticOwner::Parameter {
                fn_id: fn_id.clone(),
                param_id: param.id.clone(),
                param_name: param.name.clone(),
            },
            region: SemanticRegion::ParameterBodyInterpretation,
            op_index,
        })
        .collect()
}

fn function_body_owner_id_for_node(node_id: &str, context: &TreeContext) -> Option<String> {
    let node = context.node(node_id)?;
    if !node.in_fn_body {
        return None;
    }

    node.enclosing_fn.clone()
}

fn function_body_owner_id_for_slot(
    slot: &SemanticSlotRef,
    context: &TreeContext,
) -> Option<String> {
    let SemanticSlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    function_body_owner_id_for_node(owner_id, context)
}

fn patch_node_mentions_name(node: Option<&SemanticPatchNode>, name: &str) -> bool {
    match node {
        Some(SemanticPatchNode::Expr(expr)) => expr_mentions_name(expr, name),
        Some(SemanticPatchNode::Stmt(stmt)) => stmt_mentions_name(stmt, name),
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
