use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::commands::{COMMAND_PREFIX, default_command_dir, default_command_repo};
use crate::wasm::install::SourceAction;

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || name.contains('\\')
}

fn command_name_from_wasm_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let name = stem.strip_prefix(COMMAND_PREFIX).unwrap_or(stem);
    (!name.is_empty()).then(|| name.to_string())
}

pub fn install_command(
    name_or_path: &str,
    repo: Option<&str>,
    sha256: Option<&str>,
) -> Result<(), String> {
    if is_url(name_or_path) {
        let name = crate::wasm::install::name_from_path(name_or_path)
            .ok_or_else(|| format!("Cannot derive a command name from URL '{name_or_path}'"))?;
        crate::install_support::validate_guest_name(&name).map_err(|err| {
            format!("Invalid command name '{name}' derived from URL '{name_or_path}': {err}")
        })?;
        return crate::wasm::install::download_and_install(
            name_or_path,
            None,
            sha256,
            &default_command_dir(),
            COMMAND_PREFIX,
            &name,
            "command",
        );
    }

    let direct = PathBuf::from(name_or_path);
    if direct.is_file() && crate::wasm::is_wasm_file(&direct) {
        let name = command_name_from_wasm_file(&direct)
            .ok_or_else(|| format!("Cannot derive a command name from '{name_or_path}'"))?;
        return crate::wasm::install::install_file(
            &direct,
            sha256,
            &default_command_dir(),
            COMMAND_PREFIX,
            &name,
            "command",
        );
    }

    if direct.is_file() && sha256.is_some() {
        return Err(format!(
            "'{}' is not a WebAssembly artifact; --sha256 cannot be enforced for a native command",
            direct.display()
        ));
    }

    match resolve_local_command_dir(name_or_path) {
        Ok(directory) => build_and_install_command(&directory, sha256),
        Err(_) if repo.is_some() || has_path_separator(name_or_path) => {
            let default_repo = default_command_repo();
            install_remote_command(name_or_path, repo.unwrap_or(&default_repo), sha256)
        }
        Err(_) => {
            let default_repo = default_command_repo();
            install_remote_command(name_or_path, &default_repo, sha256)
        }
    }
}

fn resolve_local_command_dir(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);
    if candidate.is_dir() {
        return Ok(candidate);
    }

    if let Ok(cwd) = env::current_dir() {
        let mut search = vec![
            cwd.join(path),
            cwd.join("commands").join(path),
            cwd.join("commands").join("commands").join(path),
        ];
        if let Some(parent) = cwd.parent() {
            search.push(parent.join("commands").join(path));
            search.push(parent.join("commands").join("commands").join(path));
        }
        for candidate in search {
            if candidate.is_dir() {
                return Ok(candidate);
            }
        }
    }

    Err(format!("Command not found locally: '{path}'"))
}

fn build_and_install_command(directory: &Path, sha256: Option<&str>) -> Result<(), String> {
    let name = directory
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "Invalid command directory name".to_string())?;

    match crate::wasm::install::plan(directory, "command", name)? {
        SourceAction::Install(plan) => crate::wasm::install::install(
            plan,
            sha256,
            &default_command_dir(),
            COMMAND_PREFIX,
            name,
            "command",
        ),
        SourceAction::Download { url, manifest } => crate::wasm::install::download_and_install(
            &url,
            manifest,
            sha256,
            &default_command_dir(),
            COMMAND_PREFIX,
            name,
            "command",
        ),
        SourceAction::BuildThenInstall { command } => {
            crate::wasm::install::run_build(directory, &command)?;
            match crate::wasm::install::plan(directory, "command", name)? {
                SourceAction::Install(plan) => crate::wasm::install::install(
                    plan,
                    sha256,
                    &default_command_dir(),
                    COMMAND_PREFIX,
                    name,
                    "command",
                ),
                _ => Err(format!(
                    "Build command in '{}' did not produce a wasm artifact",
                    directory.display()
                )),
            }
        }
        SourceAction::NotWasm => Err(format!(
            "Command source '{}' is not a WebAssembly artifact; command addons must be wasm",
            directory.display()
        )),
    }
}

fn install_remote_command(name: &str, repo_url: &str, sha256: Option<&str>) -> Result<(), String> {
    crate::install_support::validate_repo(repo_url)?;
    let work_dir = crate::install_support::temp_work_dir(COMMAND_PREFIX, name)?;
    let temp_dir = work_dir.join("repo");

    let repo_display = repo_url.trim_end_matches(".git");
    println!("Fetching command '{name}' from {repo_display}...");

    if let Err(err) = crate::install_support::clone_repository(repo_url, &temp_dir, "command") {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(err);
    }

    let command_path = [
        temp_dir.join(name),
        temp_dir.join("commands").join(name),
        temp_dir.join("commands").join("commands").join(name),
    ]
    .into_iter()
    .find(|path| path.is_dir());

    let Some(command_path) = command_path else {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(format!(
            "Command '{name}' not found in repository '{repo_display}'.\n\
             Available commands live in the repository root or under commands/commands"
        ));
    };

    let result = build_and_install_command(&command_path, sha256);
    let _ = fs::remove_dir_all(&work_dir);
    result
}
