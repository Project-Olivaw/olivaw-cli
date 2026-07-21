//! Git-backed registry cache (distribution model v0.2).
//!
//! Layout: `~/.olivaw/cache/registry/<tag>/` — one immutable shallow checkout
//! per tag, `.git` stripped. Clones land in a `.tmp-<tag>` sibling and are
//! renamed into place, so a crashed fetch never leaves a half-valid cache.
//!
//! We shell out to the `git` CLI rather than linking git2/gix: the only
//! operation ever needed is `clone --depth 1 --branch <tag>`, everyone using
//! an embedded-Rust CLI has git, and a fetch failure already has a designed
//! fallback (the embedded registry).

use std::path::PathBuf;
use std::process::Command;

use anyhow::{bail, Context};

use crate::ui::Ui;
use crate::RegistryOpts;
use super::RegistrySource;

/// Canonical registry repo. The registry lives inside the olivaw-cli repo
/// itself; tags `registry-v<version>` pin its content.
const DEFAULT_URL: &str = "https://github.com/Project-Olivaw/olivaw-cli.git";

fn registry_url() -> String {
    std::env::var("OLIVAW_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_URL.to_string())
}

fn pinned_tag(opts: &RegistryOpts) -> String {
    opts.tag_override
        .clone()
        .unwrap_or_else(|| concat!("registry-v", env!("CARGO_PKG_VERSION")).to_string())
}

fn cache_root() -> anyhow::Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME is not set — cannot locate ~/.olivaw/cache")?;
    Ok(home.join(".olivaw").join("cache").join("registry"))
}

/// The cached checkout for the pinned tag, if present and valid.
/// A tag dir is valid iff it contains `registry.toml` (tags are immutable, so
/// a valid dir is never re-fetched).
pub fn find_valid_cache(opts: &RegistryOpts) -> anyhow::Result<Option<RegistrySource>> {
    let tag = pinned_tag(opts);
    let dir = cache_root()?.join(&tag);
    if dir.join("registry.toml").is_file() {
        Ok(Some(RegistrySource::Cached { root: dir, tag }))
    } else {
        Ok(None)
    }
}

/// Ensure the pinned tag is cached, cloning it if missing. Errors bubble up to
/// the caller, which degrades to the embedded registry with a note.
pub fn ensure_cached(ui: &Ui, opts: &RegistryOpts) -> anyhow::Result<RegistrySource> {
    if let Some(hit) = find_valid_cache(opts)? {
        return Ok(hit);
    }

    let tag = pinned_tag(opts);
    let url = registry_url();
    let root = cache_root()?;
    let final_dir = root.join(&tag);
    let tmp_dir = root.join(format!(".tmp-{tag}"));
    std::fs::create_dir_all(&root)
        .with_context(|| format!("creating {}", root.display()))?;
    // A stale tmp dir from a crashed run would make clone fail; clear it.
    if tmp_dir.exists() {
        std::fs::remove_dir_all(&tmp_dir)
            .with_context(|| format!("clearing stale {}", tmp_dir.display()))?;
    }

    let spinner = ui.spinner(&format!("fetching registry {tag}"));
    let output = Command::new("git")
        .args(["clone", "--depth", "1", "--branch", &tag, "--quiet"])
        .arg(&url)
        .arg(&tmp_dir)
        .output();
    if let Some(pb) = spinner {
        pb.finish_and_clear();
    }

    let output = output.context("running git (is git installed?)")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_dir_all(&tmp_dir);
        bail!("git clone of {url} at {tag} failed: {}", stderr.trim());
    }

    // The registry lives in the repo's registry/ subdirectory: promote it and
    // drop everything else (.git included) so the cache is plain data.
    let registry_subdir = tmp_dir.join("registry");
    let payload = if registry_subdir.join("registry.toml").is_file() {
        registry_subdir
    } else if tmp_dir.join("registry.toml").is_file() {
        // Also accept a bare registry repo (registry.toml at its root).
        tmp_dir.clone()
    } else {
        let _ = std::fs::remove_dir_all(&tmp_dir);
        bail!("checkout of {url} at {tag} contains no registry.toml");
    };

    if final_dir.exists() {
        std::fs::remove_dir_all(&final_dir)
            .with_context(|| format!("clearing {}", final_dir.display()))?;
    }
    std::fs::rename(&payload, &final_dir)
        .with_context(|| format!("installing cache at {}", final_dir.display()))?;
    let _ = std::fs::remove_dir_all(&tmp_dir);

    Ok(RegistrySource::Cached {
        root: final_dir,
        tag,
    })
}
