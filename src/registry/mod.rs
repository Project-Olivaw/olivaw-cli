//! The component registry: an embedded snapshot compiled into the binary,
//! optionally superseded by a git checkout cached under `~/.olivaw/cache`.
//!
//! Load policy: `list`/`info` NEVER touch the network (cache-if-present, else
//! embedded — keeps them well under 100 ms). Only `add`/`update` may fetch.

pub mod component;
pub mod git;
pub mod index;

use std::borrow::Cow;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{bail, Context};

use crate::suggest;
use crate::ui::Ui;
use crate::RegistryOpts;
use component::Component;
use index::RegistryIndex;

static EMBEDDED_REGISTRY: include_dir::Dir<'static> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/registry");

/// `category/name`, validated: lowercase alphanumeric + hyphens, one slash.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ComponentId {
    pub category: String,
    pub name: String,
}

impl FromStr for ComponentId {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> anyhow::Result<Self> {
        let valid_part = |p: &str| {
            !p.is_empty()
                && p.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        };
        match s.split_once('/') {
            Some((category, name)) if valid_part(category) && valid_part(name) => {
                Ok(ComponentId {
                    category: category.to_string(),
                    name: name.to_string(),
                })
            }
            _ => bail!(
                "'{s}' is not a component path — expected <category>/<name>, \
                 lowercase and hyphenated (e.g. sensors/mpu6050). \
                 Run 'olivaw list' to see all components"
            ),
        }
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.category, self.name)
    }
}

/// Where registry bytes come from.
pub enum RegistrySource {
    /// Compiled into the binary with `include_dir!`.
    Embedded,
    /// A pinned-tag checkout under `~/.olivaw/cache/registry/<tag>/`.
    Cached { root: PathBuf, tag: String },
}

pub struct Registry {
    source: RegistrySource,
    index: RegistryIndex,
}

impl Registry {
    /// Cache-if-present, else embedded. Never fetches.
    pub fn load(opts: &RegistryOpts) -> anyhow::Result<Registry> {
        if let Some(cached) = git::find_valid_cache(opts)? {
            return Self::from_source(cached);
        }
        Self::from_source(RegistrySource::Embedded)
    }

    /// Try to ensure the pinned tag is cached (fetching if needed), then load.
    /// Any fetch failure degrades to `load` with a one-line note.
    pub fn load_fetching(ui: &Ui, opts: &RegistryOpts) -> anyhow::Result<Registry> {
        if opts.offline {
            return Self::load(opts);
        }
        match git::ensure_cached(ui, opts) {
            Ok(source) => Self::from_source(source),
            Err(err) => {
                println!(
                    "  {}",
                    ui.warn(&format!(
                        "note: registry fetch failed ({err:#}) — using {} registry",
                        if git::find_valid_cache(opts)?.is_some() {
                            "cached"
                        } else {
                            "embedded"
                        }
                    ))
                );
                Self::load(opts)
            }
        }
    }

    fn from_source(source: RegistrySource) -> anyhow::Result<Registry> {
        let raw = match &source {
            RegistrySource::Embedded => Cow::Borrowed(
                EMBEDDED_REGISTRY
                    .get_file("registry.toml")
                    .context("embedded registry is missing registry.toml — this is a build bug in olivaw itself")?
                    .contents(),
            ),
            RegistrySource::Cached { root, .. } => {
                Cow::Owned(std::fs::read(root.join("registry.toml")).with_context(|| {
                    format!("reading {}", root.join("registry.toml").display())
                })?)
            }
        };
        let text = std::str::from_utf8(&raw).context("registry.toml is not UTF-8")?;
        let index: RegistryIndex =
            toml::from_str(text).context("parsing registry.toml")?;
        Ok(Registry { source, index })
    }

