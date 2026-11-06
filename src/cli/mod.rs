pub mod addon_cmd;
pub mod args;
pub mod backup_cmd;
pub mod build_info_cmd;
pub mod completions;
pub mod config_cmd;
pub mod dispatch;
pub mod doctor;
pub mod env_cmd;
pub mod fetch;
pub mod jsonc_edit;
pub mod logo_cmd;
pub mod modules;
pub mod output;
pub mod overrides;
pub mod paths_cmd;
pub mod run_cmd;
pub mod schema_cmd;
pub mod settings;
pub mod template;
pub mod theme_cmd;
pub mod uninstall_cmd;
pub mod update_cmd;
pub mod wasm_cmd;

pub use args::Cli;
pub use dispatch::run;

use crate::config::{Config, ConfigOverrides, load_config_with_overrides};

/// Per-invocation CLI state shared by the command handlers.
pub struct Context {
    pub config_path: Option<String>,
    pub overrides: ConfigOverrides,
}

impl Context {
    pub fn load_config(&self) -> Config {
        load_config_with_overrides(self.config_path.clone(), &self.overrides)
    }
}
