//! The shared install machinery: `add`, `init` and `update` all build a
//! `Plan` (what files to write, what Cargo.toml needs, what olivaw.toml will
//! record), present it, then execute it.
//!
//! Execution order is deliberate: component files first, then Cargo.toml,
//! then olivaw.toml *last* — the manifest only ever describes files that
//! actually exist on disk.

use std::borrow::Cow;
use std::collections::BTreeSet;

use anyhow::Context;

use crate::project::cargo::{add_missing_deps, CargoEditOutcome};
use crate::project::manifest::{InstalledComponent, ProjectManifest};
use crate::project::{Project, RelPath};
use crate::registry::ComponentId;

pub struct Plan {
    pub file_writes: Vec<FileWrite>,
    pub cargo_deps: Vec<CargoDep>,
    pub manifest_updates: Vec<(ComponentId, InstalledComponent)>,
}

pub struct FileWrite {
    pub dest: RelPath,
    pub contents: Cow<'static, [u8]>,
    pub kind: WriteKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteKind {
    Create,
    /// Only reachable via --force or a confirmed update.
    Overwrite,
}

#[derive(Debug, Clone)]
pub struct CargoDep {
    pub name: String,
    pub req: String,
    pub needed_by: ComponentId,
}

pub struct ExecReport {
    pub written: Vec<RelPath>,
    pub cargo: Option<CargoEditOutcome>,
}

impl Plan {
    pub fn new() -> Plan {
        Plan {
            file_writes: Vec::new(),
            cargo_deps: Vec::new(),
            manifest_updates: Vec::new(),
        }
    }

    /// Deduplicate cargo deps across components (same name+req collapses;
    /// conflicting reqs keep both so the report can show them).
    pub fn dedupe_cargo_deps(&mut self) {
        let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
        self.cargo_deps
            .retain(|d| seen.insert((d.name.clone(), d.req.clone())));
    }

    /// Execute against the project. `manifest` is the loaded (or fresh)
    /// olivaw.toml, updated and saved last.
    pub fn execute(
        &self,
        project: &Project,
        manifest: &mut ProjectManifest,
    ) -> anyhow::Result<ExecReport> {
        let mut written = Vec::new();
        for write in &self.file_writes {
            let abs = write.dest.join_under(&project.root);
            if let Some(parent) = abs.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("creating directory {}", parent.display()))?;
            }
            std::fs::write(&abs, &write.contents)
                .with_context(|| format!("writing {}", abs.display()))?;
            written.push(write.dest.clone());
        }

        let cargo = if self.cargo_deps.is_empty() {
            None
        } else {
            Some(add_missing_deps(project, &self.cargo_deps)?)
        };

        if !self.manifest_updates.is_empty() {
            for (id, installed) in &self.manifest_updates {
                manifest
                    .components
                    .insert(id.to_string(), installed.clone());
            }
            manifest.save(project)?;
        }

        Ok(ExecReport { written, cargo })
    }
}
