use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about,
    long_about = None,
    after_help = "Examples:\n  xfetch\n  xfetch --config ~/.config/xfetch/config.jsonc\n  xfetch --gen-config\n  xfetch plugin install animate-logo\n  xfetch plugin list\n  xfetch plugin remove animate-logo"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(short, long, global = true)]
    pub config: Option<String>,

    #[arg(long, global = true)]
    pub gen_config: bool,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Plugin {
        #[command(subcommand)]
        action: PluginCommands,
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
