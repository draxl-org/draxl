use draxl_merge::{check_hard_conflicts, ConflictCode};
use draxl_parser::{parse_expr_fragment, parse_stmt_fragment};
use draxl_patch::{PatchNode, PatchOp, RankedDest, SlotOwner, SlotRef};
use draxl_validate::validate_file;
use std::fs;
use std::path::PathBuf;

#[test]
fn reports_non_convergent_replace_operations_as_hard_conflicts() {
    let file = parse_and_validate("examples/04_ranks.rs.dx");
    let left = vec![PatchOp::Replace {
        target_id: "e2".to_owned(),
        replacement: PatchNode::Expr(
            parse_expr_fragment("@e2 audit()").expect("replacement should parse"),
        ),
    }];
    let right = vec![PatchOp::Replace {
        target_id: "e2".to_owned(),
        replacement: PatchNode::Expr(
            parse_expr_fragment("@e2 trace()").expect("replacement should parse"),
        ),
    }];

    let report = check_hard_conflicts(&file, &left, &right);

    assert!(report.has_conflicts());
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].code, ConflictCode::SameNodeWrite);
    assert!(report.conflicts[0].summary.contains("@e2"));
    assert_eq!(report.conflicts[0].left.len(), 1);
    assert_eq!(report.conflicts[0].right.len(), 1);
}

#[test]
fn reports_duplicate_rank_inserts_as_hard_conflicts() {
    let file = parse_and_validate("examples/04_ranks.rs.dx");
    let left = vec![PatchOp::Insert {
        dest: RankedDest {
            slot: SlotRef {
                owner: SlotOwner::Node("f4".to_owned()),
                slot: "body".to_owned(),
            },
            rank: "ah".to_owned(),
        },
        node: PatchNode::Stmt(
            parse_stmt_fragment("@s4 @e4 trace();").expect("statement fragment should parse"),
        ),
    }];
    let right = vec![PatchOp::Insert {
        dest: RankedDest {
            slot: SlotRef {
                owner: SlotOwner::Node("f4".to_owned()),
                slot: "body".to_owned(),
            },
            rank: "ah".to_owned(),
        },
        node: PatchNode::Stmt(
            parse_stmt_fragment("@s5 @e5 audit();").expect("statement fragment should parse"),
        ),
    }];

    let report = check_hard_conflicts(&file, &left, &right);

    assert!(report.has_conflicts());
    assert_eq!(report.conflicts.len(), 1);
    assert_eq!(report.conflicts[0].code, ConflictCode::SameRankedPosition);
    assert!(report.conflicts[0].summary.contains("@f4.body[ah]"));
    assert_eq!(report.conflicts[0].left.len(), 1);
    assert_eq!(report.conflicts[0].right.len(), 1);
}

#[test]
fn different_rank_inserts_do_not_report_hard_conflicts() {
    let file = parse_and_validate("examples/04_ranks.rs.dx");
    let left = vec![PatchOp::Insert {
        dest: RankedDest {
            slot: SlotRef {
                owner: SlotOwner::Node("f4".to_owned()),
                slot: "body".to_owned(),
            },
            rank: "ah".to_owned(),
        },
        node: PatchNode::Stmt(
            parse_stmt_fragment("@s4 @e4 trace();").expect("statement fragment should parse"),
        ),
    }];
    let right = vec![PatchOp::Insert {
        dest: RankedDest {
            slot: SlotRef {
                owner: SlotOwner::Node("f4".to_owned()),
                slot: "body".to_owned(),
            },
            rank: "ai".to_owned(),
        },
        node: PatchNode::Stmt(
            parse_stmt_fragment("@s5 @e5 audit();").expect("statement fragment should parse"),
        ),
    }];

    let report = check_hard_conflicts(&file, &left, &right);

    assert!(
        report.is_clean(),
        "unexpected conflicts: {:?}",
        report.conflicts
    );
}

fn parse_and_validate(relative: &str) -> draxl_ast::File {
    let source = read(relative);
    let file = draxl_parser::parse_file(&source).expect("example should parse");
    validate_file(&file).expect("example should validate");
    file
}

fn read(relative: &str) -> String {
    let path = repo_path(relative);
    fs::read_to_string(&path)
        .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
}

fn repo_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}
