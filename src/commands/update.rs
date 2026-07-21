//! `olivaw update <category>/<component>` — re-fetch a component, diffing
//! against local state. This is the trust-critical command: silently
//! destroying the user's edits is the one unforgivable bug, so a file whose
//! disk hash differs from the recorded install-time hash is only ever
//! overwritten after an explicit confirmation (default No) or --force.

use std::collections::BTreeMap;
use std::process::ExitCode;

use anyhow::{bail, Context};

use crate::checksum;
use crate::plan::{CargoDep, FileWrite, Plan, WriteKind};
use crate::project::manifest::{now_rfc3339, InstalledComponent, ProjectManifest};
use crate::project::{Project, RelPath};
use crate::registry::Registry;
use crate::ui::Ui;
use crate::RegistryOpts;

enum FileAction {
    /// Disk already matches incoming content.
    UpToDate,
    /// Disk matches the recorded hash; upstream changed the file.
    Upstream { new_len_delta: (usize, usize) },
    /// Disk differs from the recorded hash: the user edited this file.
    UserModified { old: Vec<u8> },
    /// Recorded file is gone from disk; will be restored.
    Restore,
    /// New file introduced by the new version.
    New,
}

pub fn run(
    ui: &Ui,
    opts: &RegistryOpts,
    component: &str,
    force: bool,
    dry_run: bool,
) -> anyhow::Result<ExitCode> {
    let project = Project::discover()?;
    let id: crate::registry::ComponentId = component.parse()?;
    let mut manifest = ProjectManifest::load(&project)?
        .with_context(|| format!("no olivaw.toml here — nothing is installed yet. Use 'olivaw add {id}'"))?;
    let Some(installed) = manifest.components.get(&id.to_string()).cloned() else {
        bail!(
            "{id} is not installed in this project — install it with 'olivaw add {id}'"
        );
    };

    let registry = Registry::load_fetching(ui, opts)?;
    let comp = registry.component(&id)?;

    // Work out per-file actions before touching anything.
    let mut actions: Vec<(RelPath, Vec<u8>, String, FileAction)> = Vec::new();
    let mut dropped: Vec<String> = Vec::new();
    let mut incoming_dests: Vec<String> = Vec::new();

    for file in &comp.files {
        let dest = RelPath::new(&file.dest).with_context(|| {
            format!("component {id} declares an unsafe destination — registry bug, please report it")
        })?;
        incoming_dests.push(dest.as_str().to_string());
        let incoming = registry.read_file(&id, &file.src)?.into_owned();
        let incoming_sha = checksum::sha256_hex(&incoming);
        let abs = dest.join_under(&project.root);
        let action = match std::fs::read(&abs) {
            Err(_) => {
                if installed.files.contains(&dest.as_str().to_string()) {
                    FileAction::Restore
                } else {
                    FileAction::New
                }
            }
            Ok(disk) => {
                let disk_sha = checksum::sha256_hex(&disk);
                if disk_sha == incoming_sha {
                    FileAction::UpToDate
                } else {
                    let recorded = installed.checksums.get(dest.as_str());
                    if recorded == Some(&disk_sha) {
                        let old_lines = disk.iter().filter(|b| **b == b'\n').count();
                        let new_lines = incoming.iter().filter(|b| **b == b'\n').count();
                        FileAction::Upstream {
                            new_len_delta: (new_lines.saturating_sub(old_lines), old_lines.saturating_sub(new_lines)),
                        }
                    } else {
                        // No recorded hash counts as user-modified too: when in
                        // doubt, protect the local file.
                        FileAction::UserModified { old: disk }
                    }
                }
            }
        };
        actions.push((dest, incoming, incoming_sha, action));
    }

    // Files recorded at install time but absent from the new version: report,
    // never delete — the user owns their tree.
    for old_file in &installed.files {
        if !incoming_dests.contains(old_file) {
            dropped.push(old_file.clone());
        }
    }

    let pending: Vec<&(RelPath, Vec<u8>, String, FileAction)> = actions
        .iter()
        .filter(|(_, _, _, a)| !matches!(a, FileAction::UpToDate))
        .collect();

    if pending.is_empty() && dropped.is_empty() {
        println!();
        println!(
            "  {id} is up to date {}",
            ui.dim(&format!("(v{})", comp.component.version))
        );
        println!();
        return Ok(ExitCode::SUCCESS);
    }

    // ---- present -----------------------------------------------------------
    println!();
    let version_line = if installed.version == comp.component.version {
        format!("v{}", comp.component.version)
    } else {
        format!("{} → {}", installed.version, comp.component.version)
    };
    println!("  {}  {}", ui.header(&id.to_string()), version_line);
    println!();

    let mut modified_count = 0usize;
    for (dest, incoming, _, action) in &actions {
        match action {
            FileAction::UpToDate => {}
            FileAction::New => println!("  {dest}   {}", ui.ok("new file")),
            FileAction::Restore => println!("  {dest}   {}", ui.warn("missing — will restore")),
            FileAction::Upstream { new_len_delta, .. } => {
                println!(
                    "  {dest}   upstream change {}",
                    ui.dim(&format!("(+{} −{})", new_len_delta.0, new_len_delta.1))
                );
            }
            FileAction::UserModified { old } => {
                modified_count += 1;
                println!(
                    "  {dest}   {} — local vs incoming:",
                    ui.warn("modified by you")
                );
                println!();
                let old_text = String::from_utf8_lossy(old);
                let new_text = String::from_utf8_lossy(incoming);
                ui.print_diff(&old_text, &new_text);
                println!();
            }
        }
    }
    for dropped_file in &dropped {
        println!(
            "  {dropped_file}   {}",
            ui.dim("no longer part of this component — left in place, remove it yourself if unwanted")
        );
    }

    if dry_run {
        println!();
        println!("  {}", ui.dim("--dry-run: nothing written."));
        println!();
        return Ok(ExitCode::SUCCESS);
    }

    // ---- confirm -----------------------------------------------------------
    if pending.is_empty() {
        // Only dropped-file notes; manifest still needs refreshing below.
    } else if modified_count > 0 && !force {
        if !ui.interactive() {
            bail!(
                "refusing to overwrite {modified_count} locally modified file{} non-interactively — \
                 review the diff above and re-run with --force",
                if modified_count == 1 { "" } else { "s" }
            );
        }
        let go = dialoguer::Confirm::new()
            .with_prompt(format!(
                "  Overwrite {modified_count} locally modified file{}?",
                if modified_count == 1 { "" } else { "s" }
            ))
            .default(false)
            .interact()
            .context("reading confirmation")?;
        if !go {
            println!("  Nothing written. Your edits are untouched.");
            return Ok(ExitCode::SUCCESS);
        }
    } else if !force && ui.interactive() {
        let go = dialoguer::Confirm::new()
            .with_prompt("  Apply update?")
            .default(true)
            .interact()
            .context("reading confirmation")?;
        if !go {
            println!("  Nothing written.");
            return Ok(ExitCode::SUCCESS);
        }
    }

    // ---- execute -----------------------------------------------------------
    let mut plan = Plan::new();
    let mut files = Vec::new();
    let mut checksums = BTreeMap::new();
    for (dest, incoming, sha, action) in actions {
        files.push(dest.as_str().to_string());
        checksums.insert(dest.as_str().to_string(), sha.clone());
        if matches!(action, FileAction::UpToDate) {
            continue;
        }
        plan.file_writes.push(FileWrite {
            dest,
            contents: std::borrow::Cow::Owned(incoming),
            kind: WriteKind::Overwrite,
        });
    }
    for (name, req) in &comp.dependencies.cargo {
        plan.cargo_deps.push(CargoDep {
            name: name.clone(),
            req: req.clone(),
            needed_by: id.clone(),
        });
    }
    plan.manifest_updates.push((
        id.clone(),
        InstalledComponent {
            version: comp.component.version.clone(),
            installed_at: now_rfc3339(),
            files,
            checksums,
        },
    ));
    let report = plan.execute(&project, &mut manifest)?;

    println!();
    println!(
        "  {} {}",
        ui.ok(&format!("Updated {id}")),
        ui.dim(&format!("(v{})", comp.component.version))
    );
    for written in &report.written {
        println!("    {written}");
    }
    if let Some(cargo) = &report.cargo {
        for dep in &cargo.added {
            println!("    {} {} = \"{}\" added to Cargo.toml", ui.ok("+"), dep.name, dep.req);
        }
    }
    println!();
    Ok(ExitCode::SUCCESS)
}
