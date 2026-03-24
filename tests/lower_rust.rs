mod support;

use draxl_rust::lower_file;
use support::{examples, parse_and_validate, read};

#[test]
fn examples_lower_to_the_expected_rust() {
    for example in examples() {
        let file = parse_and_validate(example.path);
        let lowered = lower_file(&file);
        let expected = read(&format!("tests/golden/{}.lowered.rs", example.slug));
        assert_eq!(
            lowered, expected,
            "lowered rust mismatch for {}",
            example.slug
        );
    }
}
