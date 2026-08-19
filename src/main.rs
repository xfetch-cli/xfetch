mod cache;
mod cli;
mod config;
mod effects;
mod extensions;
mod info;
mod logos;
mod plugins;
mod subprocess;
mod themes;
mod ui;

use crate::config::{generate_config, load_config};
use crate::effects::{install_effect, list_effects, remove_effect};
use crate::extensions::{install_extension, list_extensions, remove_extension};
use crate::info::Info;
use crate::plugins::{install_plugin, list_plugins, remove_plugin};
use crate::themes::{export_current_theme, list_themes, remove_theme, set_active_theme};
use crate::ui::draw;
use clap::Parser;
use cli::{Cli, Commands, EffectCommands, ExtensionCommands, PluginCommands, ThemeCommands};
use std::path::PathBuf;

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

    if cli.daemon_stop {
        if crate::ui::stop_daemon() {
            println!("Daemon stopped.");
        } else {
            println!("No daemon running.");
        }
        return;
    }

    if cli.daemon_live_stop {
        if crate::ui::stop_live_daemon() {
            println!("Live daemon stopped.");
        } else {
            println!("No live daemon running.");
        }
        return;
    }

    if cli.gen_config {
        match generate_config(
            cli.config.clone(),
            cli.logo.as_deref(),
            cli.layout.as_deref(),
        ) {
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
        Some(Commands::Extension { action }) => match action {
            ExtensionCommands::Install { path, repo } => {
                match install_extension(&path, repo.as_deref()) {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            ExtensionCommands::List => match list_extensions() {
                Ok(extensions) => {
                    if extensions.is_empty() {
                        println!("No extensions installed.");
                        println!(
                            "Extension directory: {}",
                            extensions::default_extension_dir().display()
                        );
                    } else {
                        println!("Installed extensions:");
                        for (name, path) in &extensions {
                            println!("  {}  ({})", name, path.display());
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
            ExtensionCommands::Remove { name } => match remove_extension(&name) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
        },
        Some(Commands::Theme { action }) => match action {
            ThemeCommands::List => match list_themes() {
                Ok(themes) => {
                    if themes.is_empty() {
                        println!("No themes installed.");
                        println!(
                            "Theme directory: {}",
                            config::default_themes_dir().display()
                        );
                    } else {
                        println!("Available themes:");
                        for (name, path) in &themes {
                            println!("  {}  ({})", name, path.display());
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
            ThemeCommands::Set { name } => {
                let config_path = cli
                    .config
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(config::default_config_path);

                match set_active_theme(&config_path, &name) {
                    Ok(()) => {
                        println!("Theme set to '{}'.", name);
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            ThemeCommands::Remove { name } => match remove_theme(&name) {
                Ok(()) => {
                    println!("Theme '{}' removed.", name);
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
            ThemeCommands::Export { name } => {
                let config = load_config(cli.config);
                match export_current_theme(&config, &name) {
                    Ok(path) => {
                        println!("Theme exported to {}", path.display());
                        println!("Set it with: xfetch theme set {}", name);
                    }
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
        },
        Some(Commands::Effects { action }) => match action {
            EffectCommands::Install { path, repo } => {
                match install_effect(&path, repo.as_deref()) {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            EffectCommands::List => match list_effects() {
                Ok(effects) => {
                    if effects.is_empty() {
                        println!("No effects installed.");
                        println!(
                            "Effect directory: {}",
                            effects::default_effect_dir().display()
                        );
                    } else {
                        println!("Installed effects:");
                        for (name, path) in &effects {
                            println!("  {}  ({})", name, path.display());
                        }
                    }
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
            EffectCommands::Remove { name } => match remove_effect(&name) {
                Ok(()) => {}
                Err(err) => {
                    eprintln!("Error: {}", err);
                    std::process::exit(1);
                }
            },
        },
        None => {
            let config = load_config(cli.config.clone());
            let (info, bench_lines) = Info::with_config(&config, cli.benchmark);
            if cli.daemon || config.daemon {
                crate::ui::draw_daemon(&info, &config);
            } else if config.daemon_live && !cli.no_daemon_live {
                let config_path = cli.config.clone().or_else(|| {
                    let d = config::default_config_path();
                    d.exists().then(|| d.to_string_lossy().into_owned())
                });
                let reload = config.daemon_live_reload || cli.daemon_live_reload;
                crate::ui::draw_live_daemon(&info, &config, config_path, reload);
            } else {
                draw(&info, &config);
            }
            if !bench_lines.is_empty() {
                println!("\n--- Benchmark -------------------------------");
                for line in &bench_lines {
                    println!("{}", line);
                }
                println!("---------------------------------------------");
            }
        }
    }
}
