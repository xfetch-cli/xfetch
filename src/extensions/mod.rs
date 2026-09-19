pub mod install;
pub mod manage;
mod runner;
mod types;

use std::env;
use std::path::{Path, PathBuf};

use crate::config::{config_dir, config_search_dirs};

pub use install::install_extension;
pub use manage::{list_extensions, remove_extension};
pub use runner::run_config_provider;

const EXTENSION_PREFIX: &str = "xfetch-extension-";
pub const DEFAULT_EXTENSION_REPO: &str = "https://github.com/xfetch-cli/extensions.git";

const EXE_EXT: &str = ".exe";

pub fn default_extension_dir() -> PathBuf {
    config_dir().join("xfetch").join("extensions")
}

pub fn extension_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}{}", EXTENSION_PREFIX, name, EXE_EXT)
    } else {
        format!("{}{}", EXTENSION_PREFIX, name)
    }
}

/// Installed wasm artifact name: `xfetch-extension-<name>.wasm`.
pub fn extension_wasm_name(name: &str) -> String {
    format!("{}.wasm", extension_binary_name(name))
}

/// Sidecar manifest name next to the wasm artifact.
pub fn extension_manifest_name(name: &str) -> String {
    format!("{}.json", extension_binary_name(name))
}

fn extract_extension_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    let name = filename.strip_prefix(EXTENSION_PREFIX)?;
    // Sidecar manifests are not extensions.
    if name.ends_with(".json") {
        return None;
    }
    let name = name.strip_suffix(".wasm").unwrap_or(name);
    if cfg!(target_os = "windows") {
        name.strip_suffix(EXE_EXT).map(|n| n.to_string())
    } else {
        Some(name.to_string())
    }
}

pub fn find_extension_binary(name: &str) -> Option<PathBuf> {
    let names = [extension_binary_name(name), extension_wasm_name(name)];

    for config_dir in config_search_dirs() {
        let xfetch_dir = config_dir.join("xfetch");

        for binary_name in &names {
            let in_extensions = xfetch_dir.join("extensions").join(binary_name);
            if in_extensions.is_file() {
                return Some(in_extensions);
            }

            let in_plugins = xfetch_dir.join("plugins").join(binary_name);
            if in_plugins.is_file() {
                return Some(in_plugins);
            }
        }
    }

    if let Ok(path) = env::var("PATH") {
        for dir in env::split_paths(&path) {
            for binary_name in &names {
                let candidate = dir.join(binary_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
    }

    // Development-time wasm targets in sibling source directories.
    if let Ok(cwd) = env::current_dir()
        && let Some(candidate) = candidate_extension_wasm_from(&cwd, name)
    {
        return Some(candidate);
    }

    None
}

/// Looks for a built extension wasm artifact in conventional development
/// locations (`target/wasm32-wasip1/release/xfetch-extension-<name>.wasm`)
/// while walking up from the working directory.
fn candidate_extension_wasm_from(base: &Path, name: &str) -> Option<PathBuf> {
    let binary_name = extension_wasm_name(name);
    let mut current = Some(base);
    while let Some(dir) = current {
        for sub in ["extensions", "extensions/extensions"] {
            for target in ["wasm32-wasip1", "wasm32-wasi"] {
                let candidate = dir
                    .join(sub)
                    .join(format!("target/{}/release", target))
                    .join(&binary_name);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }
        current = dir.parent();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[cfg(unix)]
    #[test]
    fn test_extract_extension_name_linux() {
        let path = Path::new("/usr/lib/xfetch/extensions/xfetch-extension-foo");
        assert_eq!(extract_extension_name(path), Some("foo".to_string()));
    }

    #[test]
    fn test_extract_extension_name_no_match() {
        let path = Path::new("/usr/bin/something-else");
        assert_eq!(extract_extension_name(path), None);
    }

    #[test]
    fn test_extension_binary_name() {
        let name = extension_binary_name("test");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "xfetch-extension-test.exe");
        } else {
            assert_eq!(name, "xfetch-extension-test");
        }
    }
}
