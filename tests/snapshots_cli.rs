//! Snapshot tests on CLI output (insta): UX changes must be deliberate.
//! Runs with NO_COLOR so snapshots are colour-free.

use std::process::Command;

use assert_cmd::prelude::*;

fn run(args: &[&str]) -> String {
    let dir = tempfile::tempdir().expect("tempdir");
    let out = Command::cargo_bin("olivaw")
        .expect("binary builds")
        .args(args)
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .env("HOME", dir.path())
        .output()
        .expect("runs");
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn snapshot_list() {
    insta::assert_snapshot!(run(&["list"]));
}

#[test]
fn snapshot_info_l298n() {
    insta::assert_snapshot!(run(&["info", "drivers/l298n"]));
}

#[test]
fn snapshot_info_scan_matcher() {
    insta::assert_snapshot!(run(&["info", "slam/scan-matcher"]));
}

#[test]
fn snapshot_did_you_mean() {
    insta::assert_snapshot!(run(&["info", "sensors/mpu650"]));
}

#[test]
fn snapshot_help() {
    insta::assert_snapshot!(run(&["--help"]));
}
