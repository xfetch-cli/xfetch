mod addons;
mod atomic_fs;
mod cache;
mod cli;
mod config;
mod effects;
mod error;
mod extensions;
mod hash;
mod info;
mod install_support;
mod logos;
mod plugins;
mod subprocess;
mod themes;
mod ui;
#[cfg(feature = "update")]
mod update;
mod wasm;

use crate::addons::install::infer_name;
use crate::addons::model::{AddonEntry, AddonKind};
use crate::config::{generate_config, load_config};
use crate::effects::{install_effect, list_effects, remove_effect};
use crate::extensions::{install_extension, list_extensions, remove_extension};
use crate::info::Info;
use crate::plugins::{install_plugin, list_plugins, remove_plugin};
use crate::themes::{export_current_theme, list_themes, remove_theme, set_active_theme};
use crate::ui::draw;
use clap::Parser;
use cli::{
    Cli, Commands, EffectCommands, ExtensionCommands, PluginCommands, ThemeCommands, WasmCommands,
};
use std::path::PathBuf;

fn main() {
    let cli = Cli::parse();
    error::exit_on_error(run(cli));
}

fn run(cli: Cli) -> Result<(), error::XFetchError> {
    if cli.list_layouts {
        for name in crate::ui::LAYOUT_NAMES {
            println!("{name}");
        }
        return Ok(());
    }

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
        Some(Commands::Plugin { action }) => match action {
            PluginCommands::Install {
                path,
                repo,
                sha256,
                name,
                manifest,
            } => {
                let direct = name.is_some() || manifest.is_some();
                let installed = if direct {
                    if repo.is_some() {
                        return Err(error::XFetchError::Fatal(
                            "--name/--manifest apply to direct wasm files or URLs; omit --repo"
                                .to_string(),
                        ));
                    }
                    addons::install::install_direct(
                        AddonKind::Plugin,
                        &path,
                        sha256.as_deref(),
                        name.as_deref(),
                        manifest.as_deref(),
                    )
                    .map_err(error::XFetchError::Fatal)?
                } else {
                    install_plugin(&path, repo.as_deref(), sha256.as_deref())
                        .map_err(error::XFetchError::Fatal)?;
                    infer_name(AddonKind::Plugin, &path, None).map_err(error::XFetchError::Fatal)?
                };
                addons::report::print_install_guidance(AddonKind::Plugin, &installed);
                Ok(())
            }
            PluginCommands::List { verbose } => {
                let installed = list_plugins().map_err(error::XFetchError::Fatal)?;
                let config = load_config(cli.config.clone());
                let entries = installed
                    .into_iter()
                    .map(|(name, path)| AddonEntry::load(AddonKind::Plugin, &name, &path, &config))
                    .collect::<Vec<_>>();
                addons::report::print_list(AddonKind::Plugin, &entries, verbose);
                Ok(())
            }
            PluginCommands::Info { name } => {
                let path = AddonKind::Plugin
                    .installed_path(&name)
                    .or_else(|| plugins::find_plugin_binary(&name))
                    .ok_or_else(|| {
                        error::XFetchError::Fatal(format!("Plugin '{}' is not installed.", name))
                    })?;
                let config = load_config(cli.config.clone());
                let entry = AddonEntry::load(AddonKind::Plugin, &name, &path, &config);
                addons::report::print_info(&entry);
                Ok(())
            }
            PluginCommands::Search { query, json } => {
                addons::catalog::print_search(AddonKind::Plugin, query.as_deref(), json)
                    .map_err(error::XFetchError::Fatal)
            }
            PluginCommands::Remove { name } => {
                remove_plugin(&name).map_err(error::XFetchError::Fatal)
            }
        },
        Some(Commands::Extension { action }) => match action {
            ExtensionCommands::Install {
                path,
                repo,
                sha256,
                name,
                manifest,
            } => {
                let direct = name.is_some() || manifest.is_some();
                let installed = if direct {
                    if repo.is_some() {
                        return Err(error::XFetchError::Fatal(
                            "--name/--manifest apply to direct wasm files or URLs; omit --repo"
                                .to_string(),
                        ));
                    }
                    addons::install::install_direct(
                        AddonKind::Extension,
                        &path,
                        sha256.as_deref(),
                        name.as_deref(),
                        manifest.as_deref(),
                    )
                    .map_err(error::XFetchError::Fatal)?
                } else {
                    install_extension(&path, repo.as_deref(), sha256.as_deref())
                        .map_err(error::XFetchError::Fatal)?;
                    infer_name(AddonKind::Extension, &path, None)
                        .map_err(error::XFetchError::Fatal)?
                };
                addons::report::print_install_guidance(AddonKind::Extension, &installed);
                Ok(())
            }
            ExtensionCommands::List { verbose } => {
                let installed = list_extensions().map_err(error::XFetchError::Fatal)?;
                let config = load_config(cli.config.clone());
                let entries = installed
                    .into_iter()
                    .map(|(name, path)| {
                        AddonEntry::load(AddonKind::Extension, &name, &path, &config)
                    })
                    .collect::<Vec<_>>();
                addons::report::print_list(AddonKind::Extension, &entries, verbose);
                Ok(())
            }
            ExtensionCommands::Info { name } => {
                let path = AddonKind::Extension
                    .installed_path(&name)
                    .or_else(|| extensions::find_extension_binary(&name))
                    .ok_or_else(|| {
                        error::XFetchError::Fatal(format!("Extension '{}' is not installed.", name))
                    })?;
                let config = load_config(cli.config.clone());
                let entry = AddonEntry::load(AddonKind::Extension, &name, &path, &config);
                addons::report::print_info(&entry);
                Ok(())
            }
            ExtensionCommands::Search { query, json } => {
                addons::catalog::print_search(AddonKind::Extension, query.as_deref(), json)
                    .map_err(error::XFetchError::Fatal)
            }
            ExtensionCommands::Remove { name } => {
                remove_extension(&name).map_err(error::XFetchError::Fatal)
            }
        },
        Some(Commands::Theme { action }) => match action {
            ThemeCommands::List => list_themes()
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
                }),
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
        Some(Commands::Update {
            check,
            prebuilt,
            bin_dir,
            yes,
        }) => {
            #[cfg(feature = "update")]
            {
                let options = update::UpdateOptions {
                    check,
                    prebuilt,
                    bin_dir: bin_dir.map(PathBuf::from),
                    yes,
                };
                let outcome = update::run(options).map_err(error::XFetchError::Fatal)?;
                match outcome {
                    update::UpdateOutcome::UpToDate { current, latest } => {
                        println!("xfetch {} is up to date (latest v{}).", current, latest);
                    }
                    update::UpdateOutcome::UpdateAvailable {
                        current,
                        latest,
                        method,
                        command,
                    } => {
                        println!("xfetch {} -> v{} available ({}).", current, latest, method);
                        println!("  {}", command);
                        std::process::exit(1);
                    }
                    update::UpdateOutcome::Updated { from, to, path } => {
                        println!("Updated xfetch {} -> {} ({})", from, to, path.display());
                    }
                    update::UpdateOutcome::Manual { message } => println!("{}", message),
                }
                Ok(())
            }
            #[cfg(not(feature = "update"))]
            {
                let _ = (check, prebuilt, bin_dir, yes);
                Err(error::XFetchError::Fatal(
                    "This xfetch binary was built without the 'update' feature.".to_string(),
                ))
            }
        }
        Some(Commands::Wasm { action }) => match action {
            WasmCommands::Inspect { path, json } => {
                wasm::inspect::inspect(std::path::Path::new(&path), json)
                    .map_err(error::XFetchError::Fatal)
            }
            WasmCommands::Run {
                path,
                request,
                request_file,
                timeout,
                kind,
            } => {
                let request = match (request, request_file) {
                    (Some(_), Some(_)) => {
                        Err("Use either --request or --request_file, not both".to_string())
                    }
                    (Some(request), None) => Ok(request.into_bytes()),
                    (None, Some(file)) => std::fs::read(&file)
                        .map_err(|err| format!("Failed to read '{}': {}", file, err)),
                    (None, None) => Ok(b"{}".to_vec()),
                }
                .map_err(error::XFetchError::Fatal)?;

                let kind = match kind.as_str() {
                    "effect" => wasm::GuestKind::Effect,
                    "extension" => wasm::GuestKind::Extension,
                    _ => wasm::GuestKind::Plugin,
                };
                let timeout = timeout.map(std::time::Duration::from_secs);

                wasm::run_request(std::path::Path::new(&path), &request, timeout, kind)
                    .map_err(error::XFetchError::Fatal)
                    .map(|response| {
                        use std::io::Write;
                        let stdout = std::io::stdout();
                        let mut stdout = stdout.lock();
                        let _ = stdout.write_all(&response);
                        let _ = stdout.write_all(b"\n");
                    })
            }
            WasmCommands::Wit => {
                wasm::inspect::print_wit();
                Ok(())
            }
        },
        Some(Commands::Effects { action }) => match action {
            EffectCommands::Install {
                path,
                repo,
                sha256,
                name,
                manifest,
            } => {
                let direct = name.is_some() || manifest.is_some();
                let installed = if direct {
                    if repo.is_some() {
                        return Err(error::XFetchError::Fatal(
                            "--name/--manifest apply to direct wasm files or URLs; omit --repo"
                                .to_string(),
                        ));
                    }
                    addons::install::install_direct(
                        AddonKind::Effect,
                        &path,
                        sha256.as_deref(),
                        name.as_deref(),
                        manifest.as_deref(),
                    )
                    .map_err(error::XFetchError::Fatal)?
                } else {
                    install_effect(&path, repo.as_deref(), sha256.as_deref())
                        .map_err(error::XFetchError::Fatal)?;
                    infer_name(AddonKind::Effect, &path, None).map_err(error::XFetchError::Fatal)?
                };
                addons::report::print_install_guidance(AddonKind::Effect, &installed);
                Ok(())
            }
            EffectCommands::List { verbose } => {
                let installed = list_effects().map_err(error::XFetchError::Fatal)?;
                let config = load_config(cli.config.clone());
                let entries = installed
                    .into_iter()
                    .map(|(name, path)| AddonEntry::load(AddonKind::Effect, &name, &path, &config))
                    .collect::<Vec<_>>();
                addons::report::print_list(AddonKind::Effect, &entries, verbose);
                Ok(())
            }
            EffectCommands::Info { name } => {
                let path = AddonKind::Effect
                    .installed_path(&name)
                    .or_else(|| effects::find_effect_binary(&name))
                    .ok_or_else(|| {
                        error::XFetchError::Fatal(format!("Effect '{}' is not installed.", name))
                    })?;
                let config = load_config(cli.config.clone());
                let entry = AddonEntry::load(AddonKind::Effect, &name, &path, &config);
                addons::report::print_info(&entry);
                Ok(())
            }
            EffectCommands::Search { query, json } => {
                addons::catalog::print_search(AddonKind::Effect, query.as_deref(), json)
                    .map_err(error::XFetchError::Fatal)
            }
            EffectCommands::Remove { name } => {
                remove_effect(&name).map_err(error::XFetchError::Fatal)
            }
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
            Ok(())
        }
    }
}
