use crate::cli::args::Cli;
use crate::cli::{Context, output, template};
use crate::config::{Config, ModuleConfig};
use crate::error::XFetchError;
use crate::info::Info;
use crate::runtime;

pub fn run(cli: &Cli, ctx: &Context) -> Result<(), XFetchError> {
    let config = ctx.load_config();

    let output_mode = !cli.get.is_empty() || cli.template.is_some() || cli.format.is_some();
    let daemon = cli.daemon || config.daemon || (config.daemon_live && !cli.no_daemon_live);
    if daemon && (cli.json || output_mode) {
        return Err(XFetchError::Usage(
            "daemon mode cannot be combined with --json, --get, --template or --format."
                .to_string(),
        ));
    }

    runtime::debug(format!(
        "config: {}",
        ctx.config_path
            .clone()
            .unwrap_or_else(|| crate::config::default_config_path().display().to_string())
    ));
    runtime::debug(format!(
        "layout: {}",
        config.layout.as_deref().unwrap_or("default")
    ));
    runtime::trace(format!("configured modules: {}", config.modules.len()));

    if !cli.get.is_empty() {
        let keys = cli.get.clone();
        let collect_config = config_with_modules(&config, &keys);
        let (info, _) = Info::with_config(&collect_config, false);
        let values = output::collect_values(&info, &collect_config, &keys);
        return output::print_values(&values, &keys, cli.json);
    }

    if let Some(template_source) = template_source(cli)? {
        let keys = template::keys(&template_source);
        let collect_config = config_with_modules(&config, &keys);
        let (info, _) = Info::with_config(&collect_config, false);
        let context = template::context(&info, &collect_config, &keys);
        let rendered = template::render(&template_source, &context);
        print!("{rendered}");
        if !rendered.ends_with('\n') {
            println!();
        }
        return Ok(());
    }

    if cli.json {
        let (info, bench_lines) = Info::with_config(&config, cli.benchmark);
        let json = output::FetchJson::from_info(&info, bench_lines);
        return output::print_json(&json);
    }

    let (info, bench_lines) = Info::with_config(&config, cli.benchmark);
    if cli.daemon || config.daemon {
        crate::ui::draw_daemon(&info, &config);
    } else if config.daemon_live && !cli.no_daemon_live {
        let config_path = ctx.config_path.clone().or_else(|| {
            let default = crate::config::default_config_path();
            default
                .exists()
                .then(|| default.to_string_lossy().into_owned())
        });
        let reload = config.daemon_live_reload || cli.daemon_live_reload;
        crate::ui::draw_live_daemon(&info, &config, config_path, reload);
    } else {
        crate::ui::draw(&info, &config);
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

fn config_with_modules(config: &Config, keys: &[String]) -> Config {
    let mut config = config.clone();
    config.modules = keys
        .iter()
        .filter(|key| !is_special_key(key))
        .map(|key| {
            let probe = if key == "packages_breakdown" {
                "packages"
            } else {
                key
            };
            ModuleConfig::Simple(probe.to_string())
        })
        .collect();
    config
}

fn is_special_key(key: &str) -> bool {
    matches!(key, "palette" | "header" | "sep" | "version" | "name")
}

fn template_source(cli: &Cli) -> Result<Option<String>, XFetchError> {
    if let Some(template) = &cli.format {
        return Ok(Some(template.clone()));
    }
    if let Some(path) = &cli.template {
        let content = std::fs::read_to_string(path).map_err(|err| {
            XFetchError::Config(format!("cannot read template '{}': {}", path, err))
        })?;
        return Ok(Some(content));
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn special_keys_are_not_probed() {
        assert!(is_special_key("header"));
        assert!(is_special_key("palette"));
        assert!(!is_special_key("os"));
        assert!(!is_special_key("packages:pacman"));
    }

    #[test]
    fn collect_config_only_keeps_module_keys() {
        let config = Config::default();
        let keys = vec!["os".to_string(), "header".to_string(), "cpu".to_string()];
        let collected = config_with_modules(&config, &keys);
        assert_eq!(collected.modules.len(), 2);
    }
}
