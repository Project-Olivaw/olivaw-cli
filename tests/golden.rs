//! Golden test: every registry component must actually compile when vendored
//! into a scaffolded project — the registry's core promise.
//!
//! `#[ignore]` because it builds real cargo projects (slow, network for
//! crates.io). CI runs it with `cargo test --test golden -- --ignored`.
//! Locally: same command.
//!
//! Scope: the host-checkable path (linux scaffold). The esp32 scaffold needs
//! the Xtensa toolchain and is covered by the CI container job.

use std::fs;
use std::process::Command;

use assert_cmd::prelude::*;

#[test]
#[ignore = "slow: builds a real cargo project; run explicitly / in CI"]
fn every_component_compiles_in_a_scaffolded_project() {
    let dir = tempfile::tempdir().expect("tempdir");
    let home = dir.path().join("home");
    fs::create_dir_all(&home).expect("mkdir");

    // 1. Scaffold.
    Command::cargo_bin("olivaw")
        .expect("binary builds")
        .current_dir(dir.path())
        .env("NO_COLOR", "1")
        .env("HOME", &home)
        .args(["init", "--name", "golden-bot", "--target", "linux"])
        .assert()
        .success();
    let proj = dir.path().join("golden-bot");

    // 2. Vendor every component in the registry.
    let components = [
        "comms/cmdvel-protocol",
        "drivers/l298n",
        "drivers/led",
        "kinematics/differential-drive",
        "sensors/hcsr04",
        "sensors/mpu6050",
        "sensors/rplidar",
        "slam/core-types",
        "slam/scan-matcher",
    ];
    for component in components {
        Command::cargo_bin("olivaw")
            .expect("binary builds")
            .current_dir(&proj)
            .env("NO_COLOR", "1")
            .env("HOME", &home)
            .args(["add", component, "--offline", "--force"])
            .assert()
            .success();
    }

    // 3. Wire the module tree (the one manual step `add` prints).
    fs::write(
        proj.join("src/main.rs"),
        "mod comms { pub mod cmdvel; }\n\
         mod drivers { pub mod l298n; pub mod led; }\n\
         mod kinematics { pub mod differential_drive; }\n\
         mod sensors { pub mod hcsr04; pub mod mpu6050; pub mod rplidar; }\n\
         mod slam { pub mod error; pub mod matcher; pub mod pose; pub mod scan; }\n\
         fn main() { println!(\"golden\"); }\n",
    )
    .expect("write main.rs");

    // 4. The vendored tree must compile — bins, examples, and unit tests.
    let out = Command::new("cargo")
        .args(["test", "--all-targets"])
        .current_dir(&proj)
        .output()
        .expect("cargo runs");
    assert!(
        out.status.success(),
        "vendored project failed to build/test:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
}
