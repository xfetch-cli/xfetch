use crate::plugins::{
    CARGO_CMD, CARGO_TOML, ENV_CARGO_NET_GIT_FETCH_WITH_CLI, GIT_CMD, PLUGIN_PREFIX,
    TARGET_RELEASE, default_plugin_dir, default_plugin_repo, plugin_binary_name,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || name.contains('\\')
}

fn resolve_local_plugin_dir(path: &str) -> Result<PathBuf, String> {
    let candidate = PathBuf::from(path);

    if candidate.is_dir() {
        return Ok(candidate);
    }

    if let Ok(cwd) = env::current_dir() {
        let mut search_paths = vec![
            cwd.join(path),
            cwd.join("plugins").join(path),
            cwd.join("plugins").join("plugins").join(path),
        ];

        if let Some(parent) = cwd.parent() {
            search_paths.push(parent.join("plugins").join(path));
            search_paths.push(parent.join("plugins").join("plugins").join(path));
        }

        for search_path in search_paths {
            if search_path.is_dir() {
                return Ok(search_path);
            }
        }
    }

    Err(format!("Plugin not found locally: '{}'", path))
}

pub fn install_plugin(name_or_path: &str, repo: Option<&str>) -> Result<(), String> {
    let plugin_dir = resolve_local_plugin_dir(name_or_path);

    match plugin_dir {
        Ok(dir) => build_and_install_plugin(&dir, name_or_path),
        Err(_) if repo.is_some() || has_path_separator(name_or_path) => {
            let default_repo = default_plugin_repo();
            let repo_url = repo.unwrap_or(default_repo.as_str());
            install_remote_plugin(name_or_path, repo_url)
        }
        Err(_) => {
            let default_repo = default_plugin_repo();
            install_remote_plugin(name_or_path, default_repo.as_str())
        }
    }
}

fn build_and_install_plugin(plugin_dir: &Path, name: &str) -> Result<(), String> {
    if !plugin_dir.join(CARGO_TOML).exists() {
        let display = plugin_dir.display();
        if name.contains('/') || name.contains('\\') {
            return Err(format!("No Cargo.toml found in '{}'", display));
        }
        return Err(format!(
            "Plugin '{}' not found locally and could not be fetched remotely.\n\
             No Cargo.toml found in '{}'.\n\
             Try specifying a different path or check the plugin name.",
            name, display
        ));
    }

    let plugin_name = plugin_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid plugin directory name".to_string())?;

    println!("Building plugin '{}'...", plugin_name);
    let status = Command::new(CARGO_CMD)
        .args(["build", "--release"])
        .env(ENV_CARGO_NET_GIT_FETCH_WITH_CLI, "true")
        .current_dir(plugin_dir)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to run cargo: {}", err))?;

    if !status.success() {
        return Err("Cargo build failed".to_string());
    }

    let binary_name = plugin_binary_name(plugin_name);
    let mut built_binary_candidates = vec![plugin_dir.join(TARGET_RELEASE).join(&binary_name)];

    let mut current = plugin_dir.parent();
    while let Some(dir) = current {
        built_binary_candidates.push(dir.join(TARGET_RELEASE).join(&binary_name));
        current = dir.parent();
    }

    let built_binary = built_binary_candidates
        .into_iter()
        .find(|path| path.is_file())
        .ok_or_else(|| {
            format!(
                "Built binary not found at '{}' or in any workspace target ancestor",
                plugin_dir.join(TARGET_RELEASE).join(&binary_name).display(),
            )
        })?;

    let dest_dir = default_plugin_dir();
    fs::create_dir_all(&dest_dir)
        .map_err(|err| format!("Failed to create plugin directory: {}", err))?;

    let dest_path = dest_dir.join(&binary_name);
    fs::copy(&built_binary, &dest_path)
        .map_err(|err| format!("Failed to copy plugin binary: {}", err))?;

    println!(
        "Installed plugin '{}' to {}",
        plugin_name,
        dest_path.display()
    );
    Ok(())
}

fn install_remote_plugin(name: &str, repo_url: &str) -> Result<(), String> {
    let temp_dir = env::temp_dir().join(format!("{}{}", PLUGIN_PREFIX, name));

    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)
            .map_err(|err| format!("Failed to clean temp directory: {}", err))?;
    }

    let repo_display = repo_url.trim_end_matches(".git");
    println!("Fetching plugin '{}' from {}...", name, repo_display);

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

    let plugin_path = [temp_dir.join(name), temp_dir.join("plugins").join(name)]
        .into_iter()
        .find(|path| path.is_dir());

    let Some(plugin_path) = plugin_path else {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(format!(
            "Plugin '{}' not found in repository '{}'.\n\
             Available plugins can be found in the repository root or under {}/tree/main/plugins",
            name, repo_display, repo_display
        ));
    };

    let result = build_and_install_plugin(&plugin_path, name);

    let _ = fs::remove_dir_all(&temp_dir);
    result
}
