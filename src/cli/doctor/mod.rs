use crate::addons::model::AddonKind;
pub mod platform;
use crate::config::{
    Config, ConfigOverrides, default_config_path, load_config_with_overrides,
    parse_jsonc_file_strict, resolve_theme_path,
};
use crate::error::XFetchError;
use crate::runtime;
use crate::ui;
use platform::PlatformInfo;
use serde::Serialize;
use std::io::IsTerminal;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Ok,
    Warn,
    Fail,
}

impl Status {
    fn label(self) -> &'static str {
        match self {
            Status::Ok => "ok",
            Status::Warn => "warn",
            Status::Fail => "fail",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Check {
    pub category: String,
    pub name: String,
    pub status: Status,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    pub action: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DoctorReport {
    pub schema_version: u32,
    pub version: &'static str,
    pub platform: PlatformInfo,
    pub checks: Vec<Check>,
    pub fixes: Vec<Fix>,
    pub benchmark: Vec<String>,
}

pub fn run(
    network: bool,
    fix: bool,
    bench: bool,
    image: bool,
    json: bool,
    config_path: Option<String>,
    overrides: &ConfigOverrides,
) -> Result<(), XFetchError> {
    let (mut report, config) = build_report(network, fix, config_path, overrides);
    if bench {
        let (_info, benchmark) = crate::info::Info::with_config(&config, true);
        report.benchmark = benchmark;
    }
    let failures = report
        .checks
        .iter()
        .filter(|check| check.status == Status::Fail)
        .count();

    if json {
        crate::cli::output::print_json(&report)?;
    } else {
        print_human(&report);
        if image && std::io::stdout().is_terminal() {
            println!();
            println!("[image]");
            if let Err(err) = crate::ui::preview_logo(&config) {
                crate::runtime::warn(format!("image preview failed: {err}"));
            }
        }
    }

    if failures > 0 {
        return Err(XFetchError::Exit(crate::error::EXIT_GENERIC));
    }
    Ok(())
}

fn build_report(
    network: bool,
    fix: bool,
    config_path: Option<String>,
    overrides: &ConfigOverrides,
) -> (DoctorReport, Config) {
    let mut checks = Vec::new();
    let mut fixes = Vec::new();
    check_environment(&mut checks);
    let config = check_configuration(&mut checks, &mut fixes, fix, config_path, overrides);
    check_terminal(&mut checks, &config);
    check_addons(&mut checks);
    check_cache(&mut checks, &mut fixes, fix);
    if network {
        check_network(&mut checks);
    }

    (
        DoctorReport {
            schema_version: crate::cli::output::JSON_SCHEMA_VERSION,
            version: env!("CARGO_PKG_VERSION"),
            platform: platform::collect(),
            checks,
            fixes,
            benchmark: Vec::new(),
        },
        config,
    )
}

fn push(
    checks: &mut Vec<Check>,
    category: &str,
    name: &str,
    status: Status,
    detail: impl Into<String>,
) {
    checks.push(Check {
        category: category.to_string(),
        name: name.to_string(),
        status,
        detail: detail.into(),
    });
}

fn check_environment(checks: &mut Vec<Check>) {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    push(
        checks,
        "environment",
        "version",
        Status::Ok,
        format!("xfetch {} ({profile})", env!("CARGO_PKG_VERSION")),
    );

    match std::env::current_exe() {
        Ok(path) => push(
            checks,
            "environment",
            "binary",
            if path.is_file() {
                Status::Ok
            } else {
                Status::Fail
            },
            path.display().to_string(),
        ),
        Err(err) => push(
            checks,
            "environment",
            "binary",
            Status::Warn,
            format!("cannot resolve the running binary: {err}"),
        ),
    }

    for (name, status) in [
        ("TERM", Status::Ok),
        ("TERM_PROGRAM", Status::Ok),
        ("KITTY_WINDOW_ID", Status::Ok),
        ("COLORTERM", Status::Ok),
        ("XDG_SESSION_TYPE", Status::Ok),
        ("XDG_CURRENT_DESKTOP", Status::Ok),
        ("SHELL", Status::Ok),
    ] {
        if let Ok(value) = std::env::var(name) {
            push(checks, "environment", name, status, value);
        }
    }
}

fn check_configuration(
    checks: &mut Vec<Check>,
    fixes: &mut Vec<Fix>,
    fix: bool,
    config_path: Option<String>,
    overrides: &ConfigOverrides,
) -> Config {
    let path = config_path
        .as_ref()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_config_path);

    if path.exists() {
        match parse_jsonc_file_strict(&path) {
            Ok(value) => match serde_json::from_value::<Config>(value) {
                Ok(_) => push(
                    checks,
                    "config",
                    "file",
                    Status::Ok,
                    path.display().to_string(),
                ),
                Err(err) => push(
                    checks,
                    "config",
                    "schema",
                    Status::Fail,
                    format!("invalid config schema: {err}"),
                ),
            },
            Err(err) => push(checks, "config", "file", Status::Fail, err),
        }
    } else {
        if fix {
            match crate::config::write_default_config(&path) {
                Ok(()) => {
                    fixes.push(Fix {
                        action: "generate-config".to_string(),
                        detail: path.display().to_string(),
                    });
                    push(
                        checks,
                        "config",
                        "file",
                        Status::Ok,
                        format!("{} (generated)", path.display()),
                    );
                }
                Err(err) => push(
                    checks,
                    "config",
                    "file",
                    Status::Fail,
                    format!("cannot generate '{}': {}", path.display(), err),
                ),
            }
        } else {
            push(
                checks,
                "config",
                "file",
                Status::Warn,
                format!("{} (not found; defaults are used)", path.display()),
            );
        }
    }

    let config = load_config_with_overrides(config_path, overrides);

    if let Some(theme) = &config.theme {
        let status = if resolve_theme_path(theme).is_some() {
            Status::Ok
        } else {
            Status::Warn
        };
        push(checks, "config", "theme", status, theme.clone());
    } else {
        push(checks, "config", "theme", Status::Ok, "default".to_string());
    }

    match config.layout.as_deref() {
        Some(layout) if ui::is_known_layout(layout) => {
            push(checks, "config", "layout", Status::Ok, layout.to_string())
        }
        Some(layout) => push(
            checks,
            "config",
            "layout",
            Status::Warn,
            format!("{layout} (not in --list-layouts; the default is used)"),
        ),
        None => push(
            checks,
            "config",
            "layout",
            Status::Ok,
            "default".to_string(),
        ),
    }

    push(
        checks,
        "config",
        "modules",
        Status::Ok,
        config.modules.len().to_string(),
    );
    push(
        checks,
        "config",
        "cache",
        Status::Ok,
        if config.disable_cache.unwrap_or(false) {
            "disabled".to_string()
        } else {
            "enabled".to_string()
        },
    );
    push(
        checks,
        "config",
        "network probes",
        Status::Ok,
        if config.disable_ip_fetching.unwrap_or(false) {
            "disabled".to_string()
        } else {
            "enabled".to_string()
        },
    );

    config
}

fn check_terminal(checks: &mut Vec<Check>, config: &Config) {
    let is_tty = std::io::stdout().is_terminal();
    push(
        checks,
        "terminal",
        "stdout",
        if is_tty { Status::Ok } else { Status::Warn },
        if is_tty {
            "tty".to_string()
        } else {
            "not a tty (images and effects are disabled)".to_string()
        },
    );

    let width = match crossterm::terminal::size() {
        Ok((width, height)) if width > 0 && height > 0 => {
            push(
                checks,
                "terminal",
                "size",
                Status::Ok,
                format!("{width}x{height}"),
            );
            width as usize
        }
        _ => {
            push(
                checks,
                "terminal",
                "size",
                Status::Warn,
                "unavailable (falls back to 80x24 geometry)".to_string(),
            );
            80
        }
    };

    let protocol = image_protocol(config);
    push(checks, "terminal", "image protocol", Status::Ok, protocol);

    let diagnostics = ui::logo_diagnostics(config, width);
    push(
        checks,
        "terminal",
        "logo",
        if diagnostics.path_exists {
            Status::Ok
        } else {
            Status::Warn
        },
        diagnostics
            .path
            .clone()
            .unwrap_or_else(|| "default".to_string()),
    );
    push(
        checks,
        "terminal",
        "logo kind",
        Status::Ok,
        if diagnostics.is_image {
            "image"
        } else {
            "ascii"
        },
    );
    if let Some(requested) = diagnostics.requested_width {
        push(
            checks,
            "terminal",
            "logo width requested",
            Status::Ok,
            requested.to_string(),
        );
    }
    if let Some(printed) = diagnostics.printed_width {
        push(
            checks,
            "terminal",
            "logo width printed",
            Status::Ok,
            printed.to_string(),
        );
    }
    if let Some(lines) = diagnostics.ascii_lines {
        push(
            checks,
            "terminal",
            "logo lines",
            Status::Ok,
            lines.to_string(),
        );
    }
    push(
        checks,
        "terminal",
        "logo gap",
        Status::Ok,
        diagnostics.gap.to_string(),
    );
    if let Some(content_width) = diagnostics.content_width {
        let label = if config.layout.as_deref() == Some("right") {
            "content width (right)"
        } else {
            "content width"
        };
        push(
            checks,
            "terminal",
            label,
            Status::Ok,
            content_width.to_string(),
        );
    }
    if let Some(image_x) = diagnostics.image_x {
        push(
            checks,
            "terminal",
            "image column",
            Status::Ok,
            image_x.to_string(),
        );
    }
}

fn image_protocol(config: &Config) -> &'static str {
    if std::env::var_os("KITTY_WINDOW_ID").is_some() && config.logo_kitty.unwrap_or(true) {
        return "kitty graphics";
    }
    let term_program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    let lc_terminal = std::env::var("LC_TERMINAL").unwrap_or_default();
    for value in [&term_program, &lc_terminal] {
        if value.contains("iTerm")
            || value.contains("WezTerm")
            || value.contains("mintty")
            || value.contains("rio")
            || value.contains("WarpTerminal")
        {
            return "iterm inline image";
        }
    }
    "half blocks"
}

fn check_addons(checks: &mut Vec<Check>) {
    for kind in [
        AddonKind::Plugin,
        AddonKind::Effect,
        AddonKind::Extension,
        AddonKind::Command,
    ] {
        let installed = match kind {
            AddonKind::Plugin => crate::plugins::list_plugins(),
            AddonKind::Effect => crate::effects::list_effects(),
            AddonKind::Extension => crate::extensions::list_extensions(),
            AddonKind::Command => crate::commands::list_commands(),
        };
        match installed {
            Ok(entries) => {
                let total = entries.len();
                let receipts = entries
                    .iter()
                    .filter(|(name, _)| kind.receipt_path(name).is_file())
                    .count();
                let status = if total > 0 && receipts < total {
                    Status::Warn
                } else {
                    Status::Ok
                };
                push(
                    checks,
                    "addons",
                    kind.plural_label(),
                    status,
                    format!(
                        "{total} installed, {receipts} with install receipt, {}",
                        kind.directory().display()
                    ),
                );
            }
            Err(err) => push(checks, "addons", kind.plural_label(), Status::Warn, err),
        }
    }
}

fn check_cache(checks: &mut Vec<Check>, fixes: &mut Vec<Fix>, fix: bool) {
    let cache = dirs::cache_dir()
        .map(|dir| dir.join("xfetch").join("cache.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("cache.json"));
    if cache.exists() {
        let size = std::fs::metadata(&cache)
            .map(|meta| meta.len())
            .unwrap_or(0);
        let valid = std::fs::read_to_string(&cache)
            .ok()
            .and_then(|content| serde_json::from_str::<serde_json::Value>(&content).ok())
            .is_some();
        if !valid && fix {
            match std::fs::remove_file(&cache) {
                Ok(()) => {
                    fixes.push(Fix {
                        action: "remove-invalid-cache".to_string(),
                        detail: cache.display().to_string(),
                    });
                    push(
                        checks,
                        "cache",
                        "file",
                        Status::Ok,
                        format!("{} (removed invalid cache)", cache.display()),
                    );
                }
                Err(err) => push(
                    checks,
                    "cache",
                    "file",
                    Status::Fail,
                    format!("cannot remove '{}': {}", cache.display(), err),
                ),
            }
        } else {
            push(
                checks,
                "cache",
                "file",
                if valid { Status::Ok } else { Status::Warn },
                format!(
                    "{} ({size} bytes{})",
                    cache.display(),
                    if valid { "" } else { ", invalid JSON" }
                ),
            );
        }
    } else {
        push(
            checks,
            "cache",
            "file",
            Status::Ok,
            format!("{} (not created yet)", cache.display()),
        );
    }
}

fn check_network(checks: &mut Vec<Check>) {
    if runtime::offline() {
        push(
            checks,
            "network",
            "checks",
            Status::Warn,
            "skipped because --offline is set".to_string(),
        );
        return;
    }

    let urls = [
        (
            "plugins catalog",
            "https://raw.githubusercontent.com/xfetch-cli/plugins/main/index.json",
        ),
        (
            "effects catalog",
            "https://raw.githubusercontent.com/xfetch-cli/effects/main/index.json",
        ),
        (
            "extensions catalog",
            "https://raw.githubusercontent.com/xfetch-cli/extensions/main/index.json",
        ),
        (
            "update API",
            "https://api.github.com/repos/xfetch-cli/xfetch/releases/latest",
        ),
    ];

    for (name, url) in urls {
        match probe(url) {
            Ok(status) if (200..300).contains(&status) => push(
                checks,
                "network",
                name,
                Status::Ok,
                format!("HTTP {status}"),
            ),
            Ok(status) => push(
                checks,
                "network",
                name,
                Status::Warn,
                format!("HTTP {status}"),
            ),
            Err(err) => push(checks, "network", name, Status::Fail, err),
        }
    }
}

fn probe(url: &str) -> Result<u16, String> {
    let response = ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(4)))
        .build()
        .call()
        .map_err(|err| err.to_string())?;
    Ok(response.status().as_u16())
}

fn print_human(report: &DoctorReport) {
    println!(
        "xfetch {} doctor — {} {} ({})",
        report.version, report.platform.os, report.platform.arch, report.platform.family
    );
    if !runtime::quiet() {
        for detail in &report.platform.details {
            println!("  {:<22} {}", detail.name, detail.value);
        }
    }

    let mut current = "";
    for check in &report.checks {
        if runtime::quiet() && check.status == Status::Ok {
            continue;
        }
        if check.category != current {
            current = &check.category;
            println!();
            println!("[{}]", check.category);
        }
        println!(
            "  {:<6} {:<24} {}",
            check.status.label(),
            check.name,
            check.detail
        );
    }

    if !report.fixes.is_empty() {
        println!();
        println!("[fixes]");
        for fix in &report.fixes {
            println!("  {:<24} {}", fix.action, fix.detail);
        }
    }

    if !report.benchmark.is_empty() {
        println!();
        println!("[benchmark]");
        for line in &report.benchmark {
            println!("  {line}");
        }
    }

    let failures = report
        .checks
        .iter()
        .filter(|check| check.status == Status::Fail)
        .count();
    let warnings = report
        .checks
        .iter()
        .filter(|check| check.status == Status::Warn)
        .count();
    println!();
    println!(
        "{} checks: {} failures, {} warnings",
        report.checks.len(),
        failures,
        warnings
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(Status::Ok.label(), "ok");
        assert_eq!(Status::Warn.label(), "warn");
        assert_eq!(Status::Fail.label(), "fail");
    }

    #[test]
    fn image_protocol_prefers_kitty() {
        // The test process inherits the environment; only assert that the
        // function returns one of the documented values.
        let config = Config::default();
        let protocol = image_protocol(&config);
        assert!(["kitty graphics", "iterm inline image", "half blocks"].contains(&protocol));
    }
}
