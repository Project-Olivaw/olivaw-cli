//! `olivaw list [<category>]` — every component in the registry, grouped by
//! category, marking installed ones when run inside a project.

use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

use anyhow::bail;

use crate::project::manifest::ProjectManifest;
use crate::project::Project;
use crate::registry::Registry;
use crate::suggest;
use crate::ui::Ui;
use crate::RegistryOpts;

pub fn run(ui: &Ui, opts: &RegistryOpts, category: Option<&str>) -> anyhow::Result<ExitCode> {
    let registry = Registry::load(opts)?;

    // Installed marks are best-effort: outside a project there simply are none.
    let installed: BTreeSet<String> = Project::discover()
        .ok()
        .and_then(|p| ProjectManifest::load(&p).ok().flatten())
        .map(|m| m.components.into_keys().collect())
        .unwrap_or_default();

    let mut by_category: BTreeMap<String, Vec<(String, String, String, bool)>> = BTreeMap::new();
    for id in registry.ids() {
        if let Some(entry) = registry.summary(&id) {
            by_category.entry(id.category.clone()).or_default().push((
                id.name.clone(),
                entry.version.clone(),
                entry.description.clone(),
                installed.contains(&id.to_string()),
            ));
        }
    }

    if let Some(cat) = category {
        if !by_category.contains_key(cat) {
            let cats = registry.categories();
            let msg = match suggest::did_you_mean(cat, cats.iter().map(String::as_str)) {
                Some(close) => format!(
                    "no category '{cat}' — did you mean '{close}'? \
                     Run 'olivaw list' to see all categories"
                ),
                None => format!(
                    "no category '{cat}'. Categories: {}",
                    cats.join(", ")
                ),
            };
            bail!("{msg}");
        }
        by_category.retain(|k, _| k == cat);
    }

    let name_width = by_category
        .values()
        .flatten()
        .map(|(name, ..)| name.len())
        .max()
        .unwrap_or(0)
        .max(12);

    println!();
    let mut total = 0usize;
    let mut installed_count = 0usize;
    for (cat, mut comps) in by_category {
        comps.sort();
        println!("  {}", ui.header(&cat));
        for (name, version, description, is_installed) in comps {
            total += 1;
            let mark = if is_installed {
                installed_count += 1;
                format!("   {}", ui.ok("[installed]"))
            } else {
                String::new()
            };
            println!(
                "    {name:<name_width$}  {}  {description}{mark}",
                ui.dim(&version),
            );
        }
        println!();
    }

    println!(
        "  {total} component{}, {installed_count} installed.  {}",
        if total == 1 { "" } else { "s" },
        ui.dim("Details: olivaw info <category>/<name>")
    );
    println!(
        "  {}",
        ui.dim(&format!(
            "registry v{} ({})",
            registry.version(),
            registry.source_label()
        ))
    );
    Ok(ExitCode::SUCCESS)
}
