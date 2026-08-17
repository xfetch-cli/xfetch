use crate::extensions::{default_extension_dir, extension_binary_name, extract_extension_name};
use std::fs;
use std::path::PathBuf;

pub fn remove_extension(name: &str) -> Result<(), String> {
    let binary_name = extension_binary_name(name);
    let ext_dir = default_extension_dir();
    let binary_path = ext_dir.join(&binary_name);

    if binary_path.is_file() {
        fs::remove_file(&binary_path)
            .map_err(|err| format!("Failed to remove extension '{}': {}", name, err))?;
        println!("Removed extension '{}'", name);
        Ok(())
    } else {
        Err(format!(
            "Extension '{}' is not installed (not found at {})",
            name,
            binary_path.display()
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
