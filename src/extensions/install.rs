use crate::extensions::{
    DEFAULT_EXTENSION_REPO, EXTENSION_PREFIX, default_extension_dir, extension_binary_name,
};
use crate::wasm::install::SourceAction;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// True for http(s) URLs, which install prebuilt wasm artifacts directly.
fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

/// Derives the extension name from a direct wasm file path.
fn extension_name_from_wasm_file(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let name = stem.strip_prefix("xfetch-extension-").unwrap_or(stem);
    (!name.is_empty()).then(|| name.to_string())
}

const CARGO_CMD: &str = "cargo";
const CARGO_TOML: &str = "Cargo.toml";
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

pub fn install_extension(
    name_or_path: &str,
    repo: Option<&str>,
    sha256: Option<&str>,
) -> Result<(), String> {
    // Direct URL: download a prebuilt wasm artifact from a release.
    if is_url(name_or_path) {
        let name = crate::wasm::install::name_from_path(name_or_path).ok_or_else(|| {
            format!(
                "Cannot derive an extension name from URL '{}'",
                name_or_path
            )
        })?;
        crate::install_support::validate_guest_name(&name).map_err(|err| {
            format!(
                "Invalid extension name '{}' derived from URL '{}': {}",
                name, name_or_path, err
            )
        })?;
        return crate::wasm::install::download_and_install(
            name_or_path,
            None,
            sha256,
            &default_extension_dir(),
            EXTENSION_PREFIX,
            &name,
            "extension",
        );
    }

    // Direct wasm file.
    let direct = PathBuf::from(name_or_path);
    if direct.is_file() && crate::wasm::is_wasm_file(&direct) {
        let name = extension_name_from_wasm_file(&direct)
            .ok_or_else(|| format!("Cannot derive an extension name from '{}'", name_or_path))?;
        return crate::wasm::install::install_file(
            &direct,
            sha256,
            &default_extension_dir(),
            EXTENSION_PREFIX,
            &name,
            "extension",
        );
    }

    if direct.is_file() && sha256.is_some() {
        return Err(format!(
            "'{}' is not a WebAssembly artifact; --sha256 cannot be enforced for a native \
             extension install",
            direct.display()
        ));
    }

    let ext_dir = resolve_local_extension_dir(name_or_path);

    match ext_dir {
        Ok(dir) => build_and_install_extension(&dir, name_or_path, sha256),
        Err(_) if repo.is_some() || has_path_separator(name_or_path) => {
            let default_repo = DEFAULT_EXTENSION_REPO;
            let repo_url = repo.unwrap_or(default_repo);
            install_remote_extension(name_or_path, repo_url, sha256)
        }
        Err(_) => install_remote_extension(name_or_path, DEFAULT_EXTENSION_REPO, sha256),
    }
}

fn build_and_install_extension(
    ext_dir: &Path,
    name: &str,
    sha256: Option<&str>,
) -> Result<(), String> {
    let ext_name = ext_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| "Invalid extension directory name".to_string())?;

    // Wasm sources (prebuilt artifact, artifact_url or build command) are
    // installed through the shared wasm installer; native crates keep the
    // cargo flow below.
    match crate::wasm::install::plan(ext_dir, "extension", ext_name)? {
        SourceAction::Install(plan) => {
            return crate::wasm::install::install(
                plan,
                sha256,
                &default_extension_dir(),
                EXTENSION_PREFIX,
                ext_name,
                "extension",
            );
        }
        SourceAction::Download { url, manifest } => {
            return crate::wasm::install::download_and_install(
                &url,
                manifest,
                sha256,
                &default_extension_dir(),
                EXTENSION_PREFIX,
                ext_name,
                "extension",
            );
        }
        SourceAction::BuildThenInstall { command } => {
            crate::wasm::install::run_build(ext_dir, &command)?;
            return match crate::wasm::install::plan(ext_dir, "extension", ext_name)? {
                SourceAction::Install(plan) => crate::wasm::install::install(
                    plan,
                    sha256,
                    &default_extension_dir(),
                    EXTENSION_PREFIX,
                    ext_name,
                    "extension",
                ),
                _ => Err(format!(
                    "Build command in '{}' did not produce a wasm artifact",
                    ext_dir.display()
                )),
            };
        }
        SourceAction::NotWasm => {
            if sha256.is_some() {
                return Err(format!(
                    "Extension source '{}' is not a WebAssembly artifact; --sha256 cannot be \
                     enforced. Remove --sha256 for a native extension install, or install a wasm \
                     artifact.",
                    ext_dir.display()
                ));
            }
        }
    }

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

    println!(
        "Building extension '{}' (native executable, unsandboxed)...",
        ext_name
    );
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
    crate::atomic_fs::copy_atomic(&built_binary, &dest_path)
        .map_err(|err| format!("Failed to copy extension binary: {}", err))?;

    println!(
        "Installed extension '{}' to {} (native executable, unsandboxed)",
        ext_name,
        dest_path.display()
    );
    Ok(())
}

fn install_remote_extension(
    name: &str,
    repo_url: &str,
    sha256: Option<&str>,
) -> Result<(), String> {
    crate::install_support::validate_repo(repo_url)?;
    let work_dir = crate::install_support::temp_work_dir(EXTENSION_PREFIX, name)?;
    let temp_dir = work_dir.join("repo");

    let repo_display = repo_url.trim_end_matches(".git");
    println!("Fetching extension '{}' from {}...", name, repo_display);

    if let Err(err) = crate::install_support::clone_repository(repo_url, &temp_dir, "extension") {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(err);
    }

    // The official repository nests extension crates under
    // `extensions/extensions/`; forks may keep them at the root or one level
    // down.
    let ext_path = [
        temp_dir.join(name),
        temp_dir.join(EXTENSIONS_SUBDIR).join(name),
        temp_dir
            .join(EXTENSIONS_SUBDIR)
            .join(EXTENSIONS_SUBDIR)
            .join(name),
    ]
    .into_iter()
    .find(|path| path.is_dir());

    let Some(ext_path) = ext_path else {
        let _ = fs::remove_dir_all(&work_dir);
        return Err(format!(
            "Extension '{}' not found in repository '{}'.\n\
             Available extensions can be found in the repository root or under {}/tree/main/extensions",
            name, repo_display, repo_display
        ));
    };

    let result = build_and_install_extension(&ext_path, name, sha256);

    let _ = fs::remove_dir_all(&work_dir);
    result
}
