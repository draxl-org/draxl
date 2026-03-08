mod support;

use draxl_parser::parse_file;
use draxl_printer::print_file;
use draxl_validate::validate_file;
use support::examples;

#[test]
fn examples_validate_cleanly() {
    for example in examples() {
        let source = std::fs::read_to_string(support::repo_path(example.path))
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", example.path));
        let file = parse_file(&source)
            .unwrap_or_else(|err| panic!("failed to parse {}: {err}", example.path));
        validate_file(&file)
            .unwrap_or_else(|errors| panic!("failed to validate {}: {errors:?}", example.path));
    }
}

#[test]
fn malformed_metadata_prefix_is_rejected() {
    let source = r#"
@
mod demo {}
"#;

    let error = parse_file(source).expect_err("source should fail during parsing");
    assert!(
        error
            .to_string()
            .contains("expected metadata identifier after `@` in Draxl source"),
        "unexpected parse error: {error}"
    );
}

#[test]
fn duplicate_ids_are_reported() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn first() {}
  @f1[b] fn second() {}
}
"#;

    let file = parse_file(source).expect("duplicate ids parse successfully before validation");
    let errors = validate_file(&file).expect_err("duplicate ids should fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("duplicate id `f1`")),
        "expected duplicate id error, found {errors:?}"
    );
}

#[test]
fn missing_rank_in_an_ordered_slot_is_reported() {
    let source = r#"
@m1 mod demo {
  @f1 fn run() {}
}
"#;

    let file = parse_file(source).expect("missing rank parses before validation");
    let errors = validate_file(&file).expect_err("missing rank should fail validation");
    assert!(
        errors
            .iter()
            .any(|error| error.message.contains("missing `rank`")),
        "expected missing rank error, found {errors:?}"
    );
}

#[test]
fn bad_anchor_is_reported() {
    let source = r#"
@m1 mod demo {
  @d1->missing /// Missing anchor target.
  @f1[a] fn run() {}
}
"#;

    let file = parse_file(source).expect("bad anchor parses before validation");
    let errors = validate_file(&file).expect_err("bad anchor should fail validation");
    assert!(
        errors.iter().any(|error| error
            .message
            .contains("must anchor a sibling semantic node")),
        "expected bad anchor error, found {errors:?}"
    );
}

#[test]
fn unsupported_syntax_produces_a_clear_error() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn run<T>() {}
}
"#;

    let error = parse_file(source).expect_err("generic functions are outside the bootstrap subset");
    assert!(
        error
            .to_string()
            .contains("unsupported token in the current Draxl Rust profile"),
        "unexpected unsupported syntax error: {error}"
    );
}

#[test]
fn implicit_doc_attachment_is_valid_and_stable() {
    let source = r#"
@m1 mod demo {
  @d1 /// Runs the job.
  @f2[b] fn second() {}
  @f1[a] fn first() {}
}
"#;

    let file = parse_file(source).expect("source should parse");
    validate_file(&file).expect("implicit doc attachment should validate");
    let formatted = print_file(&file);
    let first = formatted
        .find("@f1[a] fn first()")
        .expect("first function should exist");
    let doc = formatted
        .find("@d1 /// Runs the job.")
        .expect("doc comment should exist");
    let second = formatted
        .find("@f2[b] fn second()")
        .expect("second function should exist");
    assert!(
        first < doc && doc < second,
        "unexpected canonical order:\n{formatted}"
    );
}

#[test]
fn detached_comment_without_anchor_is_rejected() {
    let source = r#"
@m1 mod demo {
  @f1[a] fn run() {}
  @c1 // Detached.
}
"#;

    let file = parse_file(source).expect("detached comment parses before validation");
    let errors = validate_file(&file).expect_err("detached comment should fail validation");
    assert!(
        errors.iter().any(|error| error
            .message
            .contains("needs a following sibling or `->anchor`")),
        "expected detached comment error, found {errors:?}"
    );
}
