use crate::context::{LetRegion, TreeContext};
use draxl_ast::{Expr, Stmt};
use draxl_patch::{PatchDest, PatchNode, PatchOp, PatchValue, SlotOwner, SlotRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SemanticOwner {
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
pub(crate) enum SemanticRegion {
    BindingName,
    BindingInitializer,
    ParameterTypeContract,
    ParameterBodyInterpretation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SemanticChange {
    pub owner: SemanticOwner,
    pub region: SemanticRegion,
    pub op_index: usize,
}

pub(crate) fn extract_semantic_changes(
    ops: &[PatchOp],
    context: &TreeContext,
) -> Vec<SemanticChange> {
    let mut changes = Vec::new();

    for (op_index, op) in ops.iter().enumerate() {
        changes.extend(semantic_changes_for_op(op_index, op, context));
    }

    changes
}

fn semantic_changes_for_op(
    op_index: usize,
    op: &PatchOp,
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

fn binding_name_owner(op: &PatchOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        PatchOp::Set { path, value } if path.segments.as_slice() == ["name"] => {
            let PatchValue::Ident(_) = value else {
                return None;
            };
            let node = context.node(&path.node_id)?;
            if !node.is_let_binding {
                return None;
            }
            binding_owner_for_let(node.enclosing_let.as_deref()?, context)
        }
        _ => None,
    }
}

fn binding_initializer_owner(op: &PatchOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        PatchOp::Put { slot, .. } => init_slot_binding_owner(slot, context),
        PatchOp::Move {
            target_id,
            dest: PatchDest::Slot(slot),
        } => init_slot_binding_owner(slot, context)
            .or_else(|| node_in_init_region_binding_owner(target_id, context)),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => node_in_init_region_binding_owner(target_id, context),
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_init_region_binding_owner(&path.node_id, context)
        }
        _ => None,
    }
}

fn init_slot_binding_owner(slot: &SlotRef, context: &TreeContext) -> Option<SemanticOwner> {
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

fn parameter_type_contract_owner(op: &PatchOp, context: &TreeContext) -> Option<SemanticOwner> {
    match op {
        PatchOp::Put { slot, .. } => param_type_slot_owner(slot, context),
        PatchOp::Move {
            target_id,
            dest: PatchDest::Slot(slot),
        } => param_type_slot_owner(slot, context)
            .or_else(|| node_in_param_type_region_owner(target_id, context)),
        PatchOp::Replace { target_id, .. }
        | PatchOp::Delete { target_id }
        | PatchOp::Move { target_id, .. } => node_in_param_type_region_owner(target_id, context),
        PatchOp::Set { path, .. } | PatchOp::Clear { path } => {
            node_in_param_type_region_owner(&path.node_id, context)
        }
        _ => None,
    }
}

fn param_type_slot_owner(slot: &SlotRef, context: &TreeContext) -> Option<SemanticOwner> {
    if slot.slot != "ty" {
        return None;
    }

    let SlotOwner::Node(owner_id) = &slot.owner else {
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
    op: &PatchOp,
    context: &TreeContext,
) -> Vec<SemanticChange> {
    let (fn_id, node) = match op {
        PatchOp::Replace {
            target_id,
            replacement,
        } => {
            let Some(fn_id) = function_body_owner_id_for_node(target_id, context) else {
                return Vec::new();
            };
            (fn_id, replacement)
        }
        PatchOp::Put { slot, node } => {
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
        .filter(|param| patch_node_mentions_name(node, &param.name))
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

fn function_body_owner_id_for_slot(slot: &SlotRef, context: &TreeContext) -> Option<String> {
    let SlotOwner::Node(owner_id) = &slot.owner else {
        return None;
    };

    function_body_owner_id_for_node(owner_id, context)
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

#[cfg(test)]
mod tests {
    use super::{extract_semantic_changes, SemanticChange, SemanticOwner, SemanticRegion};
    use crate::context::TreeContext;
    use draxl_patch::{PatchNode, PatchOp, PatchPath, PatchValue, SlotOwner, SlotRef};
    use draxl_validate::validate_file;

    #[test]
    fn extracts_binding_name_changes() {
        let file = parse_source(
            r#"
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal = @e1 amount;
    @s2[b] @e2 subtotal
  }
}
"#,
        );
        let ops = vec![PatchOp::Set {
            path: PatchPath {
                node_id: "p2".to_owned(),
                segments: vec!["name".to_owned()],
            },
            value: PatchValue::Ident("subtotal_cents".to_owned()),
        }];

        let changes = extract_semantic_changes(&ops, &TreeContext::build(&file));

        assert_eq!(
            changes,
            vec![SemanticChange {
                owner: SemanticOwner::Binding {
                    let_id: "s1".to_owned(),
                    binding_id: "p2".to_owned(),
                },
                region: SemanticRegion::BindingName,
                op_index: 0,
            }]
        );
    }

    #[test]
    fn extracts_binding_initializer_changes() {
        let file = parse_source(
            r#"
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal = @e1 amount;
    @s2[b] @e2 subtotal
  }
}
"#,
        );
        let ops = vec![PatchOp::Replace {
            target_id: "e1".to_owned(),
            replacement: PatchNode::Expr(
                draxl_parser::parse_expr_fragment("@e1 zero_dollars()")
                    .expect("initializer replacement should parse"),
            ),
        }];

        let changes = extract_semantic_changes(&ops, &TreeContext::build(&file));

        assert_eq!(
            changes,
            vec![SemanticChange {
                owner: SemanticOwner::Binding {
                    let_id: "s1".to_owned(),
                    binding_id: "p2".to_owned(),
                },
                region: SemanticRegion::BindingInitializer,
                op_index: 0,
            }]
        );
    }

    #[test]
    fn extracts_parameter_type_contract_changes() {
        let file = parse_source(
            r#"
@m1 mod demo {
  @f1[a] fn is_discount_allowed(@p1[a] rate: @t1 Percent) -> @t2 bool {
    @s1[a] @e1 (@e2 rate < @l1 100)
  }
}
"#,
        );
        let ops = vec![PatchOp::Put {
            slot: SlotRef {
                owner: SlotOwner::Node("p1".to_owned()),
                slot: "ty".to_owned(),
            },
            node: PatchNode::Type(
                draxl_parser::parse_type_fragment("@t3 BasisPoints")
                    .expect("parameter type replacement fragment should parse"),
            ),
        }];

        let changes = extract_semantic_changes(&ops, &TreeContext::build(&file));

        assert_eq!(
            changes,
            vec![SemanticChange {
                owner: SemanticOwner::Parameter {
                    fn_id: "f1".to_owned(),
                    param_id: "p1".to_owned(),
                    param_name: "rate".to_owned(),
                },
                region: SemanticRegion::ParameterTypeContract,
                op_index: 0,
            }]
        );
    }

    #[test]
    fn extracts_parameter_body_interpretation_changes() {
        let file = parse_source(
            r#"
@m1 mod demo {
  @f1[a] fn is_discount_allowed(@p1[a] rate: @t1 Percent, @p2[b] other: @t3 Percent) -> @t2 bool {
    @s1[a] @e1 (@e2 rate < @l1 100)
  }
}
"#,
        );
        let ops = vec![PatchOp::Replace {
            target_id: "e1".to_owned(),
            replacement: PatchNode::Expr(
                draxl_parser::parse_expr_fragment("@e1 (@e2 rate < @l1 95)")
                    .expect("body replacement fragment should parse"),
            ),
        }];

        let changes = extract_semantic_changes(&ops, &TreeContext::build(&file));

        assert_eq!(
            changes,
            vec![SemanticChange {
                owner: SemanticOwner::Parameter {
                    fn_id: "f1".to_owned(),
                    param_id: "p1".to_owned(),
                    param_name: "rate".to_owned(),
                },
                region: SemanticRegion::ParameterBodyInterpretation,
                op_index: 0,
            }]
        );
    }

    fn parse_source(source: &str) -> draxl_ast::File {
        let file = draxl_parser::parse_file(source).expect("source should parse");
        validate_file(&file).expect("source should validate");
        file
    }
}
