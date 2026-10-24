use crate::plugins::{
    CARGO_CMD, CARGO_TOML, ENV_CARGO_NET_GIT_FETCH_WITH_CLI, PLUGIN_PREFIX, TARGET_RELEASE,
    default_plugin_dir, default_plugin_repo, plugin_binary_name,
};
use crate::wasm::install::SourceAction;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn has_path_separator(name: &str) -> bool {
    name.contains('/') || name.contains('\\')
}

/// True for http(s) URLs, which install prebuilt wasm artifacts directly.
fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

/// Derives the plugin name from a direct wasm file path, stripping the
/// conventional `xfetch-plugin-` prefix when present.
fn plugin_name_from_wasm_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let name = stem.strip_prefix("xfetch-plugin-").unwrap_or(stem);
    (!name.is_empty()).then(|| name.to_string())
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

pub fn install_plugin(
    name_or_path: &str,
    repo: Option<&str>,
    sha256: Option<&str>,
) -> Result<(), String> {
    // Direct URL: download a prebuilt wasm artifact from a release.
    if is_url(name_or_path) {
        let name = crate::wasm::install::name_from_path(name_or_path)
            .ok_or_else(|| format!("Cannot derive a plugin name from URL '{}'", name_or_path))?;
        crate::install_support::validate_guest_name(&name).map_err(|err| {
            format!(
                "Invalid plugin name '{}' derived from URL '{}': {}",
                name, name_or_path, err
            )
        })?;
        return crate::wasm::install::download_and_install(
            name_or_path,
            None,
            sha256,
            &default_plugin_dir(),
            PLUGIN_PREFIX,
            &name,
            "plugin",
        );
    }

    // Direct wasm file.
    let direct = PathBuf::from(name_or_path);
    if direct.is_file() && crate::wasm::is_wasm_file(&direct) {
        let name = plugin_name_from_wasm_file(&direct)
            .ok_or_else(|| format!("Cannot derive a plugin name from '{}'", name_or_path))?;
        return crate::wasm::install::install_file(
            &direct,
            sha256,
            &default_plugin_dir(),
            PLUGIN_PREFIX,
            &name,
            "plugin",
        );
    }

    if direct.is_file() && sha256.is_some() {
        return Err(format!(
            "'{}' is not a WebAssembly artifact; --sha256 cannot be enforced for a native \
             plugin install",
            direct.display()
        ));
    }

    let plugin_dir = resolve_local_plugin_dir(name_or_path);

    match plugin_dir {
        Ok(dir) => build_and_install_plugin(&dir, name_or_path, sha256),
        Err(_) if repo.is_some() || has_path_separator(name_or_path) => {
            let default_repo = default_plugin_repo();
            let repo_url = repo.unwrap_or(default_repo.as_str());
            install_remote_plugin(name_or_path, repo_url, sha256)
        }
        Err(_) => {
            let default_repo = default_plugin_repo();
            install_remote_plugin(name_or_path, default_repo.as_str(), sha256)
        }
    }
}

