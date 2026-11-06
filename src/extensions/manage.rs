use crate::addons::model::AddonKind;
use crate::extensions::{
    default_extension_dir, extension_binary_name, extension_manifest_name, extension_wasm_name,
    extract_extension_name,
};
use std::fs;
use std::path::PathBuf;

/// Removes an extension regardless of its runtime: the native binary, the wasm
/// artifact, the wasm sidecar manifest, the install receipt and the update
/// backups are all cleaned up.
pub fn remove_extension(name: &str) -> Result<(), String> {
    let ext_dir = default_extension_dir();
    let kind = AddonKind::Extension;
    let native = ext_dir.join(extension_binary_name(name));
    let wasm = ext_dir.join(extension_wasm_name(name));
    let manifest = ext_dir.join(extension_manifest_name(name));
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
                .map_err(|err| format!("Failed to remove extension '{}': {}", name, err))?;
            removed = true;
        }
    }

    if removed {
        println!("Removed extension '{}'", name);
        Ok(())
    } else {
        Err(format!(
            "Extension '{}' is not installed (not found in {})",
            name,
            ext_dir.display()
        ))
    }
}

pub fn list_extensions() -> Result<Vec<(String, PathBuf)>, String> {
    let mut extensions = Vec::new();

    let ext_dir = default_extension_dir();
    if ext_dir.is_dir() {
        for entry in fs::read_dir(&ext_dir)
            .map_err(|err| format!("Failed to read extension directory: {}", err))?
        {
            let entry = entry.map_err(|err| format!("Failed to read entry: {}", err))?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = extract_extension_name(&path)
            {
                extensions.push((name, path));
            }
        }
    }

    extensions.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(extensions)
}
