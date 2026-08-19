//! Effects: installable intro animations for the content lines.
//!
//! The core renders the info lines, hands them to an effect binary
//! (`xfetch-effect-<name>`, protocol from `xfetch-effect-api`), and plays the
//! returned frames before settling on the final content. Effects are opt-in:
//! a missing binary just skips the effect (no behavior change).
//!
//! Binaries are resolved like plugins: `~/.config/xfetch/effects/` and PATH.
//! `xfetch effects install` builds and installs them from the
//! `xfetch-cli/effects` repository.

pub mod install;
pub mod manage;

use crate::config::{EffectConfig, config_dir, config_search_dirs};
use crate::subprocess::run_cmd_with_stdin_timeout;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Duration;
use xfetch_effect_api::{
    EffectArgs, EffectFrame, EffectRequest, EffectResponse, parse_json_slice, to_json_vec,
};

pub use install::install_effect;
pub use manage::{list_effects, remove_effect};

const EFFECT_PREFIX: &str = "xfetch-effect-";
const EXE_EXT: &str = ".exe";
const CONFIG_DIR_NAME: &str = "xfetch";
const TARGET_RELEASE: &str = "target/release";
const CARGO_TOML: &str = "Cargo.toml";
const CARGO_CMD: &str = "cargo";
const GIT_CMD: &str = "git";
const ENV_CARGO_NET_GIT_FETCH_WITH_CLI: &str = "CARGO_NET_GIT_FETCH_WITH_CLI";
const ENV_EFFECT_REPO: &str = "XFETCH_EFFECT_REPO";

pub const DEFAULT_EFFECT_REPO: &str = "https://github.com/xfetch-cli/effects.git";

pub fn default_effect_dir() -> PathBuf {
    config_dir().join(CONFIG_DIR_NAME).join("effects")
}

pub fn default_effect_repo() -> String {
    env::var(ENV_EFFECT_REPO).unwrap_or_else(|_| DEFAULT_EFFECT_REPO.to_string())
}

pub fn effect_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}{}", EFFECT_PREFIX, name, EXE_EXT)
    } else {
        format!("{}{}", EFFECT_PREFIX, name)
    }
}

fn extract_effect_name(path: &Path) -> Option<String> {
    let filename = path.file_name()?.to_str()?;
    if let Some(name) = filename.strip_prefix(EFFECT_PREFIX) {
        if cfg!(target_os = "windows") {
            name.strip_suffix(EXE_EXT).map(|n| n.to_string())
        } else {
            Some(name.to_string())
        }
    } else {
        None
    }
}

fn find_in_path(binary_name: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&path_var) {
        let candidate = dir.join(binary_name);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// Locates an installed effect binary (`xfetch-effect-<name>`) in the config
/// dir (`xfetch/effects/`, falling back to `xfetch/plugins/`) and PATH.
pub fn find_effect_binary(name: &str) -> Option<PathBuf> {
    let binary_name = effect_binary_name(name);

    if let Some(path) = find_in_path(&binary_name) {
        return Some(path);
    }

    for config_dir in config_search_dirs() {
        let candidates = [
            config_dir.join("xfetch").join("effects").join(&binary_name),
            config_dir.join("xfetch").join("plugins").join(&binary_name),
        ];
        for candidate in candidates {
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

/// Runs the configured effect over `lines` and returns its frames.
/// `Err` (missing binary, bad response, timeout) → callers skip the effect.
pub fn run_effect(config: &EffectConfig, lines: &[String]) -> Result<Vec<EffectFrame>, String> {
    let plugin = config
        .plugin
        .as_deref()
        .ok_or_else(|| "Effect has no plugin name".to_string())?;
    let path = find_effect_binary(plugin).ok_or_else(|| format!("Effect not found: {}", plugin))?;

    let request = EffectRequest::new(
        lines.to_vec(),
        EffectArgs {
            style: config.style.clone(),
            duration_ms: config.duration_ms,
            fps: config.fps,
            args: config.args.clone(),
        },
    );
    let payload = to_json_vec(&request)
        .map_err(|err| format!("Failed to serialize effect request: {}", err))?;

    let timeout = config.timeout_secs.map(Duration::from_secs);
    let output = run_cmd_with_stdin_timeout(&path, &[], Some(&payload), timeout).ok_or_else(
        || match timeout {
            Some(d) => format!(
                "Effect '{}' exceeded its timeout of {}s",
                plugin,
                d.as_secs()
            ),
            None => format!("Failed to run effect '{}'", plugin),
        },
    )?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(if stderr.trim().is_empty() {
            format!("Effect '{}' exited with error", plugin)
        } else {
            stderr.trim().to_string()
        });
    }

    let response: EffectResponse = parse_json_slice(&output.stdout)
        .map_err(|err| format!("Failed to parse effect output: {}", err))?;
    response
        .validate()
        .map_err(|err| format!("Invalid effect response: {}", err))?;

    Ok(response.frames)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_effect_binary_name() {
        let name = effect_binary_name("decrypt");
        if cfg!(target_os = "windows") {
            assert_eq!(name, "xfetch-effect-decrypt.exe");
        } else {
            assert_eq!(name, "xfetch-effect-decrypt");
        }
    }

    #[test]
    fn test_find_missing_effect_returns_none() {
        assert!(find_effect_binary("definitely_not_an_effect_xyz").is_none());
    }

    #[test]
    fn test_run_effect_missing_binary_errors() {
        let config = EffectConfig {
            plugin: Some("definitely_not_an_effect_xyz".to_string()),
            ..EffectConfig::default()
        };
        assert!(run_effect(&config, &["a".to_string()]).is_err());
    }
}
