//! `registry.toml` — the index. Deliberately tiny: `list` parses only this,
//! which keeps it microseconds even as the registry grows.

use std::collections::BTreeMap;

#[derive(Debug, serde::Deserialize)]
pub struct RegistryIndex {
    pub registry: RegistryMeta,
    /// Keyed by full component path, e.g. `"sensors/mpu6050"`.
    #[serde(default)]
    pub components: BTreeMap<String, IndexEntry>,
}

#[derive(Debug, serde::Deserialize)]
pub struct RegistryMeta {
    /// Registry format version (independent of the CLI version).
    pub version: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct IndexEntry {
    pub version: String,
    pub description: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_index() {
        let toml = r#"
            [registry]
            version = "0.1.0"

            [components."sensors/mpu6050"]
            version = "0.1.0"
            description = "6-axis IMU"

            [components."drivers/l298n"]
            version = "0.1.0"
            description = "Dual H-bridge"
        "#;
        let index: RegistryIndex = toml::from_str(toml).expect("parses");
        assert_eq!(index.registry.version, "0.1.0");
        assert_eq!(index.components.len(), 2);
        // BTreeMap iterates sorted: drivers before sensors.
        let first = index.components.keys().next().expect("nonempty");
        assert_eq!(first, "drivers/l298n");
    }
}
