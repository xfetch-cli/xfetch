use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  xfetch\n  xfetch --config ~/.config/xfetch/config.jsonc\n  xfetch --gen-config\n  xfetch --gen-config --layout tree\n  xfetch --list-layouts\n  xfetch --clean-cache\n  xfetch --daemon\n  xfetch --daemon-stop\n  xfetch --no-daemon-live\n  xfetch --daemon-live-stop\n  xfetch --daemon-live-reload\n  xfetch update --check\n  xfetch update\n  xfetch plugin install animate-logo\n  xfetch plugin list --verbose\n  xfetch plugin info animate-logo\n  xfetch plugin search logo\n  xfetch plugin remove animate-logo\n  xfetch effects install decrypt\n  xfetch effects list --verbose\n  xfetch effects info decrypt\n  xfetch effects available\n  xfetch effects remove decrypt\n  xfetch extension install config-roulette\n  xfetch extension list --verbose\n  xfetch extension info config-roulette\n  xfetch extension search night\n  xfetch extension remove config-roulette\n  xfetch wasm inspect ./plugin.wasm\n  xfetch wasm run ./plugin.wasm --request '{\"version\":1,\"kind\":\"info_provider\"}'\n  xfetch wasm wit\n  xfetch theme list\n  xfetch theme set dracula\n  xfetch theme remove dracula\n  xfetch theme export my-theme"
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

    /// Layout for the generated config (e.g. `section`, `tree`, `compact`).
    /// Only used with `--gen-config`; defaults to `section`. Run
    /// `xfetch --list-layouts` to see every accepted name.
    #[arg(long, global = true)]
    pub layout: Option<String>,

    /// Print the layout names accepted by `--layout` and exit.
    #[arg(long, global = true)]
    pub list_layouts: bool,

    #[arg(long, global = true)]
    pub clean_cache: bool,

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
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    #[command(alias = "plugins")]
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    #[command(alias = "extensions")]
    Extension {
        #[command(subcommand)]
        action: ExtensionCommands,
    },
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
    #[command(alias = "effect")]
    Effects {
        #[command(subcommand)]
        action: EffectCommands,
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
        /// (Unix only).
        #[arg(long)]
        prebuilt: bool,
        /// Directory holding the xfetch binary to replace (with --prebuilt).
        #[arg(long, value_name = "DIR")]
        bin_dir: Option<String>,
        /// Assume yes for confirmation prompts.
        #[arg(long, short = 'y')]
        yes: bool,
    },
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    Install {
        path: String,
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
    },
    List {
        /// Show kind, runtime, version, capabilities and enabled state.
        #[arg(long, short)]
        verbose: bool,
    },
    Info {
        name: String,
    },
    /// Search the remote plugin catalog. Omit the query to list everything.
    #[command(alias = "available")]
    Search {
        query: Option<String>,
        /// Emit the matching entries as JSON.
        #[arg(long)]
        json: bool,
    },
    Remove {
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ExtensionCommands {
    Install {
        path: String,
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
    },
    List {
        /// Show kind, runtime, version, capabilities and enabled state.
        #[arg(long, short)]
        verbose: bool,
    },
    Info {
        name: String,
    },
    /// Search the remote extension catalog. Omit the query to list everything.
    #[command(alias = "available")]
    Search {
        query: Option<String>,
        /// Emit the matching entries as JSON.
        #[arg(long)]
        json: bool,
    },
    Remove {
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum ThemeCommands {
    List,
    Set { name: String },
    Remove { name: String },
    Export { name: String },
}

#[derive(Subcommand, Debug)]
pub enum EffectCommands {
    Install {
        path: String,
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
    },
    List {
        /// Show kind, runtime, version, capabilities and enabled state.
        #[arg(long, short)]
        verbose: bool,
    },
    Info {
        name: String,
    },
    /// Search the remote effect catalog. Omit the query to list everything.
    #[command(alias = "available")]
    Search {
        query: Option<String>,
        /// Emit the matching entries as JSON.
        #[arg(long)]
        json: bool,
    },
    Remove {
        name: String,
    },
}

#[derive(Subcommand, Debug)]
pub enum WasmCommands {
    /// Print the resolved kind, manifest, capabilities and limits of an
    /// artifact without executing it.
    Inspect {
        path: String,
        /// Emit the report as JSON instead of a human-readable summary.
        #[arg(long)]
        json: bool,
    },
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
        #[arg(long, value_parser = ["plugin", "effect", "extension"], default_value = "plugin")]
        kind: String,
    },
    /// Print the WIT package for component guests (componentize-py,
    /// componentize-js, wit-bindgen, ...).
    Wit,
}
