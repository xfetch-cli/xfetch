use crate::addons::model::AddonKind;
use crate::cli::args::{AddonKindArg, Cli, Commands};
use crate::cli::{
    Context, addon_cmd, backup_cmd, build_info_cmd, completions, config_cmd, doctor, env_cmd,
    fetch, logo_cmd, modules, overrides, paths_cmd, run_cmd, schema_cmd, settings, theme_cmd,
    uninstall_cmd, update_cmd, wasm_cmd,
};
use crate::error::XFetchError;
use crate::runtime;

pub fn run(mut cli: Cli) -> Result<(), XFetchError> {
    settings::init(&cli);
    overrides::validate(&cli)?;
    let overrides = overrides::build(&cli);
    let ctx = Context {
        config_path: cli.config.clone(),
        overrides,
    };
    let json = cli.json;

    if cli.backup {
        return backup_cmd::backup(cli.config.clone(), None, false, false, false, None, json);
    }

    if cli.restore {
        return backup_cmd::restore(cli.config.clone(), None, false, None, None, json);
    }

    if cli.list_layouts {
        if json {
            return crate::cli::output::print_json(&crate::ui::LAYOUT_NAMES);
        }
        for name in crate::ui::LAYOUT_NAMES {
            println!("{name}");
        }
        return Ok(());
    }

    if cli.list_modules {
        return modules::print_modules(json);
    }

    if cli.clean_cache {
        return crate::cache::clean()
            .map_err(|err| XFetchError::CleanCache(err.to_string()))
            .map(|()| {
                if json {
                    let _ = crate::cli::output::print_json(&serde_json::json!({"cleaned": true}));
                } else {
                    runtime::info("Cache cleaned.");
                }
            });
    }

    if cli.daemon_stop {
        let stopped = crate::ui::stop_daemon();
        if json {
            return crate::cli::output::print_json(&serde_json::json!({"stopped": stopped}));
        }
        runtime::info(if stopped {
            "Daemon stopped."
        } else {
            "No daemon running."
        });
        return Ok(());
    }

    if cli.daemon_live_stop {
        let stopped = crate::ui::stop_live_daemon();
        if json {
            return crate::cli::output::print_json(&serde_json::json!({"stopped": stopped}));
        }
        runtime::info(if stopped {
            "Live daemon stopped."
        } else {
            "No live daemon running."
        });
        return Ok(());
    }

    if cli.gen_config {
        let path = crate::config::generate_config(
            cli.config.clone(),
            cli.logo.as_deref(),
            cli.layout.as_deref(),
        )
        .map_err(|err| XFetchError::GenConfig(err.to_string()))?;
        if json {
            return crate::cli::output::print_json(&serde_json::json!({
                "path": path.display().to_string(),
            }));
        }
        runtime::info(format!("Generated config: {}", path.display()));
        runtime::info("Run xfetch to see the new layout.");
        return Ok(());
    }

    match cli.command.take() {
        Some(Commands::Plugin { action }) => addon_cmd::run(
            AddonKind::Plugin,
            action,
            &cli,
            &ctx.overrides,
            ctx.config_path.clone(),
        ),
        Some(Commands::Extension { action }) => addon_cmd::run(
            AddonKind::Extension,
            action,
            &cli,
            &ctx.overrides,
            ctx.config_path.clone(),
        ),
        Some(Commands::Effects { action }) => addon_cmd::run(
            AddonKind::Effect,
            action,
            &cli,
            &ctx.overrides,
            ctx.config_path.clone(),
        ),
        Some(Commands::Addon { kind, action }) => addon_cmd::run(
            addon_kind(kind),
            action,
            &cli,
            &ctx.overrides,
            ctx.config_path.clone(),
        ),
        Some(Commands::Command { action }) => addon_cmd::run(
            AddonKind::Command,
            action,
            &cli,
            &ctx.overrides,
            ctx.config_path.clone(),
        ),
        Some(Commands::Run { name, render, args }) => {
            run_cmd::run(&name, &args, render, cli.json, &ctx)
        }
        Some(Commands::Theme { action }) => theme_cmd::run(action, &cli, &ctx),
        Some(Commands::Logo { action }) => logo_cmd::run(action, cli.json),
        Some(Commands::Wasm { action }) => wasm_cmd::run(action, &cli),
        Some(Commands::Update {
            check,
            prebuilt,
            bin_dir,
            yes,
            rollback,
        }) => update_cmd::run(check, prebuilt, bin_dir, yes, rollback, &cli),
        Some(Commands::Doctor {
            network,
            fix,
            bench,
            image,
        }) => doctor::run(
            network,
            fix,
            bench,
            image,
            cli.json,
            ctx.config_path.clone(),
            &ctx.overrides,
        ),
        Some(Commands::Modules) => modules::print_modules(json),
        Some(Commands::Config { action }) => {
            config_cmd::run(action, &cli, &ctx.overrides, ctx.config_path.clone())
        }
        Some(Commands::Completions { shell }) => completions::completions(shell),
        Some(Commands::Man { output }) => completions::man(output),
        Some(Commands::Schema { kind }) => schema_cmd::run(kind),
        Some(Commands::Paths) => paths_cmd::run(ctx.config_path.clone(), cli.json),
        Some(Commands::Env) => env_cmd::run(ctx.config_path.clone(), cli.json),
        Some(Commands::BuildInfo) => build_info_cmd::run(cli.json),
        Some(Commands::SelfUninstall { purge, yes }) => uninstall_cmd::run(purge, yes, cli.json),
        None => fetch::run(&cli, &ctx),
    }
}

fn addon_kind(kind: AddonKindArg) -> AddonKind {
    match kind {
        AddonKindArg::Plugin => AddonKind::Plugin,
        AddonKindArg::Effect => AddonKind::Effect,
        AddonKindArg::Extension => AddonKind::Extension,
        AddonKindArg::Command => AddonKind::Command,
    }
}
