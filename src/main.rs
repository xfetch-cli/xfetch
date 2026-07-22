mod cache;
mod cli;
mod config;
mod info;
mod plugins;
mod ui;

use crate::config::{generate_config, load_config};
use crate::info::Info;
use crate::plugins::{install_plugin, list_plugins, remove_plugin};
use crate::ui::draw;
use clap::Parser;
use cli::{Cli, Commands, PluginCommands};

fn main() {
    let cli = Cli::parse();

    if cli.clean_cache {
        match cache::clean() {
            Ok(()) => {
                println!("Cache cleaned.");
                return;
            }
            Err(err) => {
                eprintln!("Failed to clean cache: {}", err);
                std::process::exit(1);
            }
        }
    }

    if cli.gen_config {
        match generate_config(cli.config.clone()) {
            Ok(path) => {
                println!("Generated config: {}", path.display());
                println!("Run xfetch to see the new layout.");
                return;
            }
            Err(err) => {
                eprintln!("Failed to generate config: {}", err);
                std::process::exit(1);
            }
        }
    }

    match cli.command {
        Some(Commands::Plugin { action }) => match action {
            PluginCommands::Install { path, repo } => {
                match install_plugin(&path, repo.as_deref()) {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            PluginCommands::List => match list_plugins() {
                Ok(plugins) => {
                    if plugins.is_empty() {
                        println!("No plugins installed.");
                        println!(
                            "Plugin directory: {}",
                            plugins::default_plugin_dir().display()
                        );
                    } else {
                        println!("Installed plugins:");
                        for (name, path) in &plugins {
                            println!("  {}  ({})", name, path.display());
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
            PluginCommands::Remove { name } => match remove_plugin(&name) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
        },
        None => {
            let config = load_config(cli.config);
            let info = Info::with_config(&config);
            draw(&info, &config);
        }
    }
}
