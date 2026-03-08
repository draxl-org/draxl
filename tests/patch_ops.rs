mod support;

use draxl_ast::{
    BinaryOp, Block, Expr, ExprBinary, ExprCall, ExprLit, ExprPath, Item, ItemFn, Literal, Meta,
    Param, Path, Stmt, StmtExpr, Type, TypePath,
};
use draxl_lower_rust::lower_file;
use draxl_patch::{apply_op, PatchNode, PatchOp, PatchParent};
use draxl_printer::print_file;
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
            parent: PatchParent::Node {
                id: "f1".to_owned(),
            },
            slot: "body".to_owned(),
            rank: "ah".to_owned(),
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
fn replace_item_by_id_updates_the_function_body() {
    let mut file = support::parse_and_validate("examples/01_add.rs.dx");
    let replacement = Item::Fn(ItemFn {
        meta: ranked_meta("f1", "a"),
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
    let lowered = lower_file(&file);
    assert!(lowered.contains("fn add_two("));
    assert!(lowered.contains("x + 2"));
}

#[test]
fn delete_statement_by_id_removes_the_node() {
    let mut file = support::parse_and_validate("examples/04_ranks.rs.dx");
    apply_op(
        &mut file,
        PatchOp::Delete {
            target_id: "s2".to_owned(),
        },
    )
    .expect("delete should succeed");

    validate_file(&file).expect("patched file should validate");
    let formatted = print_file(&file);
    assert!(!formatted.contains("log();"));
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
