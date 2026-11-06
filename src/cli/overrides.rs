use crate::cli::args::Cli;
use crate::cli::modules;
use crate::config::{ConfigOverrides, ModuleConfig, resolve_theme_path};
use crate::error::XFetchError;
use crate::runtime;

pub fn build(cli: &Cli) -> ConfigOverrides {
    let modules = cli.modules.as_ref().and_then(|list| {
        let parsed: Vec<ModuleConfig> = list
            .split(',')
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(|item| ModuleConfig::Simple(item.to_string()))
            .collect();
        (!parsed.is_empty()).then_some(parsed)
    });

    ConfigOverrides {
        theme: cli.theme.clone(),
        layout: cli.layout.clone(),
        modules,
        no_color: runtime::no_color(),
        offline: cli.offline,
        no_cache: cli.no_cache,
    }
}

pub fn validate(cli: &Cli) -> Result<(), XFetchError> {
    if cli.backup && cli.restore {
        return Err(XFetchError::Usage(
            "use either --backup or --restore, not both.".to_string(),
        ));
    }

    if (cli.backup || cli.restore) && cli.command.is_some() {
        return Err(XFetchError::Usage(
            "--backup/--restore cannot be combined with a subcommand; use 'xfetch config backup' or 'xfetch config restore' instead.".to_string(),
        ));
    }

    let other_action = cli.gen_config
        || cli.list_layouts
        || cli.list_modules
        || cli.clean_cache
        || cli.daemon
        || cli.daemon_stop
        || cli.no_daemon_live
        || cli.daemon_live_stop
        || cli.daemon_live_reload
        || !cli.get.is_empty()
        || cli.modules.is_some()
        || cli.template.is_some()
        || cli.format.is_some();
    if (cli.backup || cli.restore) && other_action {
        return Err(XFetchError::Usage(
            "--backup/--restore cannot be combined with another action flag.".to_string(),
        ));
    }

    if let Some(layout) = &cli.layout
        && !cli.gen_config
        && !crate::ui::is_known_layout(layout)
    {
        return Err(XFetchError::Usage(format!(
            "unknown layout '{}'. Run 'xfetch --list-layouts' to see the accepted names.",
            layout
        )));
    }

    if let Some(theme) = &cli.theme
        && !cli.gen_config
        && resolve_theme_path(theme).is_none()
    {
        return Err(XFetchError::Config(format!(
            "theme '{}' not found. Run 'xfetch theme list' to see installed themes.",
            theme
        )));
    }

    if cli.modules.is_some() && build(cli).modules.is_none() {
        return Err(XFetchError::Usage(
            "--modules needs at least one module key.".to_string(),
        ));
    }

    if !cli.get.is_empty() && cli.command.is_some() {
        return Err(XFetchError::Usage(
            "--get only applies to the default fetch; drop the subcommand.".to_string(),
        ));
    }

    if cli.template.is_some() && cli.format.is_some() {
        return Err(XFetchError::Usage(
            "use either --template or --format, not both.".to_string(),
        ));
    }

    if cli.json && (cli.template.is_some() || cli.format.is_some()) {
        return Err(XFetchError::Usage(
            "use either --json or --template/--format, not both.".to_string(),
        ));
    }

    if !cli.get.is_empty() && (cli.template.is_some() || cli.format.is_some()) {
        return Err(XFetchError::Usage(
            "use either --get or --template/--format, not both.".to_string(),
        ));
    }

    if (cli.template.is_some() || cli.format.is_some()) && cli.command.is_some() {
        return Err(XFetchError::Usage(
            "--template/--format only apply to the default fetch; drop the subcommand.".to_string(),
        ));
    }

    for key in &cli.get {
        if !modules::is_known(key) && !matches!(key.as_str(), "version" | "name") {
            return Err(XFetchError::NotFound(format!(
                "unknown module key '{}'. Run 'xfetch --list-modules'.",
                key
            )));
        }
    }

    Ok(())
}
