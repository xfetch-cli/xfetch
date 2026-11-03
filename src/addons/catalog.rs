//! Remote addon catalogs used by `search` and `available`.
//!
//! Each addon repository publishes an `index.json` at its root. The catalog is
//! intentionally small: it only carries the metadata needed to discover an
//! addon before installing it. Installation still uses the repository and
//! manifest, so the catalog cannot grant capabilities by itself.

use crate::addons::model::AddonKind;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::Duration;

/// Maximum catalog size accepted from a remote source.
const CATALOG_LIMIT: u64 = 2 * 1024 * 1024;

/// Default raw catalog URL for each repository.
const PLUGIN_CATALOG: &str = "https://raw.githubusercontent.com/xfetch-cli/plugins/main/index.json";
const EFFECT_CATALOG: &str = "https://raw.githubusercontent.com/xfetch-cli/effects/main/index.json";
const EXTENSION_CATALOG: &str =
    "https://raw.githubusercontent.com/xfetch-cli/extensions/main/index.json";

/// One addon advertised by a repository catalog.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CatalogEntry {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub install: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guest_runtime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<serde_json::Value>,
}

impl CatalogEntry {
    fn install_name(&self) -> &str {
        self.install.as_deref().unwrap_or(&self.name)
    }

    fn matches(&self, query: &str) -> bool {
        if query.is_empty() {
            return true;
        }
        let query = query.to_ascii_lowercase();
        let haystack = [
            self.name.as_str(),
            self.description.as_str(),
            self.kind.as_deref().unwrap_or_default(),
            self.runtime.as_deref().unwrap_or_default(),
        ];
        haystack
            .iter()
            .any(|value| value.to_ascii_lowercase().contains(&query))
    }
}

/// A repository catalog document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Catalog {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub catalog_version: Option<String>,
    #[serde(default)]
    pub entries: Vec<CatalogEntry>,
}

fn default_schema_version() -> u32 {
    1
}

/// Fetches the catalog for `kind` and filters it by `query`.
pub fn search(kind: AddonKind, query: &str) -> Result<Vec<CatalogEntry>, String> {
    let catalog = load(kind)?;
    Ok(catalog
        .entries
        .into_iter()
        .filter(|entry| entry.matches(query))
        .collect())
}

/// Prints search results as a readable list or JSON.
pub fn print_search(kind: AddonKind, query: Option<&str>, json: bool) -> Result<(), String> {
    let query = query.unwrap_or("");
    let entries = search(kind, query)?;

    if json {
        let body = serde_json::to_string_pretty(&entries)
            .map_err(|err| format!("Failed to serialize catalog results: {}", err))?;
        println!("{}", body);
        return Ok(());
    }

    if entries.is_empty() {
        if query.is_empty() {
            println!("No {} available in the catalog.", kind.plural_label());
        } else {
            println!("No {} matched '{}'.", kind.plural_label(), query);
        }
        return Ok(());
    }

    println!("Available {}:", kind.plural_label());
    for entry in entries {
        let version = entry.version.as_deref().unwrap_or("-");
        let entry_kind = entry.kind.as_deref().unwrap_or(kind.label());
        let runtime = entry.runtime.as_deref().unwrap_or("unknown");
        println!(
            "  {}  ({}, {}, v{})",
            entry.name, entry_kind, runtime, version
        );
        if !entry.description.is_empty() {
            println!("    {}", entry.description);
        }
        println!(
            "    install: {}",
            install_command(kind, entry.install_name())
        );
    }
    Ok(())
}

fn install_command(kind: AddonKind, name: &str) -> String {
    match kind {
        AddonKind::Plugin => format!("xfetch plugin install {}", name),
        AddonKind::Effect => format!("xfetch effects install {}", name),
        AddonKind::Extension => format!("xfetch extension install {}", name),
    }
}

fn load(kind: AddonKind) -> Result<Catalog, String> {
    let source = catalog_source(kind);
    let body = if source.starts_with("http://") || source.starts_with("https://") {
        fetch_remote(&source)?
    } else {
        fs::read_to_string(&source)
            .map_err(|err| format!("Failed to read catalog '{}': {}", source, err))?
    };
    let catalog = parse(&body).map_err(|err| format!("Invalid catalog '{}': {}", source, err))?;
    if catalog.schema_version != 1 {
        return Err(format!(
            "Unsupported catalog schema version {} in '{}'",
            catalog.schema_version, source
        ));
    }
    Ok(catalog)
}

fn catalog_source(kind: AddonKind) -> String {
    let (env_name, default) = match kind {
        AddonKind::Plugin => ("XFETCH_PLUGIN_INDEX", PLUGIN_CATALOG),
        AddonKind::Effect => ("XFETCH_EFFECT_INDEX", EFFECT_CATALOG),
        AddonKind::Extension => ("XFETCH_EXTENSION_INDEX", EXTENSION_CATALOG),
    };
    std::env::var(env_name).unwrap_or_else(|_| default.to_string())
}

fn fetch_remote(source: &str) -> Result<String, String> {
    let mut response = ureq::get(source)
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(|err| format!("Failed to fetch catalog '{}': {}", source, err))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!(
            "Failed to fetch catalog '{}': HTTP {}",
            source, status
        ));
    }
    response
        .body_mut()
        .with_config()
        .limit(CATALOG_LIMIT)
        .read_to_string()
        .map_err(|err| format!("Failed to read catalog '{}': {}", source, err))
}

fn parse(body: &str) -> Result<Catalog, serde_json::Error> {
    serde_json::from_str(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
    {
      "schema_version": 1,
      "catalog_version": "1.0.2",
      "entries": [
        {
          "name": "wasm-crypto",
          "description": "Crypto prices over HTTP",
          "version": "0.2.0",
          "kind": "info_provider",
          "runtime": "wasm",
          "path": "plugins/wasm-crypto",
          "install": "wasm-crypto"
        },
        {
          "name": "weather",
          "description": "Current weather",
          "version": "0.2.0",
          "kind": "info_provider",
          "runtime": "native",
          "path": "plugins/weather"
        }
      ]
    }
    "#;

    #[test]
    fn parses_catalog_and_filters_entries() {
        let catalog = parse(SAMPLE).expect("catalog");
        assert_eq!(catalog.entries.len(), 2);
        assert!(catalog.entries[0].matches("crypto"));
        assert!(catalog.entries[0].matches("WASM"));
        assert!(!catalog.entries[0].matches("weather"));
        assert_eq!(catalog.entries[1].install_name(), "weather");
    }

    #[test]
    fn install_commands_match_the_command_family() {
        assert_eq!(
            install_command(AddonKind::Plugin, "weather"),
            "xfetch plugin install weather"
        );
        assert_eq!(
            install_command(AddonKind::Effect, "matrix"),
            "xfetch effects install matrix"
        );
        assert_eq!(
            install_command(AddonKind::Extension, "night"),
            "xfetch extension install night"
        );
    }
}
