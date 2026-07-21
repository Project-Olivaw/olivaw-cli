//! `olivaw.toml` — the project manifest recording what is installed and the
//! sha256 of every vendored file at install time. This file is machine-owned
//! (unlike the user's Cargo.toml, which gets format-preserving edits).

use std::collections::BTreeMap;

use anyhow::Context;

use super::Project;

const HEADER: &str = "# Managed by olivaw — records installed components and their checksums.\n\
                      # Edit at your own risk; 'olivaw check' verifies against this file.\n\n";

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectManifest {
    pub project: ProjectMeta,
    /// Keyed by component path, e.g. `"sensors/mpu6050"`.
    #[serde(default)]
    pub components: BTreeMap<String, InstalledComponent>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ProjectMeta {
    pub name: String,
    pub target: String,
    pub olivaw_version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct InstalledComponent {
    pub version: String,
    /// RFC 3339 UTC timestamp.
    pub installed_at: String,
    /// Project-relative paths, in install order.
    pub files: Vec<String>,
    /// file → `"sha256:<hex>"` of the ORIGINAL vendored content. This is what
    /// makes `update` safe: a differing disk hash means the user edited the
    /// file, and it must never be overwritten silently.
    #[serde(default)]
    pub checksums: BTreeMap<String, String>,
}

impl ProjectManifest {
    /// `Ok(None)` when the project has no olivaw.toml yet.
    pub fn load(project: &Project) -> anyhow::Result<Option<ProjectManifest>> {
        let path = project.manifest_path();
        if !path.is_file() {
            return Ok(None);
        }
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("reading {}", path.display()))?;
        let manifest = toml::from_str(&text)
            .with_context(|| format!("parsing {} — if you edited it by hand, fix the syntax or restore it from version control", path.display()))?;
        Ok(Some(manifest))
    }

    /// A manifest for a project that predates olivaw (created by `add` on
    /// first use in a hand-made project).
    pub fn new_for(project_name: &str, target: &str) -> ProjectManifest {
        ProjectManifest {
            project: ProjectMeta {
                name: project_name.to_string(),
                target: target.to_string(),
                olivaw_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            components: BTreeMap::new(),
        }
    }

    pub fn save(&self, project: &Project) -> anyhow::Result<()> {
        let path = project.manifest_path();
        let body = toml::to_string_pretty(self).context("serializing olivaw.toml")?;
        std::fs::write(&path, format!("{HEADER}{body}"))
            .with_context(|| format!("writing {}", path.display()))?;
        Ok(())
    }
}

/// Now, as RFC 3339 UTC (e.g. `2026-07-20T21:30:00Z`). Hand-rolled from
/// `SystemTime` — not worth a chrono dependency for one timestamp.
#[allow(clippy::many_single_char_names)]
pub fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_secs());
    let days = secs / 86_400;
    let (h, m, s) = {
        let t = secs % 86_400;
        (t / 3600, (t % 3600) / 60, t % 60)
    };
    // Civil-from-days (Howard Hinnant's algorithm), valid for all dates we care about.
    let z = i64::try_from(days).unwrap_or(0) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mth = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mth <= 2 { y + 1 } else { y };
    format!("{y:04}-{mth:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_round_trips() {
        let mut m = ProjectManifest::new_for("my-robot", "esp32");
        m.components.insert(
            "sensors/mpu6050".to_string(),
            InstalledComponent {
                version: "0.1.0".to_string(),
                installed_at: "2026-07-20T10:30:00Z".to_string(),
                files: vec!["src/sensors/mpu6050.rs".to_string()],
                checksums: BTreeMap::from([(
                    "src/sensors/mpu6050.rs".to_string(),
                    "sha256:abc".to_string(),
                )]),
            },
        );
        let text = toml::to_string_pretty(&m).expect("serializes");
        let back: ProjectManifest = toml::from_str(&text).expect("parses");
        assert_eq!(back.project.name, "my-robot");
        let comp = back.components.get("sensors/mpu6050").expect("present");
        assert_eq!(comp.checksums.len(), 1);
    }

    #[test]
    fn rfc3339_format_shape() {
        let t = now_rfc3339();
        // 2026-07-20T21:30:00Z — 20 chars, fixed punctuation.
        assert_eq!(t.len(), 20, "unexpected shape: {t}");
        assert_eq!(&t[4..5], "-");
        assert_eq!(&t[10..11], "T");
        assert!(t.ends_with('Z'));
        assert!(t.starts_with("20"), "unexpected year: {t}");
    }
}
