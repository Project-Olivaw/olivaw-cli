//! `olivaw add <category>/<component>` — vendor a component (and its
//! component dependencies) into the project.

use std::process::ExitCode;

use anyhow::{bail, Context};

use crate::plan::{CargoDep, FileWrite, Plan, WriteKind};
use crate::project::manifest::{now_rfc3339, InstalledComponent, ProjectManifest};
use crate::project::{Project, RelPath};
use crate::registry::Registry;
use crate::resolve::{resolve, ResolvedInstall};
use crate::ui::Ui;
use crate::RegistryOpts;

pub fn run(
    ui: &Ui,
    opts: &RegistryOpts,
    component: &str,
    path: Option<&str>,
    force: bool,
) -> anyhow::Result<ExitCode> {
    let project = Project::discover()?;
    let registry = Registry::load_fetching(ui, opts)?;
    let id = component.parse()?;

    let mut manifest = ProjectManifest::load(&project)?
        .unwrap_or_else(|| ProjectManifest::new_for(&project.package_name(), "unknown"));

    let installs = resolve(&registry, std::slice::from_ref(&id), &manifest.components)?;
    if installs.is_empty() {
        println!();
        println!(
            "  {id} is already installed. {}",
            ui.dim("Use 'olivaw update' to refresh it, or 'olivaw check' to verify it.")
        );
        return Ok(ExitCode::SUCCESS);
    }

    let extras: Vec<&ResolvedInstall> = installs.iter().filter(|i| !i.requested).collect();
    if !extras.is_empty() && !force {
        println!();
        println!("  {id} also needs:");
        for extra in &extras {
            println!(
                "    {} {}",
                extra.id,
                ui.dim(&format!("v{}", extra.component.component.version))
            );
        }
        println!();
        if ui.interactive() {
            let go = dialoguer::Confirm::new()
                .with_prompt(format!("  Install {} components?", installs.len()))
                .default(true)
                .interact()
                .context("reading confirmation")?;
            if !go {
                println!("  Nothing installed.");
                return Ok(ExitCode::SUCCESS);
            }
        } else {
            bail!(
                "{id} pulls in {} more component{} (listed above) — \
                 re-run with --force to confirm non-interactively",
                extras.len(),
                if extras.len() == 1 { "" } else { "s" }
            );
        }
    }

    let dest_prefix = path
        .map(RelPath::new)
        .transpose()
        .context("--path must stay inside the project")?;

    // Build the full plan before touching anything, so conflicts abort cleanly.
    let mut plan = Plan::new();
    let mut conflicts: Vec<RelPath> = Vec::new();
    for install in &installs {
        let mut files = Vec::new();
        let mut checksums = std::collections::BTreeMap::new();
        for file in &install.component.files {
            let dest = RelPath::new(&file.dest).with_context(|| {
                format!(
                    "component {} declares an unsafe destination — registry bug, please report it",
                    install.id
                )
            })?;
            let dest = match &dest_prefix {
                Some(prefix) => dest.prefixed(prefix),
                None => dest,
            };
            let contents = registry.read_file(&install.id, &file.src)?;
            let sha256 = crate::checksum::sha256_hex(&contents);
            let exists = dest.join_under(&project.root).exists();
            if exists && !force {
                conflicts.push(dest.clone());
            }
            files.push(dest.as_str().to_string());
            checksums.insert(dest.as_str().to_string(), sha256.clone());
            plan.file_writes.push(FileWrite {
                dest,
                contents,
                kind: if exists {
                    WriteKind::Overwrite
                } else {
                    WriteKind::Create
                },
            });
        }
        for (name, req) in &install.component.dependencies.cargo {
            plan.cargo_deps.push(CargoDep {
                name: name.clone(),
                req: req.clone(),
                needed_by: install.id.clone(),
            });
        }
        plan.manifest_updates.push((
            install.id.clone(),
            InstalledComponent {
                version: install.component.component.version.clone(),
                installed_at: now_rfc3339(),
                files,
                checksums,
            },
        ));
    }
    plan.dedupe_cargo_deps();

    if !conflicts.is_empty() {
        let listing: Vec<String> = conflicts.iter().map(|c| format!("  {c}")).collect();
        bail!(
            "these files already exist:\n{}\n\
             re-run with --force to overwrite them, or use \
             'olivaw update {id}' if the component is already installed",
            listing.join("\n")
        );
    }

    let report = plan.execute(&project, &mut manifest)?;

    // ---- report ------------------------------------------------------------
    println!();
    for install in &installs {
        println!(
            "  {} {}",
            ui.ok(&format!("Added {}", install.id)),
            ui.dim(&format!("(v{})", install.component.component.version))
        );
    }
    println!();
    for written in &report.written {
        println!("    {written}");
    }

    if let Some(cargo) = &report.cargo {
        if !cargo.added.is_empty() {
            println!();
            println!("  {}", ui.header("Updated Cargo.toml:"));
            println!();
            for dep in &cargo.added {
                println!("    {} {} = \"{}\"", ui.ok("+"), dep.name, dep.req);
            }
        }
        for (dep, existing) in &cargo.skipped {
            println!();
            println!(
                "  {}",
                ui.warn(&format!(
                    "note: {} already in Cargo.toml as {existing} — component {} expects \"{}\"; left untouched",
                    dep.name, dep.needed_by, dep.req
                ))
            );
        }
    }

    for install in &installs {
        if let Some(hw) = &install.component.hardware {
            if let Some(notes) = &hw.notes {
                println!();
                println!("  Wiring: {notes}");
            }
        }
        if let Some(verification) = &install.component.verification {
            if !verification.verified {
                println!();
                println!(
                    "  {}",
                    ui.warn(&format!(
                        "{} has not been verified on hardware yet — review before trusting it on a robot",
                        install.id
                    ))
                );
            }
        }
    }

    // "Try it" for the first example file that landed.
    if let Some(example) = report
        .written
        .iter()
        .find(|w| w.as_str().starts_with("examples/"))
    {
        let name = example
            .as_str()
            .trim_start_matches("examples/")
            .trim_end_matches(".rs");
        println!();
        println!("  Try it:  cargo run --example {name}");
    }

    println!();
    Ok(ExitCode::SUCCESS)
}
