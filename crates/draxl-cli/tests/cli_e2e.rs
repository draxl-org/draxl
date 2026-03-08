use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn parse_command_reports_top_level_item_count() {
    let output = run_ok(&[
        "parse",
        repo_path("examples/01_add.rs.dx").to_str().unwrap(),
    ]);
    assert!(output.contains("parsed"));
    assert!(output.contains("1 top-level item(s)"));
}

#[test]
fn validate_command_accepts_examples() {
    let output = run_ok(&[
        "validate",
        repo_path("examples/03_match.rs.dx").to_str().unwrap(),
    ]);
    assert!(output.contains("valid"));
}

#[test]
fn fmt_command_matches_the_canonical_golden() {
    let output = run_ok(&[
        "fmt",
        repo_path("examples/02_shapes.rs.dx").to_str().unwrap(),
    ]);
    let expected = read_repo("tests/golden/02_shapes.fmt.rs.dx");
    assert_eq!(output, expected);
}

#[test]
fn dump_json_command_matches_the_json_golden() {
    let output = run_ok(&[
        "dump-json",
        repo_path("examples/01_add.rs.dx").to_str().unwrap(),
    ]);
    let expected = read_repo("tests/golden/01_add.json");
    assert_eq!(output, expected);
}

#[test]
fn lower_rust_command_matches_the_lowered_golden() {
    let output = run_ok(&[
        "lower-rust",
        repo_path("examples/04_ranks.rs.dx").to_str().unwrap(),
    ]);
    let expected = read_repo("tests/golden/04_ranks.lowered.rs");
    assert_eq!(output, expected);
}

#[test]
fn fmt_in_place_rewrites_the_file() {
    let path = write_temp_file(
        "fmt_in_place.rs.dx",
        r#"
@m1 mod demo {
@f1[a] fn run() {}
}
"#,
    );

    run_ok(&["fmt", "--in-place", path.to_str().unwrap()]);

    let rewritten = fs::read_to_string(&path).expect("temporary file should be readable");
    assert!(rewritten.contains("  @f1[a] fn run() {"));
}

#[test]
fn validate_command_reports_errors_on_invalid_input() {
    let path = write_temp_file(
        "invalid_validate.rs.dx",
        r#"
@m1 mod demo {
  @f1 fn run() {}
}
"#,
    );

    let output = run_err(&["validate", path.to_str().unwrap()]);
    assert!(output.contains("validation failed:"));
    assert!(output.contains("missing `rank`"));
}

fn run_ok(args: &[&str]) -> String {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("command should run");
    assert!(
        output.status.success(),
        "expected success, stderr was:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("stdout should be utf-8")
}

fn run_err(args: &[&str]) -> String {
    let output = Command::new(binary_path())
        .args(args)
        .output()
        .expect("command should run");
    assert!(!output.status.success(), "expected failure");
    String::from_utf8(output.stderr).expect("stderr should be utf-8")
}

fn binary_path() -> PathBuf {
    if let Some(path) = option_env!("CARGO_BIN_EXE_draxl") {
        let path = PathBuf::from(path);
        if path.exists() {
            return path;
        }
    }

    repo_root().join("target/debug").join(binary_name())
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[cfg(windows)]
fn binary_name() -> &'static str {
    "draxl.exe"
}

#[cfg(not(windows))]
fn binary_name() -> &'static str {
    "draxl"
}

fn repo_path(relative: &str) -> PathBuf {
    repo_root().join(relative)
}

fn read_repo(relative: &str) -> String {
    fs::read_to_string(repo_path(relative))
        .unwrap_or_else(|err| panic!("failed to read {relative}: {err}"))
}

fn write_temp_file(name: &str, contents: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(unique_temp_name(name));
    fs::write(&path, contents.trim_start()).expect("temporary file should be writable");
    path
}

fn unique_temp_name(name: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    format!("draxl_{stamp}_{name}")
}
