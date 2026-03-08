mod support;

use draxl_parser::parse_file;
use draxl_printer::{canonicalize_file, print_file};
use draxl_validate::validate_file;
use support::{canonical_without_spans, examples, parse_and_validate, read};

#[test]
fn examples_roundtrip_through_the_canonical_printer() {
    for example in examples() {
        let source = read(example.path);
        let parsed = parse_file(&source)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", example.path));
        validate_file(&parsed)
            .unwrap_or_else(|errors| panic!("failed to validate {}: {errors:?}", example.path));

        let formatted = print_file(&parsed);
        let expected = read(&format!("tests/golden/{}.fmt.rs.dx", example.slug));
        assert_eq!(
            formatted, expected,
            "formatter output mismatch for {}",
            example.slug
        );

        let reparsed = parse_file(&formatted)
            .unwrap_or_else(|err| panic!("failed to reparse {}: {err}", example.slug));
        validate_file(&reparsed)
            .unwrap_or_else(|errors| panic!("failed to revalidate {}: {errors:?}", example.slug));

        assert_eq!(
            canonical_without_spans(&parsed),
            canonical_without_spans(&reparsed),
            "semantic round-trip mismatch for {}",
            example.slug
        );
    }
}

#[test]
fn examples_match_json_goldens() {
    for example in examples() {
        let file = parse_and_validate(example.path);
        let json = canonicalize_file(&file).to_json_pretty();
        let expected = read(&format!("tests/golden/{}.json", example.slug));
        assert_eq!(json, expected, "json golden mismatch for {}", example.slug);
    }
}
