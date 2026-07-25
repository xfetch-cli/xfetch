pub mod install;
pub mod manage;
mod runner;
mod types;

use std::env;
use std::path::{Path, PathBuf};

pub use install::install_extension;
pub use manage::{list_extensions, remove_extension};
pub use runner::run_config_provider;

const EXTENSION_PREFIX: &str = "xfetch-extension-";
pub const DEFAULT_EXTENSION_REPO: &str = "https://github.com/xfetch-cli/extensions.git";

const EXE_EXT: &str = ".exe";

pub fn default_extension_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("xfetch").join("extensions")
}

pub fn extension_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}{}", EXTENSION_PREFIX, name, EXE_EXT)
    } else {
        format!("{}{}", EXTENSION_PREFIX, name)
    }
}

fn extract_extension_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    if let Some(name) = filename.strip_prefix(EXTENSION_PREFIX) {
        if cfg!(target_os = "windows") {
            name.strip_suffix(EXE_EXT).map(|n| n.to_string())
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

pub fn find_extension_binary(name: &str) -> Option<PathBuf> {
    let binary_name = extension_binary_name(name);
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let xfetch_dir = config_dir.join("xfetch");

    let in_extensions = xfetch_dir.join("extensions").join(&binary_name);
    if in_extensions.is_file() {
        return Some(in_extensions);
    }

    let in_plugins = xfetch_dir.join("plugins").join(&binary_name);
    if in_plugins.is_file() {
        return Some(in_plugins);
    }

    if let Ok(path) = env::var("PATH") {
        for dir in env::split_paths(&path) {
            let candidate = dir.join(&binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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