    /// All component ids in the index, sorted (BTreeMap order).
    pub fn ids(&self) -> impl Iterator<Item = ComponentId> + '_ {
        self.index
            .components
            .keys()
            .filter_map(|k| k.parse().ok())
    }

    /// Index entry (version + description) for `list`.
    pub fn summary(&self, id: &ComponentId) -> Option<&index::IndexEntry> {
        self.index.components.get(&id.to_string())
    }

    /// Parse one component.toml lazily. Errors with a did-you-mean suggestion
    /// when the id is unknown.
    pub fn component(&self, id: &ComponentId) -> anyhow::Result<Component> {
        if self.summary(id).is_none() {
            bail!("{}", self.unknown_component_message(id));
        }
        let rel = format!("{}/{}/component.toml", id.category, id.name);
        let raw = self.read_registry_file(&rel)?;
        let text = std::str::from_utf8(&raw)
            .with_context(|| format!("{rel} is not UTF-8"))?;
        let component: Component =
            toml::from_str(text).with_context(|| format!("parsing {rel}"))?;
        // The manifest must agree with its location; a mismatch is a registry
        // authoring bug worth catching loudly.
        if component.component.name != id.name || component.component.category != id.category {
            bail!(
                "registry inconsistency: {rel} declares '{}/{}' — please report this",
                component.component.category, component.component.name
            );
        }
        Ok(component)
    }

    /// A component's payload file (`src` path from a `[[files]]` entry).
    pub fn read_file(&self, id: &ComponentId, src: &str) -> anyhow::Result<Cow<'static, [u8]>> {
        self.read_registry_file(&format!("{}/{}/{src}", id.category, id.name))
    }

    fn read_registry_file(&self, rel: &str) -> anyhow::Result<Cow<'static, [u8]>> {
        match &self.source {
            RegistrySource::Embedded => Ok(Cow::Borrowed(
                EMBEDDED_REGISTRY
                    .get_file(rel)
                    .with_context(|| {
                        format!("embedded registry is missing '{rel}' — this is a bug in the registry, please report it")
                    })?
                    .contents(),
            )),
            RegistrySource::Cached { root, .. } => {
                let path = root.join(rel);
                Ok(Cow::Owned(std::fs::read(&path).with_context(|| {
                    format!("reading {}", path.display())
                })?))
            }
        }
    }

    /// "embedded" or "cache (<tag>)" for status lines.
    pub fn source_label(&self) -> String {
        match &self.source {
            RegistrySource::Embedded => "embedded".to_string(),
            RegistrySource::Cached { tag, .. } => format!("cache ({tag})"),
        }
    }

    /// Registry format version from the index.
    pub fn version(&self) -> &str {
        &self.index.registry.version
    }

    /// All category names present in the index, deduped + sorted.
    pub fn categories(&self) -> Vec<String> {
        let mut cats: Vec<String> = self
            .ids()
            .map(|id| id.category)
            .collect();
        cats.sort();
        cats.dedup();
        cats
    }

    /// The full "no component X — did you mean Y?" message.
    pub fn unknown_component_message(&self, id: &ComponentId) -> String {
        let all: Vec<String> = self.index.components.keys().cloned().collect();
        let input = id.to_string();
        match suggest::did_you_mean(&input, all.iter().map(String::as_str)) {
            Some(close) => format!(
                "no component '{input}' — did you mean '{close}'? \
                 Run 'olivaw list {}' to see all",
                close.split('/').next().unwrap_or("")
            ),
            None => format!(
                "no component '{input}'. Run 'olivaw list' to see all components"
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_id_parses_valid_paths() {
        let id: ComponentId = "sensors/mpu6050".parse().expect("valid id");
        assert_eq!(id.category, "sensors");
        assert_eq!(id.name, "mpu6050");
        assert_eq!(id.to_string(), "sensors/mpu6050");
        let id: ComponentId = "kinematics/differential-drive".parse().expect("valid id");
        assert_eq!(id.name, "differential-drive");
    }

    #[test]
    fn component_id_rejects_bad_paths() {
        for bad in [
            "mpu6050",
            "sensors/",
            "/mpu6050",
            "Sensors/mpu6050",
            "sensors/MPU6050",
            "sensors/mpu 6050",
            "a/b/c",
            "sensors/../etc",
            "",
        ] {
            assert!(bad.parse::<ComponentId>().is_err(), "should reject {bad:?}");
        }
    }
}
