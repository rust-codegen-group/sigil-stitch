use std::path::PathBuf;
use std::process::{Command, Output};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/compatibility/semver-fixtures")
        .join(name)
}

fn compare(output: &str, allowlist: &str) -> Output {
    Command::new("bash")
        .arg(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("scripts/check-semver.sh"))
        .arg("--compare")
        .arg(fixture(output))
        .arg(fixture(allowlist))
        .output()
        .unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn zero_records_match_an_empty_allowlist() {
    assert!(compare("zero.out", "empty.allowlist").status.success());
}

#[test]
fn expected_record_matches_exactly() {
    assert!(
        compare("expected.out", "expected.allowlist")
            .status
            .success()
    );
}

#[test]
fn non_exhaustive_enum_record_matches_exactly() {
    assert!(
        compare("non-exhaustive-enum.out", "non-exhaustive-enum.allowlist")
            .status
            .success()
    );
}

#[test]
fn duplicate_tool_records_are_one_canonical_identity() {
    assert!(
        compare("duplicate-output.out", "expected.allowlist")
            .status
            .success()
    );
}

#[test]
fn malformed_tool_records_fail_closed() {
    let output = compare("malformed.out", "empty.allowlist");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("output was malformed"));
}

#[test]
fn approved_record_followed_by_tool_failure_fails_closed() {
    let output = compare("aborted.out", "expected.allowlist");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("output was malformed"));
}

#[test]
fn missing_approved_records_fail() {
    let output = compare("expected.out", "missing.allowlist");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("missing approved record"));
}

#[test]
fn unexpected_records_fail() {
    let output = compare("expected.out", "empty.allowlist");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("unexpected semver record"));
}

#[test]
fn duplicate_approved_records_fail() {
    let output = compare("expected.out", "duplicate.allowlist");
    assert!(!output.status.success());
    assert!(stderr(&output).contains("duplicate approved record"));
}
