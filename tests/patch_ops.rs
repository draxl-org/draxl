mod support;

use draxl_ast::{
    BinaryOp, Block, Expr, ExprBinary, ExprCall, ExprLit, ExprPath, Item, ItemFn, Literal, Meta,
    Param, Path, Stmt, StmtExpr, Type, TypePath,
};
use draxl_patch::{
    apply_op, PatchDest, PatchNode, PatchOp, PatchPath, PatchValue, RankedDest, SlotOwner, SlotRef,
};
use draxl_printer::print_file;
use draxl_rust::lower_file;
use draxl_validate::validate_file;

#[test]
fn insert_statement_into_body_respects_rank_order() {
    let mut file = support::parse_and_validate("examples/04_ranks.rs.dx");
    let stmt = Stmt::Expr(StmtExpr {
        meta: meta("s4"),
        expr: Expr::Call(ExprCall {
            meta: Some(meta("e4")),
            callee: Box::new(Expr::Path(ExprPath {
                meta: None,
                path: path(&["trace"]),
            })),
            args: Vec::new(),
        }),
        has_semi: true,
    });

    apply_op(
        &mut file,
        PatchOp::Insert {
            dest: RankedDest {
                slot: SlotRef {
                    owner: SlotOwner::Node("f4".to_owned()),
                    slot: "body".to_owned(),
                },
                rank: "ah".to_owned(),
            },
            node: PatchNode::Stmt(stmt),
        },
    )
    .expect("insert should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    let fetch = formatted.find("fetch();").expect("fetch call should exist");
    let trace = formatted.find("trace();").expect("trace call should exist");
    let log = formatted.find("log();").expect("log call should exist");
    let validate = formatted
        .find("validate();")
        .expect("validate call should exist");
    assert!(fetch < trace && trace < log && log < validate);
}

#[test]
fn replace_preserves_outer_shell_and_inbound_attachments() {
    let source = r#"
@m1 mod demo {
  @d1->f1 /// Adds one.
  @f1[a] fn add_one(@p1[a] x: @t1 i64) -> @t2 i64 {
    @s1[a] @e1 x
  }
}
"#;

    let mut file = parse_source(source);
    let replacement = Item::Fn(ItemFn {
        meta: meta("f1"),
        name: "add_two".to_owned(),
        params: vec![Param {
            meta: ranked_meta("p9", "a"),
            name: "x".to_owned(),
            ty: Type::Path(TypePath {
                meta: meta("t9"),
                path: path(&["i64"]),
            }),
        }],
        ret_ty: Some(Type::Path(TypePath {
            meta: meta("t10"),
            path: path(&["i64"]),
        })),
        body: Block {
            meta: None,
            stmts: vec![Stmt::Expr(StmtExpr {
                meta: ranked_meta("s9", "a"),
                expr: Expr::Binary(ExprBinary {
                    meta: Some(meta("e9")),
                    lhs: Box::new(Expr::Path(ExprPath {
                        meta: None,
                        path: path(&["x"]),
                    })),
                    op: BinaryOp::Add,
                    rhs: Box::new(Expr::Lit(ExprLit {
                        meta: Some(meta("l2")),
                        value: Literal::Int(2),
                    })),
                }),
                has_semi: false,
            })],
        },
    });

    apply_op(
        &mut file,
        PatchOp::Replace {
            target_id: "f1".to_owned(),
            replacement: PatchNode::Item(replacement),
        },
    )
    .expect("replace should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    assert!(formatted.contains("@d1->f1 /// Adds one."));
    assert!(formatted.contains("@f1[a] fn add_two("));
    let lowered = lower_file(&file);
    assert!(lowered.contains("fn add_two("));
    assert!(lowered.contains("x + 2"));
}

#[test]
fn replace_rejects_competing_outer_rank_metadata() {
    let mut file = support::parse_and_validate("examples/01_add.rs.dx");
    let replacement = Item::Fn(ItemFn {
        meta: ranked_meta("f1", "z"),
        name: "add_two".to_owned(),
        params: Vec::new(),
        ret_ty: None,
        body: Block {
            meta: None,
            stmts: Vec::new(),
        },
    });

    let error = apply_op(
        &mut file,
        PatchOp::Replace {
            target_id: "f1".to_owned(),
            replacement: PatchNode::Item(replacement),
        },
    )
    .expect_err("replace should reject outer rank metadata");

    assert!(error.to_string().contains("must omit outer rank metadata"));
}

#[test]
fn delete_removes_attachment_closure_with_the_target() {
    let source = r#"
@m1 mod demo {
  @d1->f1 /// Delete me too.
  @f1[a] fn first() {}
  @f2[b] fn second() {}
}
"#;

    let mut file = parse_source(source);
    apply_op(
        &mut file,
        PatchOp::Delete {
            target_id: "f1".to_owned(),
        },
    )
    .expect("delete should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    assert!(!formatted.contains("Delete me too."));
    assert!(!formatted.contains("fn first()"));
    assert!(formatted.contains("fn second()"));
}

#[test]
fn move_carries_the_attachment_closure() {
    let source = r#"
@m1 mod demo {
  @d1->f1 /// First helper.
  @f1[a] fn first() {}
  @f2[b] fn second() {}
}
"#;

    let mut file = parse_source(source);
    apply_op(
        &mut file,
        PatchOp::Move {
            target_id: "f1".to_owned(),
            dest: PatchDest::Ranked(RankedDest {
                slot: SlotRef {
                    owner: SlotOwner::Node("m1".to_owned()),
                    slot: "items".to_owned(),
                },
                rank: "c".to_owned(),
            }),
        },
    )
    .expect("move should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    let second = formatted.find("fn second()").expect("second should exist");
    let doc = formatted
        .find("@d1->f1 /// First helper.")
        .expect("doc should move with the target");
    let first = formatted.find("fn first()").expect("first should exist");
    assert!(
        second < doc && doc < first,
        "unexpected canonical order:\n{formatted}"
    );
}

#[test]
fn put_replaces_the_slot_occupant_identity() {
    let mut file = support::parse_and_validate("examples/01_add.rs.dx");

    apply_op(
        &mut file,
        PatchOp::Put {
            slot: SlotRef {
                owner: SlotOwner::Node("f1".to_owned()),
                slot: "ret".to_owned(),
            },
            node: PatchNode::Type(Type::Path(TypePath {
                meta: meta("t99"),
                path: path(&["i128"]),
            })),
        },
    )
    .expect("put should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    assert!(formatted.contains("-> @t99 i128"));
    assert!(!formatted.contains("-> @t2 i64"));
}

#[test]
fn attach_and_detach_update_anchor_relations() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn first() {}
  @d1->f2 /// About second.
  @f2[b] fn second() {}
}
"#;

    let mut file = parse_source(source);
    apply_op(
        &mut file,
        PatchOp::Detach {
            node_id: "d1".to_owned(),
        },
    )
    .expect("detach should succeed");
    apply_op(
        &mut file,
        PatchOp::Attach {
            node_id: "d1".to_owned(),
            target_id: "f1".to_owned(),
        },
    )
    .expect("attach should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    let doc = formatted
        .find("@d1->f1 /// About second.")
        .expect("doc should attach to first");
    let first = formatted.find("fn first()").expect("first should exist");
    let second = formatted.find("fn second()").expect("second should exist");
    assert!(
        doc < first && first < second,
        "unexpected canonical order:\n{formatted}"
    );
}

#[test]
fn set_and_clear_scalar_fields_update_the_tree() {
    let source = r#"
@m1 mod demo {
  @d1 /// Old doc.
  @f1[a] fn old_name() {}
}
"#;

    let mut file = parse_source(source);
    apply_op(
        &mut file,
        PatchOp::Set {
            path: PatchPath {
                node_id: "f1".to_owned(),
                segments: vec!["name".to_owned()],
            },
            value: PatchValue::Ident("new_name".to_owned()),
        },
    )
    .expect("set should succeed");
    apply_op(
        &mut file,
        PatchOp::Clear {
            path: PatchPath {
                node_id: "d1".to_owned(),
                segments: vec!["text".to_owned()],
            },
        },
    )
    .expect("clear should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    assert!(formatted.contains("fn new_name()"));
    assert!(formatted.contains("@d1 ///\n"));
}

fn parse_source(source: &str) -> draxl_ast::File {
    let file = draxl_parser::parse_file(source).expect("source should parse");
    validate_file(&file).expect("source should validate");
    file
}

fn meta(id: &str) -> Meta {
    Meta {
        id: id.to_owned(),
        rank: None,
        anchor: None,
        slot: None,
        span: None,
    }
}

fn ranked_meta(id: &str, rank: &str) -> Meta {
    Meta {
        rank: Some(rank.to_owned()),
        ..meta(id)
    }
}

fn path(segments: &[&str]) -> Path {
    Path {
        segments: segments
            .iter()
            .map(|segment| (*segment).to_owned())
            .collect(),
    }
}
