use draxl_merge::{
    check_conflicts, check_hard_conflicts, ConflictClass, ConflictCode, ConflictOwner,
    ConflictRegion,
};
use draxl_parser::{parse_expr_fragment, parse_type_fragment};
use draxl_patch::{PatchNode, PatchOp, PatchPath, PatchValue, SlotOwner, SlotRef};
use draxl_validate::validate_file;

#[test]
fn reports_binding_rename_vs_initializer_change_as_semantic_conflict() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal = @e1 amount;
    @s2[b] @e2 subtotal
  }
}
"#;
    let file = parse_source(source);
    let left = vec![
        PatchOp::Set {
            path: PatchPath {
                node_id: "p2".to_owned(),
                segments: vec!["name".to_owned()],
            },
            value: PatchValue::Ident("subtotal_cents".to_owned()),
        },
        PatchOp::Replace {
            target_id: "e2".to_owned(),
            replacement: PatchNode::Expr(
                parse_expr_fragment("@e2 subtotal_cents")
                    .expect("reference rename fragment should parse"),
            ),
        },
    ];
    let right = vec![PatchOp::Replace {
        target_id: "e1".to_owned(),
        replacement: PatchNode::Expr(
            parse_expr_fragment("@e1 to_dollars(@e3 amount)")
                .expect("initializer replacement should parse"),
        ),
    }];

    let hard = check_hard_conflicts(&file, &left, &right);
    assert!(
        hard.is_clean(),
        "unexpected hard conflicts: {:?}",
        hard.conflicts
    );

    let report = check_conflicts(&file, &left, &right);
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].class, ConflictClass::Semantic);
    assert_eq!(
        report.conflicts[0].code,
        ConflictCode::BindingRenameVsInitializerChange
    );
    assert_eq!(
        report.conflicts[0].owner,
        Some(ConflictOwner::Binding {
            let_id: "s1".to_owned(),
            binding_id: "p2".to_owned(),
        })
    );
    assert_eq!(
        report.conflicts[0].left_regions,
        vec![ConflictRegion::BindingName]
    );
    assert_eq!(
        report.conflicts[0].right_regions,
        vec![ConflictRegion::BindingInitializer]
    );
    assert_eq!(report.conflicts[0].left.len(), 1);
    assert_eq!(report.conflicts[0].right.len(), 1);
    assert_eq!(
        report.to_json_pretty(),
        r#"{
  "conflicts": [
    {
      "class": "semantic",
      "code": "binding_rename_vs_initializer_change",
      "owner": {
        "kind": "binding",
        "let_id": "s1",
        "binding_id": "p2"
      },
      "left_regions": [
        "binding_name"
      ],
      "right_regions": [
        "binding_initializer"
      ],
      "left": [
        {
          "op_index": 0,
          "op_kind": "set",
          "target": "@p2.name"
        }
      ],
      "right": [
        {
          "op_index": 0,
          "op_kind": "replace",
          "target": "@e1"
        }
      ]
    }
  ]
}
"#
    );
}

#[test]
fn ranked_insert_example_stays_clean_under_full_conflict_check() {
    let source = include_str!("../../../examples/04_ranks.rs.dx");
    let file = parse_source(source);
    let left = vec![PatchOp::Insert {
        dest: draxl_patch::RankedDest {
            slot: draxl_patch::SlotRef {
                owner: draxl_patch::SlotOwner::Node("f4".to_owned()),
                slot: "body".to_owned(),
            },
            rank: "ah".to_owned(),
        },
        node: PatchNode::Stmt(
            draxl_parser::parse_stmt_fragment("@s4 @e4 trace();")
                .expect("statement fragment should parse"),
        ),
    }];
    let right = vec![PatchOp::Replace {
        target_id: "e2".to_owned(),
        replacement: PatchNode::Expr(
            parse_expr_fragment("@e2 audit()").expect("replacement should parse"),
        ),
    }];

    let report = check_conflicts(&file, &left, &right);
    assert!(
        report.is_clean(),
        "unexpected conflicts: {:?}",
        report.conflicts
    );
}

#[test]
fn reports_parameter_type_vs_body_interpretation_change_as_semantic_conflict() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn is_discount_allowed(@p1[a] rate: @t1 Percent) -> @t2 bool {
    @s1[a] @e1 (@e2 rate < @l1 100)
  }
}
"#;
    let file = parse_source(source);
    let left = vec![PatchOp::Put {
        slot: SlotRef {
            owner: SlotOwner::Node("p1".to_owned()),
            slot: "ty".to_owned(),
        },
        node: PatchNode::Type(
            parse_type_fragment("@t3 BasisPoints")
                .expect("parameter type replacement fragment should parse"),
        ),
    }];
    let right = vec![PatchOp::Replace {
        target_id: "e1".to_owned(),
        replacement: PatchNode::Expr(
            parse_expr_fragment("@e1 (@e2 rate < @l1 95)")
                .expect("body replacement fragment should parse"),
        ),
    }];

    let hard = check_hard_conflicts(&file, &left, &right);
    assert!(
        hard.is_clean(),
        "unexpected hard conflicts: {:?}",
        hard.conflicts
    );

    let report = check_conflicts(&file, &left, &right);
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].class, ConflictClass::Semantic);
    assert_eq!(
        report.conflicts[0].code,
        ConflictCode::ParameterTypeVsBodyInterpretationChange
    );
    assert_eq!(
        report.conflicts[0].owner,
        Some(ConflictOwner::Parameter {
            fn_id: "f1".to_owned(),
            param_id: "p1".to_owned(),
            param_name: "rate".to_owned(),
        })
    );
    assert_eq!(
        report.conflicts[0].left_regions,
        vec![ConflictRegion::ParameterTypeContract]
    );
    assert_eq!(
        report.conflicts[0].right_regions,
        vec![ConflictRegion::ParameterBodyInterpretation]
    );
    assert_eq!(report.conflicts[0].left.len(), 1);
    assert_eq!(report.conflicts[0].right.len(), 1);
}

fn parse_source(source: &str) -> draxl_ast::File {
    let file = draxl_parser::parse_file(source).expect("source should parse");
    validate_file(&file).expect("source should validate");
    file
}
