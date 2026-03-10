mod support;

use draxl::{self, patch::PatchOp};
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
fn apply_patch_text_reports_schema_errors_with_patch_locations() {
    let source = read("examples/01_add.rs.dx");
    let mut file = draxl::parse_and_validate(&source).expect("example should parse");
    let patch = "set @f1.text = \"nope\"\n";

    let error = draxl::apply_patch_text(&mut file, patch).expect_err("path should be rejected");
    assert!(error.to_string().contains("line 1, column 9"));
    assert!(error.to_string().contains("not settable"));
}
