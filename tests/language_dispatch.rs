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
