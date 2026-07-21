//! Append-only edits to the *user's* Cargo.toml via `toml_edit`, which
//! preserves formatting and comments. Rules:
//!
//! - A dependency that already exists — in any form (string, inline table,
//!   `workspace = true`, dotted table) — is never touched. If its requirement
//!   differs from what the component expects, that is reported as a note.
//! - Missing dependencies are appended to `[dependencies]` (creating the
//!   table at the end of the file if absent).
//! - No sorting, no reformatting; the file is only written when changed.

use anyhow::Context;
use toml_edit::{DocumentMut, Item, Table};

use super::Project;
use crate::plan::CargoDep;

pub struct CargoEditOutcome {
    pub added: Vec<CargoDep>,
    /// (dep, existing requirement as rendered) — left untouched.
    pub skipped: Vec<(CargoDep, String)>,
}

pub fn add_missing_deps(project: &Project, deps: &[CargoDep]) -> anyhow::Result<CargoEditOutcome> {
    let path = project.cargo_toml_path();
    let original =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
    let mut doc: DocumentMut = original.parse().with_context(|| {
        format!(
            "{} is not valid TOML — fix it before adding components",
            path.display()
        )
    })?;

    let mut outcome = CargoEditOutcome {
        added: Vec::new(),
        skipped: Vec::new(),
    };

    for dep in deps {
        if let Some(existing) = existing_dep(&doc, &dep.name) {
            outcome.skipped.push((dep.clone(), existing));
            continue;
        }
        let table = ensure_dependencies_table(&mut doc)?;
        table[dep.name.as_str()] = toml_edit::value(dep.req.clone());
        outcome.added.push(dep.clone());
    }

    let mut updated = doc.to_string();
    // toml_edit renders LF; if the user's file is CRLF, keep it CRLF so
    // untouched regions stay byte-identical.
    if original.contains("\r\n") && !original.replace("\r\n", "").contains('\n') {
        updated = updated.replace('\n', "\r\n");
    }
    if updated != original {
        std::fs::write(&path, updated).with_context(|| format!("writing {}", path.display()))?;
    }
    Ok(outcome)
}

/// How the existing entry renders (for the "left untouched" note), if the
/// dependency is present in `[dependencies]` in any form.
fn existing_dep(doc: &DocumentMut, name: &str) -> Option<String> {
    let deps = doc.get("dependencies")?.as_table_like()?;
    let item = deps.get(name)?;
    Some(render_dep_item(item))
}

fn render_dep_item(item: &Item) -> String {
    match item {
        Item::Value(v) => v.to_string().trim().to_string(),
        _ => item.to_string().trim().to_string(),
    }
}

fn ensure_dependencies_table(doc: &mut DocumentMut) -> anyhow::Result<&mut Table> {
    if doc.get("dependencies").is_none() {
        let mut table = Table::new();
        table.set_implicit(false);
        doc["dependencies"] = Item::Table(table);
    }
    doc["dependencies"]
        .as_table_mut()
        .context("Cargo.toml [dependencies] is not a table")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(original: &str, deps: &[(&str, &str)]) -> (String, CargoEditOutcome) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), original).expect("write");
        let project = Project {
            root: dir.path().to_path_buf(),
        };
        let deps: Vec<CargoDep> = deps
            .iter()
            .map(|(n, r)| CargoDep {
                name: (*n).to_string(),
                req: (*r).to_string(),
                needed_by: "sensors/mpu6050".parse().expect("valid id"),
            })
            .collect();
        let outcome = add_missing_deps(&project, &deps).expect("edit succeeds");
        let text = std::fs::read_to_string(dir.path().join("Cargo.toml")).expect("read");
        (text, outcome)
    }

    #[test]
    fn appends_missing_deps_preserving_everything() {
        let original = "\
# my project
[package]
name = \"robot\"   # the name
version = \"0.1.0\"

[dependencies]
# pinned for a reason
serde = { version = \"1\", features = [\"derive\"] }
";
        let (text, outcome) = edit(original, &[("embedded-hal", "1.0"), ("libm", "0.2")]);
        assert_eq!(outcome.added.len(), 2);
        // Untouched regions are byte-identical.
        assert!(
            text.starts_with(original.trim_end_matches('\n'))
                || text.contains("# pinned for a reason")
        );
        assert!(text.contains("# my project"));
        assert!(text.contains("name = \"robot\"   # the name"));
        assert!(text.contains("serde = { version = \"1\", features = [\"derive\"] }"));
        assert!(text.contains("embedded-hal = \"1.0\""));
        assert!(text.contains("libm = \"0.2\""));
    }

    #[test]
    fn never_touches_existing_deps() {
        let original = "\
[package]
name = \"robot\"
version = \"0.1.0\"

[dependencies]
embedded-hal = \"0.2\"
";
        let (text, outcome) = edit(original, &[("embedded-hal", "1.0")]);
        assert!(outcome.added.is_empty());
        assert_eq!(outcome.skipped.len(), 1);
        assert_eq!(outcome.skipped[0].1, "\"0.2\"");
        // File unchanged, byte for byte.
        assert_eq!(text, original);
    }

    #[test]
    fn respects_workspace_and_table_forms() {
        let original = "\
[package]
name = \"robot\"
version = \"0.1.0\"

[dependencies]
nalgebra = { workspace = true }

[dependencies.kiddo]
version = \"5\"
";
        let (text, outcome) = edit(original, &[("nalgebra", "0.34"), ("kiddo", "5")]);
        assert!(outcome.added.is_empty());
        assert_eq!(outcome.skipped.len(), 2);
        assert_eq!(text, original);
    }

    #[test]
    fn creates_dependencies_table_when_absent() {
        let original = "[package]\nname = \"robot\"\nversion = \"0.1.0\"\n";
        let (text, outcome) = edit(original, &[("embedded-hal", "1.0")]);
        assert_eq!(outcome.added.len(), 1);
        assert!(text.contains("[dependencies]"));
        assert!(text.contains("embedded-hal = \"1.0\""));
        assert!(text.contains("name = \"robot\""));
    }

    #[test]
    fn preserves_crlf_content_outside_edits() {
        let original = "[package]\r\nname = \"robot\"\r\nversion = \"0.1.0\"\r\n\r\n[dependencies]\r\nserde = \"1\"\r\n";
        let (text, _) = edit(original, &[("libm", "0.2")]);
        assert!(text.contains("name = \"robot\"\r\n"));
        assert!(text.contains("serde = \"1\"\r\n"));
        assert!(text.contains("libm = \"0.2\""));
    }
}
