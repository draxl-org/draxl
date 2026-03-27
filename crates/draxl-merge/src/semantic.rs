use crate::context::TreeContext;
use crate::model::{ConflictOwner, ConflictRegion};
use draxl_patch::{PatchDest, PatchNode, PatchOp, PatchValue, SlotOwner, SlotRef};
use draxl_rust::{
    extract_semantic_changes as extract_rust_semantic_changes, SemanticOp, SemanticPatchNode,
    SemanticSlotOwner, SemanticSlotRef,
};
pub(crate) use draxl_rust::{SemanticChange, SemanticOwner, SemanticRegion};

impl From<&SemanticOwner> for ConflictOwner {
    fn from(owner: &SemanticOwner) -> Self {
        match owner {
            SemanticOwner::Binding { let_id, binding_id } => Self::Binding {
                let_id: let_id.clone(),
                binding_id: binding_id.clone(),
            },
            SemanticOwner::Parameter {
                fn_id,
                param_id,
                param_name,
            } => Self::Parameter {
                fn_id: fn_id.clone(),
                param_id: param_id.clone(),
                param_name: param_name.clone(),
            },
        }
    }
}

impl From<SemanticRegion> for ConflictRegion {
    fn from(region: SemanticRegion) -> Self {
        match region {
            SemanticRegion::BindingName => Self::BindingName,
            SemanticRegion::BindingInitializer => Self::BindingInitializer,
            SemanticRegion::ParameterTypeContract => Self::ParameterTypeContract,
            SemanticRegion::ParameterBodyInterpretation => Self::ParameterBodyInterpretation,
        }
    }
}

pub(crate) fn extract_semantic_changes(
    ops: &[PatchOp],
    context: &TreeContext,
) -> Vec<SemanticChange> {
    let semantic_ops = ops.iter().map(translate_op).collect::<Vec<_>>();
    extract_rust_semantic_changes(&semantic_ops, context)
}

fn translate_op(op: &PatchOp) -> SemanticOp {
    match op {
        PatchOp::Insert { .. } | PatchOp::Attach { .. } | PatchOp::Detach { .. } => {
            SemanticOp::Other
        }
        PatchOp::Put { slot, node } => SemanticOp::Put {
            slot: translate_slot_ref(slot),
            node: translate_patch_node(node),
        },
        PatchOp::Replace {
            target_id,
            replacement,
        } => SemanticOp::Replace {
            target_id: target_id.clone(),
            replacement: translate_patch_node(replacement),
        },
        PatchOp::Delete { target_id } => SemanticOp::Delete {
            target_id: target_id.clone(),
        },
        PatchOp::Move { target_id, dest } => SemanticOp::Move {
            target_id: target_id.clone(),
            dest_slot: match dest {
                PatchDest::Ranked(_) => None,
                PatchDest::Slot(slot) => Some(translate_slot_ref(slot)),
            },
        },
        PatchOp::Set { path, value } => SemanticOp::Set {
            node_id: path.node_id.clone(),
            path: path.segments.clone(),
            ident_value: matches!(value, PatchValue::Ident(_)),
        },
        PatchOp::Clear { path } => SemanticOp::Clear {
            node_id: path.node_id.clone(),
        },
    }
}

fn translate_slot_ref(slot: &SlotRef) -> SemanticSlotRef {
    SemanticSlotRef {
        owner: match &slot.owner {
            SlotOwner::File => SemanticSlotOwner::File,
            SlotOwner::Node(owner_id) => SemanticSlotOwner::Node(owner_id.clone()),
        },
        slot: slot.slot.clone(),
    }
}

fn translate_patch_node(node: &PatchNode) -> Option<SemanticPatchNode> {
    match node {
        PatchNode::Expr(expr) => Some(SemanticPatchNode::Expr(expr.clone())),
        PatchNode::Stmt(stmt) => Some(SemanticPatchNode::Stmt(stmt.clone())),
        _ => None,
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
