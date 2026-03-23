mod support;

use draxl::{self, Error};
use draxl_parser::parse_expr_fragment;
use draxl_patch::{PatchNode, PatchOp, PatchPath, PatchValue};
use support::read;

#[test]
fn facade_parse_and_validate_accepts_examples() {
    let source = read("examples/01_add.rs.dx");
    let file = draxl::parse_and_validate(&source).expect("example should parse and validate");
    assert_eq!(file.items.len(), 1);
}

#[test]
fn facade_format_matches_the_golden_output() {
    let source = read("examples/02_shapes.rs.dx");
    let formatted = draxl::format_source(&source).expect("example should format");
    let expected = read("tests/golden/02_shapes.fmt.rs.dx");
    assert_eq!(formatted, expected);
}

#[test]
fn facade_dump_json_matches_the_golden_output() {
    let source = read("examples/03_match.rs.dx");
    let json = draxl::dump_json_source(&source).expect("example should dump JSON");
    let expected = read("tests/golden/03_match.json");
    assert_eq!(json, expected);
}

#[test]
fn facade_lowering_matches_the_golden_output() {
    let source = read("examples/04_ranks.rs.dx");
    let lowered = draxl::lower_rust_source(&source).expect("example should lower to Rust");
    let expected = read("tests/golden/04_ranks.lowered.rs");
    assert_eq!(lowered, expected);
}

#[test]
fn facade_surfaces_validation_errors() {
    let source = r#"
@m1 mod demo {
  @f1 fn run() {}
}
"#;

    let error = draxl::parse_and_validate(source).expect_err("missing rank should fail");
    match error {
        Error::Validation(errors) => assert!(
            errors
                .iter()
                .any(|error| error.message.contains("missing `rank`")),
            "expected missing rank validation error, found {errors:?}"
        ),
        Error::Parse(error) => panic!("expected validation error, got parse error: {error}"),
    }
}

#[test]
fn facade_check_conflicts_json_matches_the_agent_shape() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn price(@p1[a] amount: @t1 Cents) -> @t2 Cents {
    @s1[a] let @p2 subtotal = @e1 amount;
    @s2[b] @e2 subtotal
  }
}
"#;
    let file = draxl::parse_and_validate(source).expect("source should parse and validate");
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

    let json = draxl::check_conflicts_json(&file, &left, &right);

    assert_eq!(
        json,
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
