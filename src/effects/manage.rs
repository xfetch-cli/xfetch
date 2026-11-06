use crate::addons::model::AddonKind;
use crate::effects::{
    default_effect_dir, effect_binary_name, effect_manifest_name, effect_wasm_name,
    extract_effect_name,
};
use std::fs;
use std::path::PathBuf;

/// Removes an effect regardless of its runtime: the native binary, the wasm
/// artifact, the wasm sidecar manifest, the install receipt and the update
/// backups are all cleaned up.
pub fn remove_effect(name: &str) -> Result<(), String> {
    let effect_dir = default_effect_dir();
    let kind = AddonKind::Effect;
    let native = effect_dir.join(effect_binary_name(name));
    let wasm = effect_dir.join(effect_wasm_name(name));
    let manifest = effect_dir.join(effect_manifest_name(name));
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
                .map_err(|err| format!("Failed to remove effect '{}': {}", name, err))?;
            removed = true;
        }
    }

    if removed {
        Ok(())
    } else {
        Err(format!(
            "Effect '{}' is not installed (not found in {})",
            name,
            effect_dir.display()
        ))
    }
}

pub fn list_effects() -> Result<Vec<(String, PathBuf)>, String> {
    let mut effects = Vec::new();

    let effect_dir = default_effect_dir();
    if effect_dir.is_dir() {
        for entry in fs::read_dir(&effect_dir)
            .map_err(|err| format!("Failed to read effect directory: {}", err))?
        {
            let entry = entry.map_err(|err| format!("Failed to read entry: {}", err))?;
            let path = entry.path();
            if path.is_file()
                && let Some(name) = extract_effect_name(&path)
            {
                effects.push((name, path));
            }
        }
    }

    effects.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(effects)
}
