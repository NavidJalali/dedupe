//! End-to-end tests that drive the compiled `dedupe` binary, covering the
//! wiring in `main.rs` (argument validation, the scan → dedupe pipeline, and
//! the dry-run / recursive flags) that the unit tests can't reach.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::tempdir;

/// A `Command` pointing at the binary Cargo built for this test run.
fn dedupe() -> Command {
    Command::new(env!("CARGO_BIN_EXE_dedupe"))
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    let mut file = fs::File::create(&path).expect("create file");
    file.write_all(bytes).expect("write file");
    path
}

#[test]
fn rejects_zero_threads() {
    let dir = tempdir().unwrap();
    let output = dedupe()
        .arg("--threads")
        .arg("0")
        .arg(dir.path())
        .output()
        .expect("run dedupe");

    assert_eq!(output.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("number of threads must be positive"),
        "unexpected stderr: {stderr}"
    );
}

#[test]
fn removes_duplicate_end_to_end() {
    let dir = tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", b"identical contents");
    let b = write_file(dir.path(), "b.txt", b"identical contents");

    let status = dedupe().arg(dir.path()).status().expect("run dedupe");
    assert!(status.success());

    assert!(a.exists(), "the kept original should remain");
    assert!(!b.exists(), "the duplicate should be removed");
}

#[test]
fn dry_run_keeps_all_files() {
    let dir = tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", b"identical contents");
    let b = write_file(dir.path(), "b.txt", b"identical contents");

    let output = dedupe()
        .arg("--dry-run")
        .arg(dir.path())
        .output()
        .expect("run dedupe");

    assert!(output.status.success());
    assert!(a.exists());
    assert!(b.exists(), "dry-run must not delete anything");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Would remove"),
        "dry-run should report intended removals: {stdout}"
    );
}

#[test]
fn non_recursive_ignores_nested_duplicates() {
    let dir = tempdir().unwrap();
    let top = write_file(dir.path(), "a.txt", b"identical contents");
    let sub = dir.path().join("nested");
    fs::create_dir(&sub).unwrap();
    let deep = write_file(&sub, "b.txt", b"identical contents");

    let status = dedupe().arg(dir.path()).status().expect("run dedupe");
    assert!(status.success());

    // Without --recursive only the top-level file is scanned, so it never
    // forms a duplicate group and nothing is removed.
    assert!(top.exists());
    assert!(deep.exists());
}

#[test]
fn recursive_finds_nested_duplicates() {
    let dir = tempdir().unwrap();
    let top = write_file(dir.path(), "a.txt", b"identical contents");
    let sub = dir.path().join("nested");
    fs::create_dir(&sub).unwrap();
    let deep = write_file(&sub, "b.txt", b"identical contents");

    let status = dedupe()
        .arg("--recursive")
        .arg(dir.path())
        .status()
        .expect("run dedupe");
    assert!(status.success());

    // Exactly one of the two identical files should survive.
    let remaining = [&top, &deep].iter().filter(|p| p.exists()).count();
    assert_eq!(remaining, 1, "recursive run should remove one duplicate");
}

#[test]
fn distinct_files_survive() {
    let dir = tempdir().unwrap();
    let a = write_file(dir.path(), "a.txt", b"alpha");
    let b = write_file(dir.path(), "b.txt", b"beta");

    let status = dedupe().arg(dir.path()).status().expect("run dedupe");
    assert!(status.success());

    assert!(a.exists());
    assert!(b.exists());
}

#[test]
fn missing_roots_is_a_usage_error() {
    let output = dedupe().output().expect("run dedupe");
    assert!(
        !output.status.success(),
        "running without roots should fail"
    );
}

#[test]
fn help_flag_succeeds() {
    let output = dedupe().arg("--help").output().expect("run dedupe");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage"),
        "help should print usage: {stdout}"
    );
}

#[test]
fn version_flag_succeeds() {
    let output = dedupe().arg("--version").output().expect("run dedupe");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "version should print package version: {stdout}"
    );
}
