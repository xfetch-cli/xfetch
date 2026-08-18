use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  xfetch\n  xfetch --config ~/.config/xfetch/config.jsonc\n  xfetch --gen-config\n  xfetch --clean-cache\n  xfetch --daemon\n  xfetch --daemon-stop\n  xfetch plugin install animate-logo\n  xfetch plugin list\n  xfetch plugin remove animate-logo\n  xfetch extension install config-roulette\n  xfetch extension list\n  xfetch extension remove config-roulette\n  xfetch theme list\n  xfetch theme set dracula\n  xfetch theme remove dracula\n  xfetch theme export my-theme"
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
    /// Only used with `--gen-config`; defaults to `pacman`.
    #[arg(long, global = true)]
    pub layout: Option<String>,

    #[arg(long, global = true)]
    pub clean_cache: bool,

    #[arg(long, global = true)]
    pub benchmark: bool,

    #[arg(long, global = true)]
    pub daemon: bool,

    #[arg(long, global = true)]
    pub daemon_stop: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
    },
    Extension {
        #[command(subcommand)]
        action: ExtensionCommands,
    },
    Theme {
        #[command(subcommand)]
        action: ThemeCommands,
    },
}

#[derive(Subcommand, Debug)]
pub enum PluginCommands {
    Install {
        path: String,
        #[arg(long, short)]
        repo: Option<String>,
    },
    List,
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
    },
    List,
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
