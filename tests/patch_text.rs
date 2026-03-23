mod support;

use draxl::{
    self,
    patch::{PatchDest, PatchNode, PatchOp, PatchPath, PatchValue, RankedDest, SlotOwner, SlotRef},
};
use draxl_printer::print_file;
use draxl_validate::validate_file;
use support::read;

#[test]
fn parse_patch_ops_accepts_multiline_fragments_and_blank_lines() {
    let patch = r#"
replace @f1:
  @f1 fn add_two(@p1 x: @t1 i64) -> @t2 i64 {
    @s1 @e1 (
      @e2 x + @l2 2
    )
  }

set @f1.name = add_two

clear @d1.text
"#;

    let ops = draxl::parse_patch_ops(patch).expect("patch text should parse");
    assert_eq!(ops.len(), 3);
}

#[test]
fn parse_patch_ops_accepts_whole_line_patch_comments() {
    let patch = r#"
// rename the function before touching the doc payload
set @f1.name = add_two

// keep the doc node, but clear its current text
clear @d1.text
"#;

    let ops = draxl::parse_patch_ops(patch).expect("patch text should parse");
    assert_eq!(ops.len(), 2);
}

#[test]
fn parse_patch_ops_rejects_hash_patch_comments() {
    let patch = r#"
# rename the function before touching the doc payload
set @f1.name = add_two
"#;

    let error = draxl::parse_patch_ops(patch).expect_err("hash comments should be rejected");
    assert!(error.to_string().contains("expected identifier"));
}

#[test]
fn resolve_patch_ops_can_target_ids_inserted_earlier_in_the_stream() {
    let source = read("examples/04_ranks.rs.dx");
    let file = draxl::parse_and_validate(&source).expect("example should parse");
    let patch = r#"
insert @f4.body[ah]: @s4 @e4 trace();

replace @e4: @e4 audit()
"#;

    let ops = draxl::resolve_patch_ops(&file, patch).expect("patch text should resolve");
    assert!(matches!(ops[0], PatchOp::Insert { .. }));
    assert!(matches!(ops[1], PatchOp::Replace { .. }));
}

#[test]
fn apply_patch_text_updates_the_tree_end_to_end() {
    let source = r#"
@m1 mod demo {
  @d1->f2 /// About second.
  @f1[a] fn first() {}
  @f2[b] fn second() {}
}
"#;
    let mut file = draxl::parse_and_validate(source).expect("source should parse");
    let patch = r#"
detach @d1
attach @d1 -> @f1
set @f1.name = renamed_first
clear @d1.text
"#;

    draxl::apply_patch_text(&mut file, patch).expect("patch should apply");
    validate_file(&file).expect("patched file should validate");

    let formatted = print_file(&file);
    assert!(formatted.contains("@d1->f1 ///\n"));
    assert!(formatted.contains("fn renamed_first()"));
    assert!(formatted.contains("fn second()"));
}

#[test]
fn apply_patch_text_accepts_comment_lines_after_fragment_ops() {
    let source = read("examples/01_add.rs.dx");
    let mut file = draxl::parse_and_validate(&source).expect("example should parse");
    let patch = r#"
replace @e3: @e3 y_plus_one
// explain why this bundle also clears the stale summary for follow-up review
clear @d1.text
"#;

    draxl::apply_patch_text(&mut file, patch).expect("patch should apply");
    validate_file(&file).expect("patched file should validate");

    let formatted = print_file(&file);
    assert!(formatted.contains("@s2[b] @e3 y_plus_one"));
    assert!(formatted.contains("@d1 ///\n"));
}

#[test]
fn apply_patch_text_reports_schema_errors_with_patch_locations() {
    let source = read("examples/01_add.rs.dx");
    let mut file = draxl::parse_and_validate(&source).expect("example should parse");
    let patch = "set @f1.text = \"nope\"\n";

    let error = draxl::apply_patch_text(&mut file, patch).expect_err("path should be rejected");
    assert!(error.to_string().contains("line 1, column 9"));
    assert!(error.to_string().contains("not settable"));
}

