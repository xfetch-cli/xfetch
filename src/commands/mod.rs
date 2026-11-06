//! Official command addons: installable CLI commands.
//!
//! Command addons are WebAssembly (or native) guests that receive a JSON
//! request with positional arguments and return plain text. The core prints
//! the text as-is unless the user requests a layout.

pub mod install;
pub mod manage;
pub mod run;

use std::env;
use std::path::PathBuf;

use crate::config::config_dir;

pub use install::install_command;
pub use manage::{find_command_binary, list_commands, remove_command};
pub use run::run_command;

const COMMAND_PREFIX: &str = "xfetch-command-";
const EXE_EXT: &str = ".exe";
const CONFIG_DIR_NAME: &str = "xfetch";
const COMMAND_DIR_NAME: &str = "commands";
const ENV_COMMAND_REPO: &str = "XFETCH_COMMAND_REPO";

/// Default repository for official command addons.
pub const DEFAULT_COMMAND_REPO: &str = "https://github.com/xfetch-cli/commands.git";

pub fn default_command_dir() -> PathBuf {
    config_dir().join(CONFIG_DIR_NAME).join(COMMAND_DIR_NAME)
}

pub fn default_command_repo() -> String {
    env::var(ENV_COMMAND_REPO).unwrap_or_else(|_| DEFAULT_COMMAND_REPO.to_string())
}

/// Native executable name for `name`.
pub fn command_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{COMMAND_PREFIX}{name}{EXE_EXT}")
    } else {
        format!("{COMMAND_PREFIX}{name}")
    }
}

/// Installed wasm artifact name: `xfetch-command-<name>.wasm`.
pub fn command_wasm_name(name: &str) -> String {
    format!("{}.wasm", command_binary_name(name))
}

/// Sidecar manifest name next to the wasm artifact.
pub fn command_manifest_name(name: &str) -> String {
    format!("{}.json", command_binary_name(name))
}