fn build_and_install_plugin(
    plugin_dir: &Path,
    name: &str,
    sha256: Option<&str>,
) -> Result<(), String> {
    let plugin_name = plugin_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid plugin directory name".to_string())?;

    // Wasm sources (prebuilt artifact, artifact_url or build command) are
    // installed through the shared wasm installer; native crates keep the
    // cargo flow below.
    match crate::wasm::install::plan(plugin_dir, "plugin", plugin_name)? {
        SourceAction::Install(plan) => {
            return crate::wasm::install::install(
                plan,
                sha256,
                &default_plugin_dir(),
                PLUGIN_PREFIX,
                plugin_name,
                "plugin",
            );
        }
        SourceAction::Download { url, manifest } => {
            return crate::wasm::install::download_and_install(
                &url,
                manifest,
                sha256,
                &default_plugin_dir(),
                PLUGIN_PREFIX,
                plugin_name,
                "plugin",
            );
        }
        SourceAction::BuildThenInstall { command } => {
            crate::wasm::install::run_build(plugin_dir, &command)?;
            return match crate::wasm::install::plan(plugin_dir, "plugin", plugin_name)? {
                SourceAction::Install(plan) => crate::wasm::install::install(
                    plan,
                    sha256,
                    &default_plugin_dir(),
                    PLUGIN_PREFIX,
                    plugin_name,
                    "plugin",
                ),
                _ => Err(format!(
                    "Build command in '{}' did not produce a wasm artifact",
                    plugin_dir.display()
                )),
            };
        }
        SourceAction::NotWasm => {
            if sha256.is_some() {
                return Err(format!(
                    "Plugin source '{}' is not a WebAssembly artifact; --sha256 cannot be \
                     enforced. Remove --sha256 for a native plugin install, or install a wasm \
                     artifact.",
                    plugin_dir.display()
                ));
            }
        }
    }

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

    println!(
        "Building plugin '{}' (native executable, unsandboxed)...",
        plugin_name
    );
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
    crate::atomic_fs::copy_atomic(&built_binary, &dest_path)
        .map_err(|err| format!("Failed to copy plugin binary: {}", err))?;

    println!(
        "Installed plugin '{}' to {} (native executable, unsandboxed)",
        plugin_name,
        dest_path.display()
    );
    Ok(())
}

fn install_remote_plugin(name: &str, repo_url: &str, sha256: Option<&str>) -> Result<(), String> {
    crate::install_support::validate_repo(repo_url)?;
    let work_dir = crate::install_support::temp_work_dir(PLUGIN_PREFIX, name)?;
    let temp_dir = work_dir.join("repo");

    let repo_display = repo_url.trim_end_matches(".git");
    println!("Fetching plugin '{}' from {}...", name, repo_display);

    if let Err(err) = crate::install_support::clone_repository(repo_url, &temp_dir, "plugin") {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(err);
    }

    // The official repository nests plugin crates under `plugins/plugins/`;
    // forks may keep them at the root or one level down.
    let plugin_path = [
        temp_dir.join(name),
        temp_dir.join("plugins").join(name),
        temp_dir.join("plugins").join("plugins").join(name),
    ]
    .into_iter()
    .find(|path| path.is_dir());

    let Some(plugin_path) = plugin_path else {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(format!(
            "Plugin '{}' not found in repository '{}'.\n\
             Available plugins can be found in the repository root or under {}/tree/main/plugins",
            name, repo_display, repo_display
        ));
    };

    let result = build_and_install_plugin(&plugin_path, name, sha256);

    let _ = fs::remove_dir_all(&work_dir);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_install_with_sha256_is_rejected() {
        let dir =
            std::env::temp_dir().join(format!("xfetch-plugin-native-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create native plugin dir");
        fs::write(dir.join("Cargo.toml"), "[package]\nname = \"native-test\"")
            .expect("write Cargo.toml");

        let err = build_and_install_plugin(&dir, "native-test", Some(&"a".repeat(64)))
            .expect_err("native install must reject --sha256");
        assert!(
            err.contains("not a WebAssembly artifact"),
            "unexpected error: {}",
            err
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn direct_url_with_unsafe_guest_name_is_rejected() {
        let err = install_plugin("https://example.com/bad%2Fname.wasm", None, None)
            .expect_err("unsafe URL-derived name must be rejected");
        assert!(
            err.contains("Invalid plugin name") && err.contains("derived from URL"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn direct_url_with_normal_guest_name_reaches_the_installer() {
        // The invalid digest fails inside the downloader before any network
        // access, proving the URL-derived "xfetch-plugin-weather" name passed
        // validation.
        let err = install_plugin(
            "https://example.com/xfetch-plugin-weather.wasm",
            None,
            Some("abc"),
        )
        .expect_err("invalid digest must be rejected");
        assert!(
            err.contains("64-character SHA-256"),
            "unexpected error: {}",
            err
        );
    }
}
