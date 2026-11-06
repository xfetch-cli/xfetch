use crate::addons::model::AddonKind;
use crate::plugins::{
    default_plugin_dir, extract_plugin_name, plugin_binary_name, plugin_manifest_name,
    plugin_wasm_name,
};
use std::fs;
use std::path::PathBuf;

/// Removes a plugin regardless of its runtime: the native binary, the wasm
/// artifact, the wasm sidecar manifest, the install receipt and the update
/// backups are all cleaned up.
pub fn remove_plugin(name: &str) -> Result<(), String> {
    let plugin_dir = default_plugin_dir();
    let kind = AddonKind::Plugin;
    let native = plugin_dir.join(plugin_binary_name(name));
    let wasm = plugin_dir.join(plugin_wasm_name(name));
    let manifest = plugin_dir.join(plugin_manifest_name(name));
    let receipt = kind.receipt_path(name);
    let candidates = [
        native.clone(),
        wasm.clone(),
        manifest.clone(),
        receipt.clone(),
        kind.backup_path(&native),
        kind.backup_path(&wasm),
        kind.backup_path(&manifest),
        kind.backup_path(&receipt),
    ];

    let mut removed = false;
    for path in &candidates {
        if path.is_file() {
            fs::remove_file(path)
                .map_err(|err| format!("Failed to remove plugin '{}': {}", name, err))?;
            removed = true;
        }
    }

    if removed {
        println!("Removed plugin '{}'", name);
        Ok(())
    } else {
        Err(format!(
            "Plugin '{}' is not installed (not found in {})",
            name,
            plugin_dir.display()
        ))
    }
}

pub fn list_plugins() -> Result<Vec<(String, PathBuf)>, String> {
    let mut plugins = Vec::new();

    let plugin_dir = default_plugin_dir();
    if plugin_dir.is_dir() {
        for entry in fs::read_dir(&plugin_dir)
            .map_err(|err| format!("Failed to read plugin directory: {}", err))?
        {
            let entry = entry.map_err(|err| format!("Failed to read entry: {}", err))?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = extract_plugin_name(&path)
            {
                plugins.push((name, path));
            }
        }
    }

    plugins.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(plugins)
}
