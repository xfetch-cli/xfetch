mod cache;
mod cli;
mod config;
mod effects;
mod error;
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
    error::exit_on_error(run(cli));
}

fn run(cli: Cli) -> Result<(), error::XFetchError> {
    if cli.clean_cache {
        return cache::clean()
            .map_err(|err| error::XFetchError::CleanCache(err.to_string()))
            .map(|()| println!("Cache cleaned."));
    }

    if cli.daemon_stop {
        if crate::ui::stop_daemon() {
            println!("Daemon stopped.");
        } else {
            println!("No daemon running.");
        }
        return Ok(());
    }

    if cli.daemon_live_stop {
        if crate::ui::stop_live_daemon() {
            println!("Live daemon stopped.");
        } else {
            println!("No live daemon running.");
        }
        return Ok(());
    }

    if cli.gen_config {
        return generate_config(
            cli.config.clone(),
            cli.logo.as_deref(),
            cli.layout.as_deref(),
        )
        .map_err(|err| error::XFetchError::GenConfig(err.to_string()))
        .map(|path| {
            println!("Generated config: {}", path.display());
            println!("Run xfetch to see the new layout.");
        });
    }

    match cli.command {
        Some(Commands::Plugin { action }) => {
            match action {
                PluginCommands::Install { path, repo } => {
                    install_plugin(&path, repo.as_deref()).map_err(error::XFetchError::Fatal)
                }
                PluginCommands::List => list_plugins()
                    .map_err(error::XFetchError::Fatal)
                    .map(|plugins| {
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
                    }),
                PluginCommands::Remove { name } => {
                    remove_plugin(&name).map_err(error::XFetchError::Fatal)
                }
            }
        }
        Some(Commands::Extension { action }) => match action {
            ExtensionCommands::Install { path, repo } => {
                install_extension(&path, repo.as_deref()).map_err(error::XFetchError::Fatal)
            }
            ExtensionCommands::List => list_extensions()
                .map_err(error::XFetchError::Fatal)
                .map(|extensions| {
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
                }),
            ExtensionCommands::Remove { name } => {
                remove_extension(&name).map_err(error::XFetchError::Fatal)
            }
        },
        Some(Commands::Theme { action }) => match action {
            ThemeCommands::List => {
                list_themes()
                    .map_err(error::XFetchError::Fatal)
                    .map(|themes| {
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
                    })
            }
            ThemeCommands::Set { name } => {
                let config_path = cli
                    .config
                    .as_ref()
                    .map(PathBuf::from)
                    .unwrap_or_else(config::default_config_path);

                set_active_theme(&config_path, &name)
                    .map_err(error::XFetchError::Fatal)
                    .map(|()| println!("Theme set to '{}'.", name))
            }
            ThemeCommands::Remove { name } => remove_theme(&name)
                .map_err(error::XFetchError::Fatal)
                .map(|()| println!("Theme '{}' removed.", name)),
            ThemeCommands::Export { name } => {
                let config = load_config(cli.config);
                export_current_theme(&config, &name)
                    .map_err(error::XFetchError::Fatal)
                    .map(|path| {
                        println!("Theme exported to {}", path.display());
                        println!("Set it with: xfetch theme set {}", name);
                    })
            }
        },
        Some(Commands::Effects { action }) => {
            match action {
                EffectCommands::Install { path, repo } => {
                    install_effect(&path, repo.as_deref()).map_err(error::XFetchError::Fatal)
                }
                EffectCommands::List => list_effects()
                    .map_err(error::XFetchError::Fatal)
                    .map(|effects| {
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
                    }),
                EffectCommands::Remove { name } => {
                    remove_effect(&name).map_err(error::XFetchError::Fatal)
                }
            }
        }
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
            Ok(())
        }
    }
}
