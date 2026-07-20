use crate::plugins::{default_plugin_dir, extract_plugin_name, plugin_binary_name};
use std::fs;
use std::path::PathBuf;

pub fn remove_plugin(name: &str) -> Result<(), String> {
    let binary_name = plugin_binary_name(name);
    let plugin_dir = default_plugin_dir();
    let binary_path = plugin_dir.join(&binary_name);

    if binary_path.is_file() {
        fs::remove_file(&binary_path)
            .map_err(|err| format!("Failed to remove plugin '{}': {}", name, err))?;
        println!("Removed plugin '{}'", name);
        Ok(())
    } else {
        Err(format!(
            "Plugin '{}' is not installed (not found at {})",
            name,
            binary_path.display()
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
