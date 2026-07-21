//! `olivaw info <category>/<component>` — description, hardware, files,
//! Cargo.toml additions, dependencies, verification status.

use std::process::ExitCode;

use crate::registry::{ComponentId, Registry};
use crate::ui::Ui;
use crate::RegistryOpts;

pub fn run(ui: &Ui, opts: &RegistryOpts, component: &str) -> anyhow::Result<ExitCode> {
    let registry = Registry::load(opts)?;
    let id: ComponentId = component.parse()?;
    let comp = registry.component(&id)?;

    println!();
    println!(
        "  {}  {}  ·  {}",
        ui.header(&id.to_string()),
        ui.dim(&format!("v{}", comp.component.version)),
        ui.dim(&comp.component.license),
    );
    println!("  {}", comp.component.description);
    println!();

    if let Some(hw) = &comp.hardware {
        let mut line = hw.devices.join(", ");
        for extra in [&hw.interface, &hw.voltage].into_iter().flatten() {
            use std::fmt::Write;
            let _ = write!(line, " · {extra}");
        }
        println!("  {}       {line}", ui.header("Hardware"));
        if let Some(notes) = &hw.notes {
            println!("                 {notes}");
        }
    }

    if let Some(compat) = &comp.compatibility {
        let mut parts = Vec::new();
        match compat.no_std {
            Some(true) => parts.push("no_std".to_string()),
            Some(false) => parts.push("std".to_string()),
            None => {}
        }
        if let Some(hal) = &compat.embedded_hal {
            parts.push(format!("embedded-hal {hal}"));
        }
        if !compat.targets.is_empty() {
            parts.push(compat.targets.join(", "));
        }
        println!("  {}  {}", ui.header("Compatibility"), parts.join(" · "));
    }

    if let Some(verification) = &comp.verification {
        let status = if verification.verified {
            ui.ok("verified on hardware")
        } else {
            ui.warn("NOT yet verified on hardware")
        };
        println!("  {}   {status}", ui.header("Verification"));
        if let Some(reference) = &verification.reference {
            println!("                 {reference}");
        }
    }

    println!();
    println!("  {}", ui.header("Files"));
    for file in &comp.files {
        let optional = if file.optional {
            ui.dim("        (optional)")
        } else {
            String::new()
        };
        println!("    {}{optional}", file.dest);
    }

    if !comp.dependencies.cargo.is_empty() {
        println!();
        println!("  {}", ui.header("Cargo.toml additions"));
        for (name, req) in &comp.dependencies.cargo {
            println!("    {name} = \"{req}\"");
        }
    }

    println!();
    if comp.dependencies.components.is_empty() {
        println!("  Component dependencies: none");
    } else {
        println!("  {}", ui.header("Component dependencies"));
        for (path, req) in &comp.dependencies.components {
            println!("    {path} {}", ui.dim(req));
        }
    }

    println!();
    println!("  Install:  olivaw add {id}");
    Ok(ExitCode::SUCCESS)
}
