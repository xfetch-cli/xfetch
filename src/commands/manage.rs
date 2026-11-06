use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::addons::model::AddonKind;
use crate::commands::{
    COMMAND_PREFIX, EXE_EXT, command_binary_name, command_manifest_name, command_wasm_name,
    default_command_dir,
};

pub fn list_commands() -> Result<Vec<(String, PathBuf)>, String> {
    let mut commands = Vec::new();
    let directory = default_command_dir();
    if directory.is_dir() {
        let entries = fs::read_dir(&directory)
            .map_err(|err| format!("Failed to read commands directory: {err}"))?;
        for entry in entries {
            let entry = entry.map_err(|err| format!("Failed to read entry: {err}"))?;
            let path = entry.path();
            if let Some(name) = extract_command_name(&path) {
                commands.push((name, path));
            }
        }
    }
    commands.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(commands)
}

pub fn remove_command(name: &str) -> Result<(), String> {
    let kind = AddonKind::Command;
    let directory = default_command_dir();
    let native = directory.join(command_binary_name(name));
    let wasm = directory.join(command_wasm_name(name));
    let manifest = directory.join(command_manifest_name(name));
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
                .map_err(|err| format!("Failed to remove command '{name}': {err}"))?;
            removed = true;
        }
    }
    if removed {
        Ok(())
    } else {
        Err(format!(
            "Command '{name}' is not installed (not found in {})",
            directory.display()
        ))
    }
}

pub fn find_command_binary(name: &str) -> Option<PathBuf> {
    let direct = Path::new(name);
    if direct.is_file() {
        return Some(direct.to_path_buf());
    }

    let directory = default_command_dir();
    for candidate in [
        directory.join(command_binary_name(name)),
        directory.join(command_wasm_name(name)),
    ] {
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    if let Ok(dev_dir) = env::var("XFETCH_COMMAND_DEV_DIR") {
        let dev = PathBuf::from(dev_dir);
        for candidate in [
            dev.join(command_binary_name(name)),
            dev.join(command_wasm_name(name)),
        ] {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    if let Ok(path) = env::var("PATH") {
        for directory in env::split_paths(&path) {
            for binary in [command_binary_name(name), command_wasm_name(name)] {
                let candidate = directory.join(binary);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    None
}

fn extract_command_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    let name = filename.strip_prefix(COMMAND_PREFIX)?;
    if name.ends_with(".json") || name.ends_with(".bak") {
        return None;
    }
    let name = name.strip_suffix(".wasm").unwrap_or(name);
    if cfg!(target_os = "windows") {
        name.strip_suffix(EXE_EXT).map(str::to_string)
    } else {
        Some(name.to_string())
    }
}
