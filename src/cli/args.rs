use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(
    name = "xfetch",
    author,
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  xfetch\n  xfetch --json\n  xfetch --get os,cpu,memory\n  xfetch --modules os,kernel,cpu --layout compact\n  xfetch --config ~/.config/xfetch/config.jsonc\n  xfetch --gen-config\n  xfetch --gen-config --layout tree\n  xfetch --list-layouts\n  xfetch --list-modules\n  xfetch --clean-cache\n  xfetch --daemon\n  xfetch --daemon-stop\n  xfetch doctor\n  xfetch doctor --json --network\n  xfetch modules --json\n  xfetch config validate\n  xfetch config print\n  xfetch config get layout\n  xfetch config set layout right\n  xfetch completions zsh\n  xfetch man\n  xfetch addon plugin install animate-logo\n  xfetch plugin install animate-logo\n  xfetch plugin install --all\n  xfetch plugin update --all --check\n  xfetch plugin rollback animate-logo\n  xfetch plugin list --verbose\n  xfetch plugin info animate-logo\n  xfetch plugin search logo\n  xfetch plugin remove animate-logo\n  xfetch effects install decrypt\n  xfetch extension install config-roulette\n  xfetch wasm inspect ./plugin.wasm\n  xfetch wasm run ./plugin.wasm --request '{\"version\":1,\"kind\":\"info_provider\"}'\n  xfetch wasm wit\n  xfetch theme list\n  xfetch theme set dracula\n  xfetch theme remove dracula\n  xfetch theme export my-theme\n  xfetch update --check\n  xfetch update"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[arg(long, global = true)]
    pub gen_config: bool,

    /// Logo to embed in the generated config, overriding the detected
    /// OS/distro (e.g. `--logo arch`, `--logo windows-11`). Only used with
    /// `--gen-config`; requires network access to the logos catalog.
    #[arg(long, global = true)]
    pub logo: Option<String>,

    /// Layout name. Overrides the configured layout at runtime; with
    /// `--gen-config` it selects the generated layout instead.
    #[arg(long, global = true)]
    pub layout: Option<String>,

    /// Theme name. Overrides the configured theme at runtime.
    #[arg(long, global = true)]
    pub theme: Option<String>,

    /// Print the layout names accepted by `--layout` and exit.
    #[arg(long, global = true)]
    pub list_layouts: bool,

    /// Print the module keys accepted in the config and exit.
    #[arg(long, global = true)]
    pub list_modules: bool,

    #[arg(long, global = true)]
    pub clean_cache: bool,

    /// Back up the config currently in use, including its theme and
    /// config-roulette routes.
    #[arg(long, global = true)]
    pub backup: bool,

    /// Restore the most recent config backup.
    #[arg(long, global = true)]
    pub restore: bool,

    #[arg(long, global = true)]
    pub benchmark: bool,

    #[arg(long, global = true)]
    pub daemon: bool,

    #[arg(long, global = true)]
    pub daemon_stop: bool,

    /// Disable the live stats daemon (`daemon_live` in config) from the
    /// terminal.
    #[arg(long, global = true)]
    pub no_daemon_live: bool,

    /// Stop the running live stats daemon.
    #[arg(long, global = true)]
    pub daemon_live_stop: bool,

    /// Force hot reload in the live stats daemon (equivalent to
    /// `"daemon_live_reload": true` in the config).
    #[arg(long, global = true)]
    pub daemon_live_reload: bool,

    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    pub json: bool,

    /// Print only specific module keys. Repeat the flag or use commas:
    /// `--get os,cpu --get memory`.
    #[arg(long = "get", global = true, value_delimiter = ',', action = clap::ArgAction::Append)]
    pub get: Vec<String>,

    /// Override the rendered module list with a comma-separated key list.
    #[arg(long = "modules", alias = "only", global = true)]
    pub modules: Option<String>,

    /// Control color output.
    #[arg(long, global = true, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,

    /// Disable colors; same as `--color never`.
    #[arg(long, global = true)]
    pub no_color: bool,

    /// Disable network access (public IP, catalogs, update checks, logo fetch).
    #[arg(long, global = true)]
    pub offline: bool,

    /// Bypass the package/public-IP cache for this run.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Suppress informational output.
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Print extra diagnostics.
    #[arg(long, global = true)]
    pub verbose: bool,

    /// Log level: off, error, warn, info, debug or trace. Overrides
    /// --quiet/--verbose.
    #[arg(long, global = true, value_enum)]
    pub log_level: Option<LogLevel>,

    /// Emit logs as JSON lines on stderr.
    #[arg(long, global = true)]
    pub log_json: bool,

    /// Render the fetched modules with a template file (`{{key}}`).
    #[arg(long, global = true)]
    pub template: Option<String>,

    /// Render the fetched modules with an inline template (`{{key}}`).
    #[arg(long, global = true)]
    pub format: Option<String>,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ColorChoice {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LogLevel {
    Off,
    Error,
    Warn,
    #[default]
    Info,
    Debug,
    Trace,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SchemaKind {
    Fetch,
    Backup,
    Addon,
    Update,
    Doctor,
    Config,
    #[default]
    All,
}

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddonKindArg {
    Plugin,
    Effect,
    Extension,
    Command,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Manage plugins.
    #[command(alias = "plugins")]
    Plugin {
        #[command(subcommand)]
        action: AddonCommands,
    },
    /// Manage config-provider extensions.
    #[command(alias = "extensions")]
    Extension {
        #[command(subcommand)]
        action: AddonCommands,
    },
    /// Manage CLI command addons.
    #[command(alias = "commands")]
    Command {
        #[command(subcommand)]
        action: AddonCommands,
    },
    /// Run an installed command addon.
    Run {
        /// Installed command name.
        name: String,
        /// Render the output through the core layout and logo.
        #[arg(long)]
        render: bool,
        /// Arguments passed to the command.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Manage local and remote themes.
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
    /// Browse and preview the ASCII logo catalog.
    Logo {
        #[command(subcommand)]
        action: LogoCommands,
    },
    /// Manage intro effects.
    #[command(alias = "effect")]
    Effects {
        #[command(subcommand)]
        action: AddonCommands,
    },
    /// Manage any addon kind with one command family.
    Addon {
        #[arg(value_enum)]
        kind: AddonKindArg,
        #[command(subcommand)]
        action: AddonCommands,
    },
    /// WebAssembly guest tooling: inspect artifacts, run them with a raw JSON
    /// request and print the component protocol.
    Wasm {
        #[command(subcommand)]
        action: WasmCommands,
    },
    /// Check for a newer xfetch release and install it.
    Update {
        /// Only check; exit with code 1 when a newer release exists.
        #[arg(long)]
        check: bool,
        /// Install the prebuilt release in place, even for local builds
        /// (Unix and Windows).
        #[arg(long)]
        prebuilt: bool,
        /// Directory holding the xfetch binary to replace (with --prebuilt).
        #[arg(long, value_name = "DIR")]
        bin_dir: Option<String>,
        /// Assume yes for confirmation prompts.
        #[arg(long, short = 'y')]
        yes: bool,
        /// Restore the previous binary from the update backup.
        #[arg(long)]
        rollback: bool,
    },
    /// Diagnose the installation, configuration, terminal and addons.
    Doctor {
        /// Also check remote catalog and update endpoints.
        #[arg(long)]
        network: bool,
        /// Apply safe automatic fixes for common problems.
        #[arg(long)]
        fix: bool,
        /// Run the info collection benchmark and include it in the report.
        #[arg(long)]
        bench: bool,
        /// Print a preview of the configured logo/image in a TTY.
        #[arg(long)]
        image: bool,
    },
    /// List the module keys accepted in the config.
    Modules,
    /// Inspect or edit the configuration.
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },
    /// Generate shell completions.
    Completions {
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Render the man page.
    Man {
        /// Write the page to this path instead of stdout.
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },
    /// Print the JSON Schema for the CLI machine-readable outputs.
    Schema {
        #[arg(value_enum, default_value_t = SchemaKind::All)]
        kind: SchemaKind,
    },
    /// Print the resolved config, addon, cache and data paths.
    Paths,
    /// Print the platform, terminal, environment and runtime settings.
    Env,
    /// Print build information and enabled features.
    #[command(name = "build-info")]
    BuildInfo,
    /// Remove the xfetch binary and optionally its config, cache and data.
    #[command(name = "self-uninstall")]
    SelfUninstall {
        /// Also remove config, cache and data.
        #[arg(long)]
        purge: bool,
        /// Skip the confirmation prompt.
        #[arg(long, short)]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum AddonCommands {
    /// Install an addon from a catalog name, local path, repository, wasm file or URL.
    Install {
        path: Option<String>,
        #[arg(long, short)]
        repo: Option<String>,
        /// Expected SHA-256 (hex) of a downloaded wasm artifact.
        #[arg(long, value_name = "HEX")]
        sha256: Option<String>,
        /// Override the installed name for a direct wasm file or URL.
        #[arg(long)]
        name: Option<String>,
        /// Sidecar manifest to attach to a direct wasm file or URL.
        #[arg(long, value_name = "FILE")]
        manifest: Option<String>,
        /// Install every addon available in the catalog.
        #[arg(long)]
        all: bool,
        /// Reinstall addons that are already installed (with --all).
        #[arg(long)]
        force: bool,
    },
    /// Update an installed addon, or every installed addon with --all.
    Update {
        name: Option<String>,
        /// Update every installed addon.
        #[arg(long)]
        all: bool,
        /// Only check; exit with code 1 when an update is available.
        #[arg(long)]
        check: bool,
        /// Reinstall even when the catalog version is not newer.
        #[arg(long)]
        force: bool,
    },
    /// Restore the previous artifact saved by the last update.
    Rollback { name: String },
    /// List installed addons.
    List {
        /// Show kind, runtime, version, capabilities and enabled state.
        #[arg(long, short)]
        verbose: bool,
    },
    /// Show one installed addon and its configuration snippet.
    Info { name: String },
    /// Search the remote catalog. Omit the query to list everything.
    #[command(alias = "available")]
    Search { query: Option<String> },
    /// Remove an installed addon.
    Remove { name: String },
}

#[derive(Subcommand, Debug)]
pub enum ThemeCommands {
    /// List installed themes.
    List,
    /// Search the remote theme registry.
    Search { query: Option<String> },
    /// Show metadata for one remote theme.
    Info { name: String },
    /// Install a theme from the remote registry.
    Install {
        name: String,
        /// Reinstall an already installed theme.
        #[arg(long)]
        force: bool,
    },
    /// Update installed themes from the remote registry.
    Update {
        name: Option<String>,
        /// Update every installed theme present in the registry.
        #[arg(long)]
        all: bool,
        /// Only report; exit 1 when an update is available.
        #[arg(long)]
        check: bool,
        /// Reinstall even when the registry version is not newer.
        #[arg(long)]
        force: bool,
    },
    /// Set the active theme in the config.
    Set { name: String },
    /// Remove an installed theme.
    Remove { name: String },
    /// Export the current visual config as a reusable theme.
    Export { name: String },
}

#[derive(Subcommand, Debug)]
pub enum LogoCommands {
    /// List logos from the remote catalog.
    List { query: Option<String> },
    /// Print one logo from the remote catalog.
    Preview { name: String },
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Parse the config file and validate its schema.
    Validate,
    /// Print the resolved config (defaults, theme, extensions and overrides).
    Print {
        /// Print the config file as stored instead of the resolved config.
        #[arg(long)]
        raw: bool,
    },
    /// Print one value by dotted key path.
    Get { key: String },
    /// Set one value. Dotted paths and array indices are supported.
    Set { key: String, value: String },
    /// Remove one value by dotted key path.
    Unset { key: String },
    /// Replace the config with the built-in defaults after a safety backup.
    Reset {
        /// Skip the confirmation prompt.
        #[arg(long, short)]
        yes: bool,
    },
    /// Open the config in $VISUAL or $EDITOR after a safety backup.
    Edit,
    /// List saved config backups.
    Backups,
    /// Delete old backups, keeping the newest N.
    Prune {
        /// Number of backups to keep.
        #[arg(long, default_value_t = 5)]
        keep: usize,
        /// Print what would be deleted without removing files.
        #[arg(long)]
        dry_run: bool,
    },
    /// Back up the config currently in use.
    Backup {
        /// Write to this file or directory instead of the default backup directory.
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
        /// Include installed addon artifacts, manifests and receipts.
        #[arg(long)]
        with_addons: bool,
        /// Compress the backup with gzip.
        #[arg(long)]
        compress: bool,
        /// Encrypt the backup with a passphrase.
        #[arg(long)]
        encrypt: bool,
        /// Read the backup passphrase from this file instead of
        /// XFETCH_BACKUP_PASSPHRASE.
        #[arg(long, value_name = "PATH")]
        passphrase_file: Option<String>,
    },
    /// Restore a config backup.
    Restore {
        /// Backup file (or directory) to restore. Defaults to the latest backup.
        #[arg(long, value_name = "PATH")]
        from: Option<String>,
        /// Print what would be restored without writing files.
        #[arg(long)]
        dry_run: bool,
        /// Remap files under the original config directory into this root.
        #[arg(long, value_name = "DIR")]
        root: Option<String>,
        /// Read the backup passphrase from this file instead of
        /// XFETCH_BACKUP_PASSPHRASE.
        #[arg(long, value_name = "PATH")]
        passphrase_file: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
pub enum WasmCommands {
    /// Print the resolved kind, manifest, capabilities and limits of an
    /// artifact without executing it.
    Inspect { path: String },
    /// Execute a wasm guest with a raw JSON request and print its response.
    /// Useful for authors and for scripted end-to-end checks.
    Run {
        path: String,
        /// JSON request passed to the guest (defaults to `{}`).
        #[arg(long)]
        request: Option<String>,
        /// Read the JSON request from a file instead of `--request`.
        #[arg(long, value_name = "PATH")]
        request_file: Option<String>,
        /// Safety timeout in seconds; overrides the manifest limit.
        #[arg(long)]
        timeout: Option<u64>,
        /// Guest contract used to select the component world.
        #[arg(long, value_parser = ["plugin", "effect", "extension", "command"], default_value = "plugin")]
        kind: String,
    },
    /// Print the WIT package for component guests (componentize-py,
    /// componentize-js, wit-bindgen, ...).
    Wit,
}

#[cfg(test)]
mod help_tests {
    use super::*;
    use clap::CommandFactory;

    #[test]
    fn all_subcommands_have_descriptions() {
        fn walk(command: &clap::Command, path: &str) {
            for subcommand in command.get_subcommands() {
                if subcommand.get_name() == "help" {
                    continue;
                }
                assert!(
                    subcommand.get_about().is_some(),
                    "{path} {} is missing a help description",
                    subcommand.get_name()
                );
                walk(subcommand, &format!("{path} {}", subcommand.get_name()));
            }
        }
        walk(&Cli::command(), "xfetch");
    }
}
