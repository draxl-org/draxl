mod support;

use draxl_lower_rust::lower_file;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use support::{examples, parse_and_validate};

#[test]
fn lowered_examples_compile_as_rust_libraries() {
    for example in examples() {
        let file = parse_and_validate(example.path);
        let lowered = lower_file(&file);
        compile_example(example.slug, &lowered);
    }
}

fn compile_example(slug: &str, source: &str) {
    let input_path = write_temp_file(slug, source);
    let output_path = temp_path(slug, "rlib");
    let crate_name = format!("lowered_{}", sanitize_slug(slug));
    let input = input_path
        .to_str()
        .expect("temporary source path should be valid utf-8");
    let output = output_path
        .to_str()
        .expect("temporary output path should be valid utf-8");

    let result = Command::new("rustc")
        .args([
            "--edition",
            "2021",
            "--crate-type",
            "lib",
            "--crate-name",
            &crate_name,
            input,
            "-o",
            output,
        ])
        .output()
        .expect("rustc should be available while running tests");

    assert!(
        result.status.success(),
        "lowered Rust for {slug} did not compile\nstdout:\n{}\nstderr:\n{}\nsource:\n{source}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr),
    );

    let _ = fs::remove_file(input_path);
    let _ = fs::remove_file(output_path);
}

fn write_temp_file(slug: &str, source: &str) -> PathBuf {
    let path = temp_path(slug, "rs");
    fs::write(&path, source).expect("temporary lowered Rust file should be writable");
    path
}

fn temp_path(slug: &str, ext: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "draxl_{}_{}.{}",
        unique_stamp(),
        sanitize_slug(slug),
        ext
    ));
    path
}

fn sanitize_slug(slug: &str) -> String {
    slug.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn unique_stamp() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos()
}
