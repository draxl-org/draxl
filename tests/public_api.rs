mod support;

use draxl::{self, Error};
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
