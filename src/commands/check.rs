//! `olivaw check` — verify installed components against olivaw.toml. Pure
//! read-only. Exit codes: 0 clean, 1 drift found, 2 could-not-run (via the
//! error path in main).

use std::process::ExitCode;

use crate::checksum;
use crate::project::manifest::{InstalledComponent, ProjectManifest};
use crate::project::{Project, RelPath};
use crate::ui::Ui;

/// Drift state of one vendored file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileState {
    Clean,
    /// Disk content differs from the recorded install-time hash: the user
    /// (or something else) edited it.
    Modified,
    Missing,
    /// No recorded checksum (manifest predates it or was hand-edited).
    Unknown,
}

/// Compare every recorded file's on-disk hash with the recorded one.
pub fn scan(project: &Project, installed: &InstalledComponent) -> Vec<(String, FileState)> {
    installed
        .files
        .iter()
        .map(|file| {
            let state = match RelPath::new(file) {
                Err(_) => FileState::Unknown,
                Ok(rel) => {
                    let abs = rel.join_under(&project.root);
                    match std::fs::read(&abs) {
                        Err(_) => FileState::Missing,
                        Ok(bytes) => match installed.checksums.get(file) {
                            None => FileState::Unknown,
                            Some(recorded) => {
                                if checksum::sha256_hex(&bytes) == *recorded {
                                    FileState::Clean
                                } else {
                                    FileState::Modified
                                }
                            }
                        },
                    }
                }
            };
            (file.clone(), state)
        })
        .collect()
}

pub fn run(ui: &Ui, quiet: bool) -> anyhow::Result<ExitCode> {
    let project = Project::discover()?;
    let Some(manifest) = ProjectManifest::load(&project)? else {
        if !quiet {
            println!();
            println!(
                "  No olivaw.toml here — nothing to check. {}",
                ui.dim("Components installed with 'olivaw add' are tracked automatically.")
            );
        }
        return Ok(ExitCode::SUCCESS);
    };

    let mut issues = 0usize;
    let mut lines: Vec<String> = Vec::new();
    let mut fixes: Vec<String> = Vec::new();

    for (path, installed) in &manifest.components {
        lines.push(format!(
            "  {}  {}",
            ui.header(path),
            ui.dim(&format!("v{}", installed.version))
        ));
        let mut component_modified = false;
        let mut component_missing = false;
        for (file, state) in scan(&project, installed) {
            let label = match state {
                FileState::Clean => ui.ok("ok"),
                FileState::Modified => {
                    issues += 1;
                    component_modified = true;
                    ui.warn("modified")
                }
                FileState::Missing => {
                    issues += 1;
                    component_missing = true;
                    ui.warn("missing")
                }
                FileState::Unknown => {
                    issues += 1;
                    ui.warn("no recorded checksum")
                }
            };
            lines.push(format!("    {file:<40} {label}"));
        }
        if component_modified {
            fixes.push(format!(
                "modified → olivaw update {path}   (shows a diff first; your edits are safe)"
            ));
        }
        if component_missing {
            fixes.push(format!("missing  → olivaw add {path} --force"));
        }
    }

    // Missing cargo deps: every component's declared crates must be present.
    let cargo_toml = std::fs::read_to_string(project.cargo_toml_path()).unwrap_or_default();
    let cargo_doc: Option<toml_edit::DocumentMut> = cargo_toml.parse().ok();
    let registry = crate::registry::Registry::load(&crate::RegistryOpts {
        offline: true,
        tag_override: None,
    })
    .ok();
    if let (Some(doc), Some(registry)) = (&cargo_doc, &registry) {
        let mut dep_lines = Vec::new();
        for path in manifest.components.keys() {
            // Re-read requirements from the registry when available; skip
            // silently if the component is unknown to this CLI build.
            let Ok(id) = path.parse::<crate::registry::ComponentId>() else {
                continue;
            };
            let Ok(component) = registry.component(&id) else {
                continue;
            };
            for (name, req) in &component.dependencies.cargo {
                let present = doc
                    .get("dependencies")
                    .and_then(toml_edit::Item::as_table_like)
                    .is_some_and(|t| t.get(name).is_some());
                if !present {
                    issues += 1;
                    dep_lines.push(format!(
                        "    {:<40} {}   {}",
                        format!("{name} = \"{req}\""),
                        ui.warn("missing"),
                        ui.dim(&format!("(needed by {path})"))
                    ));
                }
            }
        }
        if !dep_lines.is_empty() {
            lines.push(format!("  {}", ui.header("Cargo.toml")));
            lines.extend(dep_lines);
            fixes.push("missing deps → add the lines above to [dependencies]".to_string());
        }
    }

    if issues == 0 {
        if !quiet {
            println!();
            println!(
                "  {} {} component{} clean.",
                ui.ok("✓"),
                manifest.components.len(),
                if manifest.components.len() == 1 {
                    ""
                } else {
                    "s"
                }
            );
            println!();
        }
        return Ok(ExitCode::SUCCESS);
    }

    println!();
    for line in lines {
        println!("{line}");
    }
    println!();
    println!(
        "  {} issue{} found.",
        issues,
        if issues == 1 { "" } else { "s" }
    );
    fixes.sort();
    fixes.dedup();
    for fix in fixes {
        println!("    {fix}");
    }
    println!();
    Ok(ExitCode::FAILURE)
}
