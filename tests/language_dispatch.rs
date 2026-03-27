mod support;

use draxl::{self, LowerLanguage};
use support::read;

#[test]
fn parser_dispatch_matches_default_rust_backend() {
    let source = read("examples/01_add.rs.dx");

    let default = draxl_parser::parse_file(&source).expect("default parse should succeed");
    let explicit = draxl_parser::parse_file_for_language(LowerLanguage::Rust, &source)
        .expect("explicit rust parse should succeed");

    assert_eq!(explicit, default);
}

#[test]
fn printer_dispatch_matches_default_rust_backend() {
    let source = read("examples/02_shapes.rs.dx");
    let file = draxl_parser::parse_file(&source).expect("example should parse");

    let default = draxl_printer::print_file(&file);
    let explicit = draxl_printer::print_file_for_language(LowerLanguage::Rust, &file);

    assert_eq!(explicit, default);
}

#[test]
fn facade_format_dispatch_matches_default_rust_path() {
    let source = read("examples/03_match.rs.dx");

    let default = draxl::format_source(&source).expect("default formatter should succeed");
    let explicit = draxl::format_source_for_language(LowerLanguage::Rust, &source)
        .expect("explicit rust formatter should succeed");

    assert_eq!(explicit, default);
}

#[test]
fn facade_lower_dispatch_matches_default_rust_path() {
    let source = read("examples/04_ranks.rs.dx");

    let default = draxl::lower_rust_source(&source).expect("default lowering should succeed");
    let explicit = draxl::lower_source_for_language(LowerLanguage::Rust, &source)
        .expect("explicit rust lowering should succeed");

    assert_eq!(explicit, default);
}

#[test]
fn structured_patch_dispatch_matches_default_rust_path() {
    let source = read("examples/01_add.rs.dx");
    let patch = "set @f1.name = add_two\n";
    let file = draxl::parse_and_validate(&source).expect("example should parse");
    let ops = draxl::resolve_patch_ops_for_language(LowerLanguage::Rust, &file, patch)
        .expect("patch should resolve");

    let mut default = file.clone();
    let mut explicit = file.clone();

    draxl::apply_patches(&mut default, ops.clone())
        .expect("default patch application should succeed");
    draxl::apply_patches_for_language(LowerLanguage::Rust, &mut explicit, ops)
        .expect("explicit rust patch application should succeed");

    assert_eq!(explicit, default);
}

#[test]
fn merge_dispatch_matches_default_rust_path() {
    let source = read("examples/01_add.rs.dx");
    let file = draxl::parse_and_validate(&source).expect("example should parse");
    let left = draxl::resolve_patch_ops_for_language(
        LowerLanguage::Rust,
        &file,
        "set @f1.name = renamed_first\n",
    )
    .expect("left patch should resolve");
    let right = draxl::resolve_patch_ops_for_language(
        LowerLanguage::Rust,
        &file,
        "set @f1.name = renamed_second\n",
    )
    .expect("right patch should resolve");

    let default = draxl::check_conflicts(&file, &left, &right);
    let explicit = draxl::check_conflicts_for_language(LowerLanguage::Rust, &file, &left, &right);

    assert_eq!(explicit.to_json_pretty(), default.to_json_pretty());
}
