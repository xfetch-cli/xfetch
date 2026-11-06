//! Installed-addon metadata used by `plugin/effects/extension update`.
//!
//! Native addons are built from source and have no embedded version, so the
//! installer records how they were installed in a sidecar receipt. The updater
//! uses that receipt to decide where to rebuild from and which version is
//! currently installed.

use crate::addons::catalog::CatalogEntry;
use crate::addons::model::AddonKind;
use crate::hash::sha256_hex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Receipt schema accepted by this version.
pub const SCHEMA_VERSION: u32 = 1;

/// Metadata written after a successful addon install.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InstallReceipt {
    pub schema_version: u32,
    pub kind: String,
    pub name: String,
    pub runtime: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub repo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(rename = "ref", default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    #[serde(default)]
    pub installed_at: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
}

impl InstallReceipt {
    /// Location of the receipt for an installed addon.
    pub fn path(kind: AddonKind, name: &str) -> PathBuf {
        kind.receipt_path(name)
    }

    /// Loads the receipt for `name`, if present and valid.
    pub fn load(kind: AddonKind, name: &str) -> Option<Self> {
        Self::load_from(&Self::path(kind, name))
    }

    /// Loads and validates a receipt from an explicit path.
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = fs::read_to_string(path).ok()?;
        let receipt: Self = serde_json::from_str(&text).ok()?;
        (receipt.schema_version == SCHEMA_VERSION).then_some(receipt)
    }

    /// Writes the receipt atomically.
    pub fn write(&self, kind: AddonKind, name: &str) -> Result<(), String> {
        self.write_to(&Self::path(kind, name))
    }

    /// Writes the receipt atomically to an explicit path.
    pub fn write_to(&self, path: &Path) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|err| format!("Failed to serialize install receipt: {}", err))?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("Failed to create '{}': {}", parent.display(), err))?;
        }
        crate::atomic_fs::write_atomic(path, &bytes)
            .map_err(|err| format!("Failed to write '{}': {}", path.display(), err))
    }
}

/// Records the source and installed version after a successful install.
///
/// `source` is `catalog`, `local`, `repo`, `url` or `manual`. `source_path`
/// stores a local directory or URL; `repo` is the repository URL. A catalog
/// entry, when available, supplies the version, SDK and reference.
pub fn record(
    kind: AddonKind,
    name: &str,
    source: &str,
    repo: Option<&str>,
    source_path: Option<&str>,
    catalog: Option<&CatalogEntry>,
) -> Result<(), String> {
    let path = kind.installed_path(name).ok_or_else(|| {
        format!(
            "Cannot write install receipt: {} '{}' is not installed",
            kind.label(),
            name
        )
    })?;
    let runtime = if crate::wasm::is_wasm_file(&path) {
        "wasm"
    } else {
        "native"
    };

    let previous = InstallReceipt::load(kind, name);
    let wasm_version = if runtime == "wasm" {
        crate::wasm::manifest::load(&path)
            .ok()
            .and_then(|loaded| loaded.manifest.version)
    } else {
        None
    };
    let version = catalog
        .and_then(|entry| entry.version.clone())
        .or(wasm_version)
        .or_else(|| {
            previous
                .as_ref()
                .and_then(|receipt| receipt.version.clone())
        });
    let sdk_version = catalog
        .and_then(|entry| entry.sdk_version.clone())
        .or_else(|| {
            previous
                .as_ref()
                .and_then(|receipt| receipt.sdk_version.clone())
        });
    let reference = catalog
        .and_then(|entry| entry.reference.clone())
        .or_else(|| {
            previous
                .as_ref()
                .and_then(|receipt| receipt.reference.clone())
        });
    let repo = repo
        .map(str::to_string)
        .or_else(|| catalog.and_then(|entry| entry.repo.clone()))
        .or_else(|| previous.as_ref().and_then(|receipt| receipt.repo.clone()));
    let stored_path = if matches!(source, "local" | "url" | "repo") {
        source_path
            .map(str::to_string)
            .or_else(|| previous.as_ref().and_then(|receipt| receipt.path.clone()))
            .or_else(|| catalog.and_then(|entry| entry.path.clone()))
    } else {
        catalog
            .and_then(|entry| entry.path.clone())
            .or_else(|| source_path.map(str::to_string))
            .or_else(|| previous.as_ref().and_then(|receipt| receipt.path.clone()))
    };
    let sha256 = fs::read(&path).ok().map(|bytes| sha256_hex(&bytes));

    let receipt = InstallReceipt {
        schema_version: SCHEMA_VERSION,
        kind: kind.label().to_string(),
        name: name.to_string(),
        runtime: runtime.to_string(),
        version,
        sdk_version,
        source: source.to_string(),
        repo,
        path: stored_path,
        reference,
        installed_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
        sha256,
    };
    receipt.write(kind, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch-receipt-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    #[test]
    fn parses_optional_fields() {
        let text = r#"{
          "schema_version": 1,
          "kind": "plugin",
          "name": "weather",
          "runtime": "native",
          "source": "catalog",
          "repo": "https://github.com/xfetch-cli/plugins.git",
          "path": "plugins/weather",
          "ref": "v0.2.0",
          "version": "0.2.0",
          "sdk_version": "0.3.0",
          "installed_at": 100,
          "sha256": "abc"
        }"#;
        let receipt: InstallReceipt = serde_json::from_str(text).expect("parse");
        assert_eq!(receipt.name, "weather");
        assert_eq!(receipt.reference.as_deref(), Some("v0.2.0"));
        assert_eq!(receipt.sdk_version.as_deref(), Some("0.3.0"));
    }

    #[test]
    fn writes_and_reads_a_receipt() {
        let dir = scratch("roundtrip");
        let path = dir.join("install.json");
        let receipt = InstallReceipt {
            schema_version: SCHEMA_VERSION,
            kind: "effect".to_string(),
            name: "decrypt".to_string(),
            runtime: "native".to_string(),
            version: Some("0.2.0".to_string()),
            sdk_version: None,
            source: "local".to_string(),
            repo: None,
            path: Some("/tmp/effects/decrypt".to_string()),
            reference: None,
            installed_at: 1,
            sha256: None,
        };
        receipt.write_to(&path).expect("write");
        assert_eq!(InstallReceipt::load_from(&path), Some(receipt));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_other_schema_versions() {
        let dir = scratch("schema");
        let path = dir.join("install.json");
        fs::write(
            &path,
            r#"{"schema_version":2,"kind":"plugin","name":"x","runtime":"native","source":"local"}"#,
        )
        .expect("write");
        assert!(InstallReceipt::load_from(&path).is_none());
        let _ = fs::remove_dir_all(&dir);
    }
}