#[test]
fn structured_and_textual_put_share_the_same_slot_validation_message() {
    let source = read("examples/01_add.rs.dx");
    let mut structured_file = draxl::parse_and_validate(&source).expect("example should parse");
    let mut textual_file = draxl::parse_and_validate(&source).expect("example should parse");

    let structured_error = draxl::apply_patch(
        &mut structured_file,
        PatchOp::Put {
            slot: SlotRef {
                owner: SlotOwner::Node("f1".to_owned()),
                slot: "body".to_owned(),
            },
            node: PatchNode::Type(
                draxl::parser::parse_type_fragment("@t9 i128").expect("type fragment should parse"),
            ),
        },
    )
    .expect_err("slot should be rejected")
    .to_string();

    let textual_error = draxl::apply_patch_text(&mut textual_file, "put @f1.body: @t9 i128\n")
        .expect_err("slot should be rejected")
        .to_string();

    assert_eq!(structured_error, strip_location(&textual_error));
}

#[test]
fn structured_and_textual_set_share_the_same_path_validation_message() {
    let source = read("examples/01_add.rs.dx");
    let mut structured_file = draxl::parse_and_validate(&source).expect("example should parse");
    let mut textual_file = draxl::parse_and_validate(&source).expect("example should parse");

    let structured_error = draxl::apply_patch(
        &mut structured_file,
        PatchOp::Set {
            path: PatchPath {
                node_id: "f1".to_owned(),
                segments: vec!["text".to_owned()],
            },
            value: PatchValue::Str("nope".to_owned()),
        },
    )
    .expect_err("path should be rejected")
    .to_string();

    let textual_error = draxl::apply_patch_text(&mut textual_file, "set @f1.text = \"nope\"\n")
        .expect_err("path should be rejected")
        .to_string();

    assert_eq!(structured_error, strip_location(&textual_error));
}

#[test]
fn structured_and_textual_delete_share_the_same_slot_validation_message() {
    let source = read("examples/01_add.rs.dx");
    let mut structured_file = draxl::parse_and_validate(&source).expect("example should parse");
    let mut textual_file = draxl::parse_and_validate(&source).expect("example should parse");

    let structured_error = draxl::apply_patch(
        &mut structured_file,
        PatchOp::Delete {
            target_id: "e3".to_owned(),
        },
    )
    .expect_err("delete should be rejected")
    .to_string();

    let textual_error = draxl::apply_patch_text(&mut textual_file, "delete @e3\n")
        .expect_err("delete should be rejected")
        .to_string();

    assert_eq!(structured_error, strip_location(&textual_error));
}

#[test]
fn structured_and_textual_move_share_the_same_slot_validation_message() {
    let source = read("examples/01_add.rs.dx");
    let mut structured_file = draxl::parse_and_validate(&source).expect("example should parse");
    let mut textual_file = draxl::parse_and_validate(&source).expect("example should parse");

    let structured_error = draxl::apply_patch(
        &mut structured_file,
        PatchOp::Move {
            target_id: "e3".to_owned(),
            dest: PatchDest::Ranked(RankedDest {
                slot: SlotRef {
                    owner: SlotOwner::Node("f1".to_owned()),
                    slot: "body".to_owned(),
                },
                rank: "az".to_owned(),
            }),
        },
    )
    .expect_err("move should be rejected")
    .to_string();

    let textual_error = draxl::apply_patch_text(&mut textual_file, "move @e3 -> @f1.body[az]\n")
        .expect_err("move should be rejected")
        .to_string();

    assert_eq!(structured_error, strip_location(&textual_error));
}

#[test]
fn structured_and_textual_attach_share_the_same_sibling_validation_message() {
    let source = read("examples/01_add.rs.dx");
    let mut structured_file = draxl::parse_and_validate(&source).expect("example should parse");
    let mut textual_file = draxl::parse_and_validate(&source).expect("example should parse");

    let structured_error = draxl::apply_patch(
        &mut structured_file,
        PatchOp::Attach {
            node_id: "d1".to_owned(),
            target_id: "s1".to_owned(),
        },
    )
    .expect_err("attach should be rejected")
    .to_string();

    let textual_error = draxl::apply_patch_text(&mut textual_file, "attach @d1 -> @s1\n")
        .expect_err("attach should be rejected")
        .to_string();

    assert_eq!(structured_error, strip_location(&textual_error));
}

fn strip_location(message: &str) -> String {
    message
        .split(" at line ")
        .next()
        .expect("split should always produce a first segment")
        .to_owned()
}
