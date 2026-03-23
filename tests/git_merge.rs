use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{SystemTime, UNIX_EPOCH};

const BASE_SOURCE: &str = r#"fn price(amount: Cents) -> Cents {
    let subtotal =
        normalize(
            convert(
                amount
            )
        );

    audit();
    emit_metric("subtotal", subtotal);

    subtotal
}
"#;

const RENAME_SOURCE: &str = r#"fn price(amount: Cents) -> Cents {
    let subtotal_cents =
        normalize(
            convert(
                amount
            )
        );

    audit();
    emit_metric("subtotal", subtotal_cents);

    subtotal_cents
}
"#;

const UNIT_SOURCE: &str = r#"fn price(amount: Cents) -> Cents {
    let subtotal =
        normalize(
            convert(
                to_dollars(amount)
            )
        );

    audit();
    emit_metric("subtotal", subtotal);

    subtotal
}
"#;

const EXPECTED_MERGED_SOURCE: &str = r#"fn price(amount: Cents) -> Cents {
    let subtotal_cents =
        normalize(
            convert(
                to_dollars(amount)
            )
        );

    audit();
    emit_metric("subtotal", subtotal_cents);

    subtotal_cents
}
"#;

const CALL_BASE_SOURCE: &str = r#"fn charge(_amount: u64) {}
fn charge_dollars(_amount: u64) {}
fn to_cents(amount: u64) -> u64 { amount * 100 }
fn normalize(amount: u64) -> u64 { amount }
fn audit() {}

fn checkout(amount: u64) {
    charge(
        normalize(
            amount
        )
    );

    audit();
}
"#;

const CALL_CALLEE_SOURCE: &str = r#"fn charge(_amount: u64) {}
fn charge_dollars(_amount: u64) {}
fn to_cents(amount: u64) -> u64 { amount * 100 }
fn normalize(amount: u64) -> u64 { amount }
fn audit() {}

fn checkout(amount: u64) {
    charge_dollars(
        normalize(
            amount
        )
    );

    audit();
}
"#;

const CALL_ARGUMENT_SOURCE: &str = r#"fn charge(_amount: u64) {}
fn charge_dollars(_amount: u64) {}
fn to_cents(amount: u64) -> u64 { amount * 100 }
fn normalize(amount: u64) -> u64 { amount }
fn audit() {}

fn checkout(amount: u64) {
    charge(
        normalize(
            to_cents(amount)
        )
    );

    audit();
}
"#;

const EXPECTED_CALL_MERGED_SOURCE: &str = r#"fn charge(_amount: u64) {}
fn charge_dollars(_amount: u64) {}
fn to_cents(amount: u64) -> u64 { amount * 100 }
fn normalize(amount: u64) -> u64 { amount }
fn audit() {}

fn checkout(amount: u64) {
    charge_dollars(
        normalize(
            to_cents(amount)
        )
    );

    audit();
}
"#;

