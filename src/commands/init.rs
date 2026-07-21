//! `olivaw init` — scaffold a new robotics project that builds and flashes
//! immediately. A broken scaffold is worse than no scaffold.

use std::process::ExitCode;

use anyhow::{bail, Context};

use crate::plan::WriteKind;
use crate::project::manifest::ProjectManifest;
use crate::project::Project;
use crate::templates::{scaffold_plan, Target};
use crate::ui::Ui;

pub fn run(ui: &Ui, name: Option<&str>, target: Target, force: bool) -> anyhow::Result<ExitCode> {
    let cwd = std::env::current_dir().context("reading current directory")?;
    let (root, project_name) = match name {
        Some(name) => {
            if name.is_empty()
                || !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                bail!("'{name}' is not a valid project name — use letters, digits, '-' and '_'");
            }
            (cwd.join(name), name.to_string())
        }
        None => {
            let dir_name = cwd
                .file_name()
                .and_then(|n| n.to_str())
                .context("current directory has no usable name — use --name")?
                .to_string();
            (cwd.clone(), dir_name)
        }
    };

    let mut plan = scaffold_plan(&project_name, target)?;

    // Conflict scan before any write.
    let mut conflicts = Vec::new();
    for write in &mut plan.file_writes {
        if write.dest.join_under(&root).exists() {
            if force {
                write.kind = WriteKind::Overwrite;
            } else {
                conflicts.push(write.dest.clone());
            }
        }
    }
    if !conflicts.is_empty() {
        let listing: Vec<String> = conflicts.iter().map(|c| format!("  {c}")).collect();
        bail!(
            "these files already exist:\n{}\nre-run with --force to overwrite them",
            listing.join("\n")
        );
    }

    std::fs::create_dir_all(&root).with_context(|| format!("creating {}", root.display()))?;
    let project = Project { root: root.clone() };
    let mut manifest = ProjectManifest::new_for(&project_name, target.as_str());
    let report = plan.execute(&project, &mut manifest)?;
    // Scaffolds carry no [[components]], so save the manifest explicitly.
    manifest.save(&project)?;

    println!();
    println!(
        "  {} {}",
        ui.ok(&format!("Created {project_name}/")),
        ui.dim(&format!("(target: {})", target.as_str()))
    );
    println!();
    for written in &report.written {
        println!("    {written}");
    }
    println!("    olivaw.toml");
    println!();
    println!("  {}", ui.header("Next:"));
    if name.is_some() {
        println!("    cd {project_name}");
    }
    match target {
        Target::Esp32 => {
            println!(
                "    espup install                   {}",
                ui.dim("(once — installs the Xtensa Rust toolchain)")
            );
            println!("    cargo install espflash          {}", ui.dim("(once)"));
            println!(
                "    cargo run                       {}",
                ui.dim("→ flashes and blinks the onboard LED (GPIO2)")
            );
        }
        Target::Rp2040 => {
            println!("    cargo install elf2uf2-rs        {}", ui.dim("(once)"));
            println!(
                "    hold BOOTSEL, plug in the Pico, then:  cargo run   {}",
                ui.dim("→ onboard LED blinks")
            );
        }
        Target::Linux => {
            println!(
                "    cargo run                       {}",
                ui.dim("→ prints a simulated blinky")
            );
        }
    }
    println!();
    println!("  Add your first component:  olivaw add sensors/mpu6050");
    println!();
    Ok(ExitCode::SUCCESS)
}
