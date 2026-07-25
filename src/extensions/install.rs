use crate::extensions::{
    DEFAULT_EXTENSION_REPO, EXTENSION_PREFIX, default_extension_dir, extension_binary_name,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const CARGO_CMD: &str = "cargo";
const CARGO_TOML: &str = "Cargo.toml";
const GIT_CMD: &str = "git";
const TARGET_RELEASE: &str = "target/release";
const ENV_CARGO_NET_GIT_FETCH_WITH_CLI: &str = "CARGO_NET_GIT_FETCH_WITH_CLI";

const EXTENSIONS_SUBDIR: &str = "extensions";

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || name.contains('\\')
}

fn resolve_local_extension_dir(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);

    if candidate.is_dir() {
        return Ok(candidate);
    }

    if let Ok(cwd) = env::current_dir() {
        let mut search_paths = vec![
            cwd.join(path),
            cwd.join("extensions").join(path),
            cwd.join("extensions").join(EXTENSIONS_SUBDIR).join(path),
        ];

        if let Some(parent) = cwd.parent() {
            search_paths.push(parent.join("extensions").join(path));
            search_paths.push(parent.join("extensions").join(EXTENSIONS_SUBDIR).join(path));
        }

        for search_path in search_paths {
            if search_path.is_dir() {
                return Ok(search_path);
            }
        }
    }

    Err(format!("Extension not found locally: '{}'", path))
}

pub fn install_extension(name_or_path: &str, repo: Option<&str>) -> Result<(), String> {
    let ext_dir = resolve_local_extension_dir(name_or_path);

    match ext_dir {
        Ok(dir) => build_and_install_extension(&dir, name_or_path),
        Err(_) if repo.is_some() || has_path_separator(name_or_path) => {
            let default_repo = DEFAULT_EXTENSION_REPO;
            let repo_url = repo.unwrap_or(default_repo);
            install_remote_extension(name_or_path, repo_url)
        }
        Err(_) => {
            install_remote_extension(name_or_path, DEFAULT_EXTENSION_REPO)
        }
    }
}

fn build_and_install_extension(ext_dir: &Path, name: &str) -> Result<(), String> {
    if !ext_dir.join(CARGO_TOML).exists() {
        let display = ext_dir.display();
        if name.contains('/') || name.contains('\\') {
            return Err(format!("No Cargo.toml found in '{}'", display));
        }
        return Err(format!(
            "Extension '{}' not found locally and could not be fetched remotely.\n\
             No Cargo.toml found in '{}'.\n\
             Try specifying a different path or check the extension name.",
            name, display
        ));
    }

    let ext_name = ext_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid extension directory name".to_string())?;

    println!("Building extension '{}'...", ext_name);
    let status = Command::new(CARGO_CMD)
        .args(["build", "--release"])
        .env(ENV_CARGO_NET_GIT_FETCH_WITH_CLI, "true")
        .current_dir(ext_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to run cargo: {}", err))?;

    if !status.success() {
        return Err("Cargo build failed".to_string());
    }

    let binary_name = extension_binary_name(ext_name);
    let mut built_candidates = vec![ext_dir.join(TARGET_RELEASE).join(&binary_name)];

    let mut current = ext_dir.parent();
    while let Some(dir) = current {
        built_candidates.push(dir.join(TARGET_RELEASE).join(&binary_name));
        current = dir.parent();
    }

    let built_binary = built_candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "Built binary not found at '{}' or in any workspace target ancestor",
                ext_dir.join(TARGET_RELEASE).join(&binary_name).display(),
            )
        })?;

    let dest_dir = default_extension_dir();
    fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("Failed to create extension directory: {}", err))?;

    let dest_path = dest_dir.join(&binary_name);
    fs::copy(&built_binary, &dest_path)
        .map_err(|err| format!("Failed to copy extension binary: {}", err))?;

    println!(
        "Installed extension '{}' to {}",
        ext_name,
        dest_path.display()
    );
    Ok(())
}

fn install_remote_extension(name: &str, repo_url: &str) -> Result<(), String> {
    let temp_dir = env::temp_dir().join(format!("{}{}", EXTENSION_PREFIX, name));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|err| format!("Failed to clean temp directory: {}", err))?;
    }

    let repo_display = repo_url.trim_end_matches(".git");
    println!("Fetching extension '{}' from {}...", name, repo_display);

    let status = Command::new(GIT_CMD)
        .args(["clone", "--depth", "1", repo_url])
        .arg(&temp_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to run git: {}. Is git installed?", err))?;

    if !status.success() {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err("Failed to clone repository".to_string());
    }

    let ext_path = [
        temp_dir.join(name),
        temp_dir.join(EXTENSIONS_SUBDIR).join(name),
    ]
    .into_iter()
    .find(|path| path.is_dir());

    let Some(ext_path) = ext_path else {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Extension '{}' not found in repository '{}'.\n\
             Available extensions can be found in the repository root or under {}/tree/main/extensions",
            name, repo_display, repo_display
        ));
    };

    let result = build_and_install_extension(&ext_path, name);

    let _ = fs::remove_dir_all(&temp_dir);
    result
}
