use crate::error::XFetchError;
use crate::runtime;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

pub fn run(purge: bool, yes: bool, json: bool) -> Result<(), XFetchError> {
    if !yes {
        if std::io::stdin().is_terminal() {
            eprint!(
                "Remove the xfetch binary{}? [y/N]: ",
                if purge { " and its config/data" } else { "" }
            );
            let mut answer = String::new();
            std::io::stdin()
                .read_line(&mut answer)
                .map_err(|err| XFetchError::Fatal(format!("cannot read confirmation: {err}")))?;
            if !matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes") {
                runtime::info("Uninstall cancelled.");
                return Ok(());
            }
        } else {
            return Err(XFetchError::Usage(
                "refusing to uninstall without --yes in non-interactive mode".to_string(),
            ));
        }
    }

    let current = std::env::current_exe()
        .map_err(|err| XFetchError::Fatal(format!("cannot locate the xfetch binary: {err}")))?;
    let mut removed = Vec::new();

    if purge {
        for dir in purge_dirs() {
            if dir.exists() {
                std::fs::remove_dir_all(&dir).map_err(|err| {
                    XFetchError::Fatal(format!("cannot remove '{}': {}", dir.display(), err))
                })?;
                removed.push(dir.display().to_string());
            }
        }
    }

    let backup = backup_path(&current);
    if backup.is_file() {
        let _ = std::fs::remove_file(&backup);
    }
    remove_binary(&current)?;
    removed.push(current.display().to_string());

    if json {
        return crate::cli::output::print_json(&serde_json::json!({
            "schema_version": crate::cli::output::JSON_SCHEMA_VERSION,
            "status": "uninstalled",
            "purged": purge,
            "removed": removed,
        }));
    }
    runtime::info("xfetch uninstalled.");
    for path in &removed {
        println!("  {path}");
    }
    Ok(())
}

fn purge_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(crate::config::config_dir().join("xfetch"));
    if let Some(cache) = dirs::cache_dir() {
        dirs.push(cache.join("xfetch"));
    }
    if let Some(data) = dirs::data_dir() {
        dirs.push(data.join("xfetch"));
    }
    dirs
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_file_name(format!(
        "{}.bak",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("xfetch")
    ))
}

#[cfg(unix)]
fn remove_binary(path: &Path) -> Result<(), XFetchError> {
    std::fs::remove_file(path)
        .map_err(|err| XFetchError::Fatal(format!("cannot remove '{}': {}", path.display(), err)))
}

#[cfg(windows)]
fn remove_binary(path: &Path) -> Result<(), XFetchError> {
    let command = format!(
        "ping 127.0.0.1 -n 3 >NUL & del /F /Q \"{}\"",
        path.display()
    );
    std::process::Command::new("cmd")
        .args(["/C", &command])
        .spawn()
        .map(|_| ())
        .map_err(|err| XFetchError::Fatal(format!("cannot schedule removal: {err}")))
}

#[cfg(not(any(unix, windows)))]
fn remove_binary(path: &Path) -> Result<(), XFetchError> {
    std::fs::remove_file(path)
        .map_err(|err| XFetchError::Fatal(format!("cannot remove '{}': {}", path.display(), err)))
}
