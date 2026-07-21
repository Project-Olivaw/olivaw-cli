//! The user's project: root discovery and the `RelPath` safety type.
//!
//! olivaw never writes outside the project directory. `RelPath` is the only
//! path type the plan executor accepts, and it is impossible to construct one
//! that escapes the root — `component.toml` destinations are data and are
//! treated as hostile until proven safe here.

pub mod cargo;
pub mod manifest;

use std::fmt;
use std::path::{Component as PathComponent, Path, PathBuf};

use anyhow::{bail, Context};

/// A project rooted at the directory containing `Cargo.toml`.
pub struct Project {
    pub root: PathBuf,
}

impl Project {
    /// Walk up from the current directory to the nearest `Cargo.toml`.
    pub fn discover() -> anyhow::Result<Project> {
        let cwd = std::env::current_dir().context("reading current directory")?;
        let mut dir: &Path = &cwd;
        loop {
            if dir.join("Cargo.toml").is_file() {
                return Ok(Project {
                    root: dir.to_path_buf(),
                });
            }
            match dir.parent() {
                Some(parent) => dir = parent,
                None => bail!(
                    "not inside a Rust project (no Cargo.toml found from {} upward). \
                     olivaw vendors code into an existing project — cd into one, \
                     or create one with 'olivaw init --target <esp32|rp2040|linux>'",
                    cwd.display()
                ),
            }
        }
    }

    pub fn manifest_path(&self) -> PathBuf {
        self.root.join("olivaw.toml")
    }

    pub fn cargo_toml_path(&self) -> PathBuf {
        self.root.join("Cargo.toml")
    }
}

/// A relative path proven safe at construction: no absolute paths, no `..`,
/// no `~`, no drive/UNC prefixes. Joining it under a root cannot escape it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RelPath(String);

impl RelPath {
    pub fn new(s: &str) -> anyhow::Result<RelPath> {
        if s.is_empty() {
            bail!("empty destination path in component manifest");
        }
        if s.starts_with('~') {
            bail!("destination path '{s}' must not reference the home directory");
        }
        let path = Path::new(s);
        for comp in path.components() {
            match comp {
                PathComponent::Normal(_) | PathComponent::CurDir => {}
                PathComponent::ParentDir => {
                    bail!("destination path '{s}' must not contain '..'")
                }
                PathComponent::RootDir | PathComponent::Prefix(_) => {
                    bail!("destination path '{s}' must be relative to the project root")
                }
            }
        }
        // Normalize away any leading "./" so recorded paths are canonical.
        let normalized: PathBuf = path
            .components()
            .filter(|c| !matches!(c, PathComponent::CurDir))
            .collect();
        let normalized = normalized
            .to_str()
            .with_context(|| format!("destination path '{s}' is not valid UTF-8"))?
            .to_string();
        if normalized.is_empty() {
            bail!("destination path '{s}' resolves to nothing");
        }
        Ok(RelPath(normalized))
    }

    /// Prefix with a (validated) directory: `dir/self`.
    pub fn prefixed(&self, dir: &RelPath) -> RelPath {
        RelPath(format!("{}/{}", dir.0, self.0))
    }

    /// Absolute path under `root`. Safe by construction.
    pub fn join_under(&self, root: &Path) -> PathBuf {
        root.join(&self.0)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RelPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_normal_relative_paths() {
        for ok in [
            "src/sensors/mpu6050.rs",
            "examples/read.rs",
            "./src/lib.rs",
            "a",
        ] {
            assert!(RelPath::new(ok).is_ok(), "should accept {ok:?}");
        }
        assert_eq!(
            RelPath::new("./src/lib.rs").expect("valid").as_str(),
            "src/lib.rs"
        );
    }

    #[test]
    fn rejects_escaping_paths() {
        for bad in [
            "",
            "../evil.rs",
            "src/../../evil.rs",
            "/etc/passwd",
            "~/x.rs",
            "~root/x.rs",
        ] {
            assert!(RelPath::new(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_prefixes() {
        for bad in ["C:\\evil.rs", "C:/evil.rs", "\\\\server\\share\\x.rs"] {
            assert!(RelPath::new(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn prefixing_stays_relative() {
        let p = RelPath::new("src/l298n.rs").expect("valid");
        let dir = RelPath::new("vendor").expect("valid");
        assert_eq!(p.prefixed(&dir).as_str(), "vendor/src/l298n.rs");
    }
}
