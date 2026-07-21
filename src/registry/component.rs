//! `component.toml` — per-component metadata and file manifest. Schema per
//! CLAUDE.md, plus the `[verification]` table recording honest hardware
//! status (a documented schema addition).

use std::collections::BTreeMap;

#[derive(Debug, serde::Deserialize)]
pub struct Component {
    pub component: ComponentMeta,
    pub hardware: Option<Hardware>,
    pub compatibility: Option<Compatibility>,
    pub verification: Option<Verification>,
    #[serde(default)]
    pub files: Vec<FileEntry>,
    #[serde(default)]
    pub dependencies: Dependencies,
}

#[derive(Debug, serde::Deserialize)]
pub struct ComponentMeta {
    pub name: String,
    pub category: String,
    pub version: String,
    pub description: String,
    pub license: String,
}

#[derive(Debug, serde::Deserialize)]
pub struct Hardware {
    #[serde(default)]
    pub devices: Vec<String>,
    pub interface: Option<String>,
    pub voltage: Option<String>,
    pub notes: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Compatibility {
    pub no_std: Option<bool>,
    pub embedded_hal: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,
}

/// Honest hardware-verification status. Ports of reference firmware start
/// `verified = false` until someone flashes them on a real board.
#[derive(Debug, serde::Deserialize)]
pub struct Verification {
    pub verified: bool,
    /// Board it was verified on, or the reference implementation it ports.
    pub reference: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct FileEntry {
    /// Path inside the component's registry directory.
    pub src: String,
    /// Destination relative to the user's project root. Treated as untrusted
    /// data — validated through `RelPath` before any write.
    pub dest: String,
    #[serde(default)]
    pub optional: bool,
}

#[derive(Debug, Default, serde::Deserialize)]
pub struct Dependencies {
    /// Crates the user must have in Cargo.toml, name → version requirement.
    #[serde(default)]
    pub cargo: BTreeMap<String, String>,
    /// Other olivaw components this one needs, path → version requirement.
    #[serde(default)]
    pub components: BTreeMap<String, String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_component_toml() {
        let toml = r#"
            [component]
            name = "mpu6050"
            category = "sensors"
            version = "0.1.0"
            description = "6-axis IMU driver"
            license = "MIT OR Apache-2.0"

            [hardware]
            devices = ["MPU-6050", "MPU-6500"]
            interface = "i2c"
            voltage = "3.3V or 5V"
            notes = "Default address 0x68"

            [compatibility]
            no_std = true
            embedded_hal = "1.0"
            targets = ["esp32", "rp2040"]

            [verification]
            verified = false
            reference = "MPU-6050 datasheet rev 3.4"

            [[files]]
            src = "src/mpu6050.rs"
            dest = "src/sensors/mpu6050.rs"

            [[files]]
            src = "example.rs"
            dest = "examples/mpu6050_read.rs"
            optional = true

            [dependencies.cargo]
            "embedded-hal" = "1.0"
            "libm" = "0.2"

            [dependencies.components]
            "slam/core-types" = "0.1"
        "#;
        let c: Component = toml::from_str(toml).expect("parses");
        assert_eq!(
            format!("{}/{}", c.component.category, c.component.name),
            "sensors/mpu6050"
        );
        assert_eq!(c.files.len(), 2);
        assert!(c.files[1].optional);
        assert_eq!(c.dependencies.cargo.len(), 2);
        assert_eq!(
            c.dependencies.components.keys().next().map(String::as_str),
            Some("slam/core-types")
        );
        assert!(!c.verification.expect("present").verified);
    }

    #[test]
    fn minimal_component_parses() {
        let toml = r#"
            [component]
            name = "x"
            category = "misc"
            version = "0.1.0"
            description = "d"
            license = "MIT"
        "#;
        let c: Component = toml::from_str(toml).expect("parses");
        assert!(c.files.is_empty());
        assert!(c.dependencies.cargo.is_empty());
        assert!(c.hardware.is_none());
    }
}
