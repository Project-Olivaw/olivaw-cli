//! Integration tests: real `olivaw` binary against temp projects.
//!
//! Every test runs with `NO_COLOR=1`, a temp `HOME` (so no real registry
//! cache is touched) and `--offline` for write commands (no network in
//! tests).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::prelude::*;
use predicates::prelude::*;

/// A scratch Rust project in a tempdir, with an isolated HOME.
struct Scratch {
    _dir: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Scratch {
    fn new() -> Scratch {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join("proj");
        let home = dir.path().join("home");
        fs::create_dir_all(root.join("src")).expect("mkdir");
        fs::create_dir_all(&home).expect("mkdir");
        fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"scratch-bot\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n",
        )
        .expect("write Cargo.toml");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("write main.rs");
        Scratch {
            _dir: dir,
            root,
            home,
        }
    }

    /// An empty directory (no Cargo.toml) inside the tempdir.
    fn bare(&self) -> PathBuf {
        let bare = self.root.parent().expect("parent").join("bare");
        fs::create_dir_all(&bare).expect("mkdir");
        bare
    }

    fn olivaw(&self) -> Command {
        let mut cmd = Command::cargo_bin("olivaw").expect("binary builds");
        cmd.current_dir(&self.root)
            .env("NO_COLOR", "1")
            .env("HOME", &self.home);
        cmd
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.root.join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
    }

    fn exists(&self, rel: &str) -> bool {
        self.root.join(rel).exists()
    }
}

// ---------------------------------------------------------------- list/info

#[test]
fn list_shows_all_components_grouped() {
    let s = Scratch::new();
    s.olivaw()
        .arg("list")
        .assert()
        .success()
        .stdout(predicate::str::contains("sensors"))
        .stdout(predicate::str::contains("mpu6050"))
        .stdout(predicate::str::contains("slam"))
        .stdout(predicate::str::contains("scan-matcher"))
        .stdout(predicate::str::contains("9 components"))
        .stdout(predicate::str::contains("registry v"));
}

