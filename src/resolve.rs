//! Recursive component-dependency resolution: DFS post-order gives a
//! dependencies-first install order; an on-stack set detects cycles and the
//! error names the cycle.

use std::collections::{BTreeMap, BTreeSet};

use anyhow::{bail, Context};

use crate::project::manifest::InstalledComponent;
use crate::registry::component::Component;
use crate::registry::{ComponentId, Registry};

pub struct ResolvedInstall {
    pub id: ComponentId,
    pub component: Component,
    /// Requested explicitly (vs pulled in as a dependency).
    pub requested: bool,
}

/// Resolve `roots` and their transitive component dependencies into install
/// order. Components already recorded in `installed` are skipped (not
/// re-vendored) but still traversed so their deps are checked.
pub fn resolve(
    registry: &Registry,
    roots: &[ComponentId],
    installed: &BTreeMap<String, InstalledComponent>,
) -> anyhow::Result<Vec<ResolvedInstall>> {
    let mut order: Vec<ResolvedInstall> = Vec::new();
    let mut done: BTreeSet<ComponentId> = BTreeSet::new();
    let mut stack: Vec<ComponentId> = Vec::new();

    for root in roots {
        visit(registry, root, true, installed, &mut done, &mut stack, &mut order)?;
    }
    Ok(order)
}

fn visit(
    registry: &Registry,
    id: &ComponentId,
    requested: bool,
    installed: &BTreeMap<String, InstalledComponent>,
    done: &mut BTreeSet<ComponentId>,
    stack: &mut Vec<ComponentId>,
    order: &mut Vec<ResolvedInstall>,
) -> anyhow::Result<()> {
    if done.contains(id) {
        return Ok(());
    }
    if let Some(pos) = stack.iter().position(|s| s == id) {
        let mut cycle: Vec<String> = stack[pos..].iter().map(ToString::to_string).collect();
        cycle.push(id.to_string());
        bail!(
            "dependency cycle in the registry: {} — this is a registry bug, please report it",
            cycle.join(" → ")
        );
    }

    let component = registry
        .component(id)
        .with_context(|| format!("resolving {id}"))?;

    stack.push(id.clone());
    for dep_path in component.dependencies.components.keys() {
        let dep_id: ComponentId = dep_path.parse().with_context(|| {
            format!("component {id} declares invalid dependency '{dep_path}'")
        })?;
        visit(registry, &dep_id, false, installed, done, stack, order)?;
    }
    stack.pop();

    done.insert(id.clone());
    if !installed.contains_key(&id.to_string()) {
        order.push(ResolvedInstall {
            id: id.clone(),
            component,
            requested,
        });
    }
    Ok(())
}
