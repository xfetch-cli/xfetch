pub mod install;
pub mod manage;
pub mod run;

use std::env;
use std::path::{Path, PathBuf};

pub use install::install_plugin;
pub use manage::{list_plugins, remove_plugin};
pub use run::{run_info_plugin, run_logo_animation_plugin};
pub use xfetch_plugin_api::AnimationFrame;

const PLUGIN_PREFIX: &str = "xfetch-plugin-";
const EXE_EXT: &str = ".exe";
const LEGACY_PLUGINS_DIR: &str = "plugins";
const SIBLING_PLUGIN_REPO_DIR: &str = "plugins";
const GROUPED_PLUGIN_DIR: &str = "plugins";
const CONFIG_DIR_NAME: &str = "xfetch";
const TARGET_RELEASE: &str = "target/release";
const CARGO_TOML: &str = "Cargo.toml";
const CARGO_CMD: &str = "cargo";
const GIT_CMD: &str = "git";
const ENV_PATH: &str = "PATH";
const ENV_PLUGIN_REPO: &str = "XFETCH_PLUGIN_REPO";
const ENV_PLUGIN_DEV_DIR: &str = "XFETCH_PLUGIN_DEV_DIR";

pub const DEFAULT_PLUGIN_REPO: &str = "https://github.com/xfetch-cli/plugins.git";

pub fn default_plugin_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join(CONFIG_DIR_NAME).join(LEGACY_PLUGINS_DIR)
}

pub fn default_plugin_repo() -> String {
    env::var(ENV_PLUGIN_REPO).unwrap_or_else(|_| DEFAULT_PLUGIN_REPO.to_string())
}

pub fn plugin_binary_name(plugin_name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}{}", PLUGIN_PREFIX, plugin_name, EXE_EXT)
    } else {
        format!("{}{}", PLUGIN_PREFIX, plugin_name)
    }
}

fn extract_plugin_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    if let Some(name) = filename.strip_prefix(PLUGIN_PREFIX) {
        if cfg!(target_os = "windows") {
            name.strip_suffix(EXE_EXT).map(|n| n.to_string())
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

fn candidate_plugin_dirs_from(base_dir: &Path, plugin_name: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();

    if let Ok(custom_dir) = env::var(ENV_PLUGIN_DEV_DIR) {
        let custom_dir = PathBuf::from(custom_dir);
        candidates.push(custom_dir.join(plugin_name));
        candidates.push(custom_dir.join(LEGACY_PLUGINS_DIR).join(plugin_name));
        candidates.push(
            custom_dir
                .join(SIBLING_PLUGIN_REPO_DIR)
                .join(GROUPED_PLUGIN_DIR)
                .join(plugin_name),
        );
    }

    candidates.push(base_dir.join(plugin_name));
    candidates.push(base_dir.join(LEGACY_PLUGINS_DIR).join(plugin_name));
    candidates.push(base_dir.join(SIBLING_PLUGIN_REPO_DIR).join(plugin_name));
    candidates.push(
        base_dir
            .join(SIBLING_PLUGIN_REPO_DIR)
            .join(GROUPED_PLUGIN_DIR)
            .join(plugin_name),
    );

    if let Some(parent) = base_dir.parent() {
        candidates.push(parent.join(SIBLING_PLUGIN_REPO_DIR).join(plugin_name));
        candidates.push(
            parent
                .join(SIBLING_PLUGIN_REPO_DIR)
                .join(GROUPED_PLUGIN_DIR)
                .join(plugin_name),
        );
    }

    candidates.dedup();
    candidates
}

fn candidate_plugin_binary_paths_from(plugin_dir: &Path, binary_name: &str) -> Vec<PathBuf> {
    let mut candidates = vec![plugin_dir.join(TARGET_RELEASE).join(binary_name), plugin_dir.join(binary_name)];

    let mut current = plugin_dir.parent();
    while let Some(dir) = current {
        candidates.push(dir.join(TARGET_RELEASE).join(binary_name));
        current = dir.parent();
    }

    candidates.dedup();

    candidates
}

fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os(ENV_PATH)?;
    for dir in env::split_paths(&path_var) {
        let candidate = dir.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

fn find_plugin_binary(plugin_name: &str) -> Option<PathBuf> {
    let direct_path = Path::new(plugin_name);
    if direct_path.components().count() > 1 && direct_path.is_file() {
        return Some(direct_path.to_path_buf());
    }

    let binary_name = plugin_binary_name(plugin_name);

    if let Some(path) = find_in_path(&binary_name) {
        return Some(path);
    }

    let config_dir = default_plugin_dir();
    let in_config_dir = config_dir.join(&binary_name);
    if in_config_dir.is_file() {
        return Some(in_config_dir);
    }

    if let Ok(cwd) = env::current_dir() {
        for plugin_dir in candidate_plugin_dirs_from(&cwd, plugin_name) {
            for candidate in candidate_plugin_binary_paths_from(&plugin_dir, &binary_name) {
                if candidate.is_file() {
                    return Some(candidate);
                }
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
    fn test_extract_plugin_name_linux() {
        let path = Path::new("/usr/lib/xfetch/plugins/xfetch-plugin-hello");
        assert_eq!(extract_plugin_name(path), Some("hello".to_string()));
    }

    #[test]
    fn test_extract_plugin_name_with_exe() {
        let path = Path::new("xfetch-plugin-hello.exe");
        let result = extract_plugin_name(path);
        if cfg!(target_os = "windows") {
            assert_eq!(result, Some("hello".to_string()));
        } else {
            assert_eq!(result, Some("hello.exe".to_string()));
        }
    }

    #[test]
    fn test_extract_plugin_name_no_match() {
        let path = Path::new("/usr/bin/something-else");
        assert_eq!(extract_plugin_name(path), None);
    }

    #[test]
    fn test_extract_plugin_name_invalid_utf8() {
        let path = Path::new("/tmp/random-file");
        assert_eq!(extract_plugin_name(path), None);
    }

    #[test]
    fn test_plugin_binary_name() {
        let name = plugin_binary_name("test");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "xfetch-plugin-test.exe");
        } else {
            assert_eq!(name, "xfetch-plugin-test");
        }
    }

    #[test]
    fn test_candidate_plugin_dirs_include_sibling_repo() {
        let base = Path::new("/workspace/xfetch");
        let candidates = candidate_plugin_dirs_from(base, "animate-logo");

        assert!(candidates.contains(&PathBuf::from(
            "/workspace/xfetch/plugins/plugins/animate-logo"
        )));
    }

    #[test]
    fn test_candidate_plugin_dirs_include_parent_sibling_repo() {
        let base = Path::new("/workspace/xfetch/core");
        let candidates = candidate_plugin_dirs_from(base, "animate-logo");

        assert!(candidates.contains(&PathBuf::from(
            "/workspace/xfetch/plugins/plugins/animate-logo"
        )));
    }

    #[test]
    fn test_candidate_binary_paths_include_workspace_target() {
        let plugin_dir = Path::new("/workspace/xfetch/plugins/plugins/animate-logo");
        let candidates = candidate_plugin_binary_paths_from(
            plugin_dir,
            "xfetch-plugin-animate-logo",
        );

        assert!(candidates.contains(&PathBuf::from(
            "/workspace/xfetch/plugins/plugins/animate-logo/target/release/xfetch-plugin-animate-logo"
        )));
        assert!(candidates.contains(&PathBuf::from(
            "/workspace/xfetch/plugins/target/release/xfetch-plugin-animate-logo"
        )));
    }
}