#[test]
fn list_filters_by_category_and_suggests_on_typo() {
    let s = Scratch::new();
    s.olivaw()
        .args(["list", "drivers"])
        .assert()
        .success()
        .stdout(predicate::str::contains("l298n"))
        .stdout(predicate::str::contains("2 components"));
    s.olivaw()
        .args(["list", "driver"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("did you mean 'drivers'?"));
}

#[test]
fn info_shows_hardware_deps_and_verification() {
    let s = Scratch::new();
    s.olivaw()
        .args(["info", "drivers/l298n"])
        .assert()
        .success()
        .stdout(predicate::str::contains("drivers/l298n"))
        .stdout(predicate::str::contains("embedded-hal = \"1.0\""))
        .stdout(predicate::str::contains("NOT yet verified"))
        .stdout(predicate::str::contains("olivaw add drivers/l298n"));
}

#[test]
fn info_suggests_close_component_names() {
    let s = Scratch::new();
    s.olivaw()
        .args(["info", "sensors/mpu650"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("did you mean 'sensors/mpu6050'?"))
        .stderr(predicate::str::contains("olivaw list sensors"));
}

#[test]
fn malformed_component_path_explains_the_format() {
    let s = Scratch::new();
    s.olivaw()
        .args(["info", "mpu6050"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("<category>/<name>"));
}

// ---------------------------------------------------------------------- add

#[test]
fn add_refuses_outside_a_rust_project() {
    let s = Scratch::new();
    let bare = s.bare();
    let mut cmd = Command::cargo_bin("olivaw").expect("binary builds");
    cmd.current_dir(&bare)
        .env("NO_COLOR", "1")
        .env("HOME", &s.home)
        .args(["add", "drivers/l298n", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("no Cargo.toml"))
        .stderr(predicate::str::contains("olivaw init"));
}

#[test]
fn add_happy_path_writes_files_manifest_and_cargo_toml() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "sensors/mpu6050", "--offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added sensors/mpu6050"))
        .stdout(predicate::str::contains("+ embedded-hal = \"1.0\""))
        .stdout(predicate::str::contains("Wiring:"))
        .stdout(predicate::str::contains("cargo run --example mpu6050_read"));

    assert!(s.exists("src/sensors/mpu6050.rs"));
    assert!(s.exists("examples/mpu6050_read.rs"));

    let manifest = s.read("olivaw.toml");
    assert!(manifest.contains("[components.\"sensors/mpu6050\"]"));
    assert!(manifest.contains("sha256:"), "checksums must be recorded");
    assert!(
        manifest.contains("name = \"scratch-bot\""),
        "package name from Cargo.toml"
    );

    let cargo = s.read("Cargo.toml");
    assert!(cargo.contains("embedded-hal = \"1.0\""));
}

#[test]
fn add_refuses_to_overwrite_existing_files_without_force() {
    let s = Scratch::new();
    fs::create_dir_all(s.root.join("src/drivers")).expect("mkdir");
    fs::write(s.root.join("src/drivers/l298n.rs"), "// mine\n").expect("write");
    s.olivaw()
        .args(["add", "drivers/l298n", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exist"))
        .stderr(predicate::str::contains("--force"));
    assert_eq!(
        s.read("src/drivers/l298n.rs"),
        "// mine\n",
        "file untouched"
    );

    s.olivaw()
        .args(["add", "drivers/l298n", "--offline", "--force"])
        .assert()
        .success();
    assert!(s.read("src/drivers/l298n.rs").contains("pub struct Motor"));
}

#[test]
fn add_never_touches_an_existing_cargo_dependency() {
    let s = Scratch::new();
    fs::write(
        s.root.join("Cargo.toml"),
        "# keep me\n[package]\nname = \"scratch-bot\"\nversion = \"0.1.0\"\n\n[dependencies]\nembedded-hal = \"0.2\"   # pinned old\n",
    )
    .expect("write");
    s.olivaw()
        .args(["add", "drivers/l298n", "--offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already in Cargo.toml as \"0.2\""))
        .stdout(predicate::str::contains("left untouched"));
    let cargo = s.read("Cargo.toml");
    assert!(cargo.contains("# keep me"));
    assert!(cargo.contains("embedded-hal = \"0.2\"   # pinned old"));
}

#[test]
fn add_already_installed_is_a_friendly_no_op() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("already installed"));
}

#[test]
fn add_with_path_prefix_stays_inside_the_project() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline", "--path", "vendor"])
        .assert()
        .success();
    assert!(s.exists("vendor/src/drivers/led.rs"));

    s.olivaw()
        .args(["add", "drivers/l298n", "--offline", "--path", "../evil"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("inside the project"));
}

// -------------------------------------------------------- dependency resolve

#[test]
fn add_resolves_component_dependencies_in_order() {
    let s = Scratch::new();
    // Non-interactive without --force: refuse, naming the extra component.
    s.olivaw()
        .args(["add", "slam/scan-matcher", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--force"));

    let out = s
        .olivaw()
        .args(["add", "slam/scan-matcher", "--offline", "--force"])
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout).to_string();
    let dep = stdout.find("Added slam/core-types").expect("dep installed");
    let root = stdout
        .find("Added slam/scan-matcher")
        .expect("root installed");
    assert!(dep < root, "dependency must install before dependent");

    assert!(s.exists("src/slam/pose.rs"));
    assert!(s.exists("src/slam/matcher/icp.rs"));
    let manifest = s.read("olivaw.toml");
    assert!(manifest.contains("[components.\"slam/core-types\"]"));
    assert!(manifest.contains("[components.\"slam/scan-matcher\"]"));
}

// --------------------------------------------------------------------- init

#[test]
fn init_scaffolds_a_named_linux_project() {
    let s = Scratch::new();
    let parent = s.root.parent().expect("parent").to_path_buf();
    let mut cmd = Command::cargo_bin("olivaw").expect("binary builds");
    cmd.current_dir(&parent)
        .env("NO_COLOR", "1")
        .env("HOME", &s.home)
        .args(["init", "--name", "my-bot", "--target", "linux"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Created my-bot/"));
    for file in [
        "Cargo.toml",
        "src/main.rs",
        "olivaw.toml",
        ".gitignore",
        "README.md",
    ] {
        assert!(parent.join("my-bot").join(file).exists(), "missing {file}");
    }
    let cargo = fs::read_to_string(parent.join("my-bot/Cargo.toml")).expect("read");
    assert!(
        cargo.contains("name = \"my-bot\""),
        "project name substituted"
    );
    let manifest = fs::read_to_string(parent.join("my-bot/olivaw.toml")).expect("read");
    assert!(manifest.contains("target = \"linux\""));
}

#[test]
fn init_rp2040_scaffold_has_the_flash_plumbing() {
    let s = Scratch::new();
    let parent = s.root.parent().expect("parent").to_path_buf();
    let mut cmd = Command::cargo_bin("olivaw").expect("binary builds");
    cmd.current_dir(&parent)
        .env("NO_COLOR", "1")
        .env("HOME", &s.home)
        .args(["init", "--name", "pico-bot", "--target", "rp2040"])
        .assert()
        .success();
    for file in [
        "memory.x",
        "build.rs",
        ".cargo/config.toml",
        "rust-toolchain.toml",
    ] {
        assert!(
            parent.join("pico-bot").join(file).exists(),
            "missing {file}"
        );
    }
    let config = fs::read_to_string(parent.join("pico-bot/.cargo/config.toml")).expect("read");
    assert!(config.contains("thumbv6m-none-eabi"));
    assert!(config.contains("elf2uf2-rs"));
}

#[test]
fn init_refuses_to_clobber_existing_files() {
    let s = Scratch::new();
    let parent = s.root.parent().expect("parent").to_path_buf();
    fs::create_dir_all(parent.join("taken/src")).expect("mkdir");
    fs::write(
        parent.join("taken/src/main.rs"),
        "fn main() { /* mine */ }\n",
    )
    .expect("write");
    let mut cmd = Command::cargo_bin("olivaw").expect("binary builds");
    cmd.current_dir(&parent)
        .env("NO_COLOR", "1")
        .env("HOME", &s.home)
        .args(["init", "--name", "taken", "--target", "linux"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("already exist"));
    let main = fs::read_to_string(parent.join("taken/src/main.rs")).expect("read");
    assert!(main.contains("/* mine */"), "existing file untouched");
}

#[test]
fn init_rejects_bad_project_names() {
    let s = Scratch::new();
    s.olivaw()
        .args(["init", "--name", "bad name!", "--target", "linux"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not a valid project name"));
}

// ------------------------------------------------------------- update/check

#[test]
fn check_reports_clean_and_exits_zero() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    s.olivaw()
        .arg("check")
        .assert()
        .success()
        .stdout(predicate::str::contains("clean"));
    // --quiet prints nothing when clean.
    let out = s.olivaw().args(["check", "--quiet"]).assert().success();
    assert!(out.get_output().stdout.is_empty());
}

#[test]
fn check_detects_modified_and_missing_files_with_exit_code_one() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    // Drift: edit one file, delete another.
    let led = s.root.join("src/drivers/led.rs");
    let mut text = fs::read_to_string(&led).expect("read");
    text.push_str("// tweak\n");
    fs::write(&led, text).expect("write");
    fs::remove_file(s.root.join("examples/led_modes.rs")).expect("rm");

    s.olivaw()
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("modified"))
        .stdout(predicate::str::contains("missing"))
        .stdout(predicate::str::contains("olivaw update drivers/led"));
}

#[test]
fn check_flags_missing_cargo_dependencies() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/l298n", "--offline"])
        .assert()
        .success();
    // Remove the dep the component needs.
    fs::write(
        s.root.join("Cargo.toml"),
        "[package]\nname = \"scratch-bot\"\nversion = \"0.1.0\"\n\n[dependencies]\n",
    )
    .expect("write");
    s.olivaw()
        .arg("check")
        .assert()
        .code(1)
        .stdout(predicate::str::contains("embedded-hal"))
        .stdout(predicate::str::contains("needed by drivers/l298n"));
}

#[test]
fn update_refuses_to_clobber_local_edits_non_interactively() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    let led = s.root.join("src/drivers/led.rs");
    let mine = format!(
        "{}// my precious edit\n",
        fs::read_to_string(&led).expect("read")
    );
    fs::write(&led, &mine).expect("write");

    s.olivaw()
        .args(["update", "drivers/led", "--offline"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("modified by you"))
        .stdout(predicate::str::contains("my precious edit"))
        .stderr(predicate::str::contains("refusing to overwrite"));
    assert_eq!(s.read("src/drivers/led.rs"), mine, "edit must survive");
}

#[test]
fn update_dry_run_writes_nothing() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    let led = s.root.join("src/drivers/led.rs");
    let mine = format!("{}// keep\n", fs::read_to_string(&led).expect("read"));
    fs::write(&led, &mine).expect("write");

    s.olivaw()
        .args(["update", "drivers/led", "--offline", "--dry-run"])
        .assert()
        .success()
        .stdout(predicate::str::contains("nothing written"));
    assert_eq!(s.read("src/drivers/led.rs"), mine);
}

#[test]
fn update_force_restores_pristine_content_and_new_checksums() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    let led = s.root.join("src/drivers/led.rs");
    fs::write(&led, "// clobbered\n").expect("write");

    s.olivaw()
        .args(["update", "drivers/led", "--offline", "--force"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated drivers/led"));
    assert!(s.read("src/drivers/led.rs").contains("pub struct Blinker"));
    s.olivaw().arg("check").assert().success();
}

#[test]
fn update_of_up_to_date_component_says_so() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .assert()
        .success();
    s.olivaw()
        .args(["update", "drivers/led", "--offline"])
        .assert()
        .success()
        .stdout(predicate::str::contains("up to date"));
}

#[test]
fn update_of_uninstalled_component_points_to_add() {
    let s = Scratch::new();
    s.olivaw()
        .args(["update", "drivers/led", "--offline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("olivaw add drivers/led"));
}

// ------------------------------------------------------- git-backed registry

/// Build a git repo that acts as a remote registry with one extra component,
/// tagged `registry-vTEST`.
fn build_remote_registry(dir: &Path) -> String {
    let repo = dir.join("remote-registry");
    let reg = repo.join("registry");
    fs::create_dir_all(reg.join("drivers/servo/src")).expect("mkdir");
    fs::write(
        reg.join("registry.toml"),
        "[registry]\nversion = \"9.9.9\"\n\n[components.\"drivers/servo\"]\nversion = \"0.1.0\"\ndescription = \"A remote-only test component\"\n",
    )
    .expect("write");
    fs::write(
        reg.join("drivers/servo/component.toml"),
        "[component]\nname = \"servo\"\ncategory = \"drivers\"\nversion = \"0.1.0\"\ndescription = \"servo\"\nlicense = \"MIT\"\n\n[[files]]\nsrc = \"src/servo.rs\"\ndest = \"src/drivers/servo.rs\"\n",
    )
    .expect("write");
    fs::write(reg.join("drivers/servo/src/servo.rs"), "//! test servo\n").expect("write");

    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(args)
            .current_dir(&repo)
            .env("GIT_AUTHOR_NAME", "test")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "test")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .expect("git runs");
        assert!(
            ok.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&ok.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["add", "."]);
    git(&["commit", "-qm", "registry"]);
    git(&["tag", "registry-vTEST"]);
    repo.to_string_lossy().into_owned()
}

#[test]
fn fetches_registry_from_git_at_pinned_tag_and_caches_it() {
    let s = Scratch::new();
    let url = build_remote_registry(s.root.parent().expect("parent"));

    s.olivaw()
        .args(["add", "drivers/servo", "--registry-tag", "registry-vTEST"])
        .env("OLIVAW_REGISTRY_URL", &url)
        .assert()
        .success()
        .stdout(predicate::str::contains("Added drivers/servo"));
    assert!(s.exists("src/drivers/servo.rs"));
    assert!(
        s.home
            .join(".olivaw/cache/registry/registry-vTEST/registry.toml")
            .is_file(),
        "checkout cached under ~/.olivaw"
    );

    // Second run must hit the cache (kill the remote to prove it).
    fs::remove_dir_all(&url).expect("rm remote");
    s.olivaw()
        .args(["list", "--registry-tag", "registry-vTEST"])
        .assert()
        .success()
        .stdout(predicate::str::contains("servo"))
        .stdout(predicate::str::contains("cache (registry-vTEST)"));
}

#[test]
fn fetch_failure_degrades_to_embedded_registry_with_a_note() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--registry-tag", "registry-vNOPE"])
        .env("OLIVAW_REGISTRY_URL", "/nonexistent/repo.git")
        .assert()
        .success()
        .stdout(predicate::str::contains("registry fetch failed"))
        .stdout(predicate::str::contains("Added drivers/led"));
}

#[test]
fn offline_flag_skips_fetch_entirely() {
    let s = Scratch::new();
    s.olivaw()
        .args(["add", "drivers/led", "--offline"])
        .env("OLIVAW_REGISTRY_URL", "/nonexistent/repo.git")
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Added drivers/led")
                .and(predicate::str::contains("fetch failed").not()),
        );
}