// This fixture is intentionally shaped so Git's text merge succeeds even though
// the merged result combines two semantically coupled edits that should be
// reviewed together.
#[test]
fn git_merges_semantic_conflict_without_reporting_a_text_conflict() {
    let repo = TempGitRepo::new();
    repo.write_file("price.rs", BASE_SOURCE);
    repo.git_ok(&["init", "-q"]);
    repo.git_ok(&["add", "price.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "base",
    ]);

    let base_branch = repo.git_stdout(&["branch", "--show-current"]);
    let base_branch = base_branch.trim();

    repo.git_ok(&["checkout", "-q", "-b", "rename-branch"]);
    repo.write_file("price.rs", RENAME_SOURCE);
    repo.git_ok(&["add", "price.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "rename subtotal",
    ]);

    repo.git_ok(&["checkout", "-q", base_branch]);
    repo.git_ok(&["checkout", "-q", "-b", "unit-branch"]);
    repo.write_file("price.rs", UNIT_SOURCE);
    repo.git_ok(&["add", "price.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "change subtotal unit",
    ]);

    let merge = repo.git(&["merge", "rename-branch"]);
    assert!(
        merge.status.success(),
        "expected merge success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    let merged = repo.read_file("price.rs");
    assert_eq!(merged, EXPECTED_MERGED_SOURCE);
    assert!(
        !merged.contains("<<<<<<<"),
        "merged file unexpectedly contains conflict markers:\n{merged}"
    );
}

#[test]
fn git_merges_call_contract_semantic_conflict_without_reporting_a_text_conflict() {
    let repo = TempGitRepo::new();
    repo.write_file("checkout.rs", CALL_BASE_SOURCE);
    repo.git_ok(&["init", "-q"]);
    repo.git_ok(&["add", "checkout.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "base",
    ]);

    let base_branch = repo.git_stdout(&["branch", "--show-current"]);
    let base_branch = base_branch.trim();

    repo.git_ok(&["checkout", "-q", "-b", "callee-branch"]);
    repo.write_file("checkout.rs", CALL_CALLEE_SOURCE);
    repo.git_ok(&["add", "checkout.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "change callee contract",
    ]);

    repo.git_ok(&["checkout", "-q", base_branch]);
    repo.git_ok(&["checkout", "-q", "-b", "argument-branch"]);
    repo.write_file("checkout.rs", CALL_ARGUMENT_SOURCE);
    repo.git_ok(&["add", "checkout.rs"]);
    repo.git_ok(&[
        "-c",
        "user.name=Codex",
        "-c",
        "user.email=codex@example.com",
        "commit",
        "-q",
        "-m",
        "change argument representation",
    ]);

    let merge = repo.git(&["merge", "callee-branch"]);
    assert!(
        merge.status.success(),
        "expected merge success\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&merge.stdout),
        String::from_utf8_lossy(&merge.stderr)
    );

    let merged = repo.read_file("checkout.rs");
    assert_eq!(merged, EXPECTED_CALL_MERGED_SOURCE);
    assert!(
        !merged.contains("<<<<<<<"),
        "merged file unexpectedly contains conflict markers:\n{merged}"
    );
}

struct TempGitRepo {
    path: PathBuf,
}

impl TempGitRepo {
    fn new() -> Self {
        let mut path = std::env::temp_dir();
        path.push(unique_temp_name("draxl_git_merge_test"));
        fs::create_dir_all(&path)
            .unwrap_or_else(|err| panic!("failed to create temp repo {}: {err}", path.display()));
        Self { path }
    }

    fn write_file(&self, relative: &str, contents: &str) {
        let path = self.path.join(relative);
        fs::write(&path, contents)
            .unwrap_or_else(|err| panic!("failed to write {}: {err}", path.display()));
    }

    fn read_file(&self, relative: &str) -> String {
        let path = self.path.join(relative);
        fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()))
    }

    fn git(&self, args: &[&str]) -> Output {
        Command::new("git")
            .args(args)
            .current_dir(&self.path)
            .output()
            .unwrap_or_else(|err| panic!("failed to run git {:?}: {err}", args))
    }

    fn git_ok(&self, args: &[&str]) {
        let output = self.git(args);
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn git_stdout(&self, args: &[&str]) -> String {
        let output = self.git(args);
        assert!(
            output.status.success(),
            "git {:?} failed\nstdout:\n{}\nstderr:\n{}",
            args,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).expect("stdout should be utf-8")
    }
}

impl Drop for TempGitRepo {
    fn drop(&mut self) {
        if let Err(err) = fs::remove_dir_all(&self.path) {
            if self.path.exists() {
                panic!("failed to remove temp repo {}: {err}", self.path.display());
            }
        }
    }
}

fn unique_temp_name(prefix: &str) -> String {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be monotonic enough for tests")
        .as_nanos();
    format!("{prefix}_{stamp}")
}
