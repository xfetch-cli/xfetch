use crate::effects::{default_effect_dir, effect_binary_name, extract_effect_name};
use std::fs;
use std::path::PathBuf;

pub fn remove_effect(name: &str) -> Result<(), String> {
    let binary_name = effect_binary_name(name);
    let effect_dir = default_effect_dir();
    let binary_path = effect_dir.join(&binary_name);

    if binary_path.is_file() {
        fs::remove_file(&binary_path)
            .map_err(|err| format!("Failed to remove effect '{}': {}", name, err))?;
        println!("Removed effect '{}'", name);
        Ok(())
    } else {
        Err(format!(
            "Effect '{}' is not installed (not found at {})",
            name,
            binary_path.display()
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
