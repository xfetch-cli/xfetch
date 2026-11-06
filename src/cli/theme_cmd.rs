use crate::cli::args::{Cli, ThemeCommands};
use crate::cli::{Context, output};
use crate::config::{
    default_themes_dir, load_config_with_overrides, parse_jsonc_str_strict, resolve_theme_path,
};
use crate::error::XFetchError;
use crate::runtime;
use crate::themes::{export_current_theme, list_themes, remove_theme, set_active_theme};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const THEMES_INDEX: &str = "https://raw.githubusercontent.com/xfetch-cli/themes/main/index.json";
const THEME_INDEX_LIMIT: usize = 4 * 1024 * 1024;
const THEME_FILE_LIMIT: usize = 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ThemeIndex {
    #[serde(default)]
    registry: String,
    themes: Vec<ThemeEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ThemeEntry {
    name: String,
    #[serde(default)]
    author: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    layout: Option<String>,
    #[serde(default)]
    palette_style: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    source: String,
    #[serde(default)]
    screenshot: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ThemeMeta {
    version: String,
    source: String,
    installed_unix: u64,
}

pub fn run(action: ThemeCommands, cli: &Cli, ctx: &Context) -> Result<(), XFetchError> {
    match action {
        ThemeCommands::List => list(cli),
        ThemeCommands::Search { query } => search(query.as_deref(), cli.json),
        ThemeCommands::Info { name } => info(&name, cli.json),
        ThemeCommands::Install { name, force } => install(&name, force, cli.json),
        ThemeCommands::Update {
            name,
            all,
            check,
            force,
        } => update(name.as_deref(), all, check, force, cli.json),
        ThemeCommands::Set { name } => {
            let config_path = ctx
                .config_path
                .as_ref()
                .map(PathBuf::from)
                .unwrap_or_else(crate::config::default_config_path);
            set_active_theme(&config_path, &name).map_err(XFetchError::Config)?;
            if cli.json {
                return output::print_json(&serde_json::json!({
                    "schema_version": output::JSON_SCHEMA_VERSION,
                    "status": "set",
                    "theme": name,
                    "path": config_path.display().to_string(),
                }));
            }
            runtime::info(format!("Theme set to '{}'.", name));
            Ok(())
        }
        ThemeCommands::Remove { name } => {
            remove_theme(&name).map_err(XFetchError::Config)?;
            if cli.json {
                return output::print_json(&serde_json::json!({
                    "schema_version": output::JSON_SCHEMA_VERSION,
                    "status": "removed",
                    "theme": name,
                }));
            }
            runtime::info(format!("Theme '{}' removed.", name));
            Ok(())
        }
        ThemeCommands::Export { name } => {
            let config = load_config_with_overrides(ctx.config_path.clone(), &ctx.overrides);
            let path = export_current_theme(&config, &name).map_err(XFetchError::Config)?;
            if cli.json {
                return output::print_json(&serde_json::json!({
                    "schema_version": output::JSON_SCHEMA_VERSION,
                    "status": "exported",
                    "theme": name,
                    "path": path.display().to_string(),
                }));
            }
            runtime::info(format!("Theme exported to {}", path.display()));
            runtime::info(format!("Set it with: xfetch theme set {}", name));
            Ok(())
        }
    }
}

fn list(cli: &Cli) -> Result<(), XFetchError> {
    let themes = list_themes().map_err(XFetchError::Config)?;
    if cli.json {
        let entries: Vec<serde_json::Value> = themes
            .iter()
            .map(|(name, path)| {
                let meta = read_meta(name);
                serde_json::json!({
                    "name": name,
                    "path": path.display().to_string(),
                    "version": meta.as_ref().map(|meta| meta.version.clone()),
                    "source": meta.as_ref().map(|meta| meta.source.clone()),
                    "installed_unix": meta.as_ref().map(|meta| meta.installed_unix),
                })
            })
            .collect();
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "themes": entries,
        }));
    }
    if themes.is_empty() {
        println!("No themes installed.");
        println!(
            "Theme directory: {}",
            crate::config::default_themes_dir().display()
        );
    } else {
        println!("Available themes:");
        for (name, path) in &themes {
            let version = read_meta(name)
                .map(|meta| format!(" v{}", meta.version))
                .unwrap_or_default();
            println!("  {}{}  ({})", name, version, path.display());
        }
    }
    Ok(())
}

fn search(query: Option<&str>, json: bool) -> Result<(), XFetchError> {
    let index = fetch_index()?;
    let query = query.map(|query| query.to_ascii_lowercase());
    let entries: Vec<&ThemeEntry> = index
        .themes
        .iter()
        .filter(|entry| {
            let Some(query) = &query else {
                return true;
            };
            entry.name.to_ascii_lowercase().contains(query)
                || entry.description.to_ascii_lowercase().contains(query)
                || entry
                    .tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains(query))
        })
        .collect();

    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "themes": entries,
        }));
    }

    println!(
        "{:<20} {:<10} {:<10} DESCRIPTION",
        "NAME", "VERSION", "AUTHOR"
    );
    for entry in entries {
        println!(
            "{:<20} {:<10} {:<10} {}",
            entry.name, entry.version, entry.author, entry.description
        );
    }
    Ok(())
}

fn info(name: &str, json: bool) -> Result<(), XFetchError> {
    let index = fetch_index()?;
    let entry = find_entry(&index, name)?;
    let meta = read_meta(&entry.name);
    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "theme": entry,
            "installed": resolve_theme_path(&entry.name).is_some(),
            "installed_version": meta.as_ref().map(|meta| meta.version.clone()),
        }));
    }
    println!("Name: {}", entry.name);
    println!("Author: {}", entry.author);
    println!("Version: {}", entry.version);
    println!("Description: {}", entry.description);
    if let Some(layout) = &entry.layout {
        println!("Layout: {}", layout);
    }
    if let Some(style) = &entry.palette_style {
        println!("Palette style: {}", style);
    }
    if !entry.tags.is_empty() {
        println!("Tags: {}", entry.tags.join(", "));
    }
    println!("Source: {}", entry.source);
    println!(
        "Installed: {}",
        if resolve_theme_path(&entry.name).is_some() {
            "yes"
        } else {
            "no"
        }
    );
    if let Some(meta) = meta {
        println!("Installed version: {}", meta.version);
    }
    Ok(())
}

fn install(name: &str, force: bool, json: bool) -> Result<(), XFetchError> {
    let index = fetch_index()?;
    let entry = find_entry(&index, name)?.clone();
    let target = theme_path(&entry.name);
    if target.exists() && !force {
        return Err(XFetchError::Config(format!(
            "theme '{}' is already installed; use --force to reinstall",
            entry.name
        )));
    }
    let content = fetch_theme(&index, &entry)?;
    write_theme(&target, &content)?;
    write_meta(&entry).map_err(XFetchError::Config)?;

    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "status": "installed",
            "name": entry.name,
            "version": entry.version,
            "path": target.display().to_string(),
        }));
    }
    runtime::info(format!(
        "Installed theme '{}' v{} to {}",
        entry.name,
        entry.version,
        target.display()
    ));
    Ok(())
}

fn update(
    name: Option<&str>,
    all: bool,
    check: bool,
    force: bool,
    json: bool,
) -> Result<(), XFetchError> {
    if all && name.is_some() {
        return Err(XFetchError::Usage(
            "use either a theme name or --all, not both".to_string(),
        ));
    }
    if !all && name.is_none() {
        return Err(XFetchError::Usage(
            "specify a theme name or use --all".to_string(),
        ));
    }

    let index = fetch_index()?;
    let targets: Vec<String> = if all {
        list_themes()
            .map_err(XFetchError::Config)?
            .into_iter()
            .map(|(name, _)| name)
            .filter(|name| index.themes.iter().any(|entry| &entry.name == name))
            .collect()
    } else {
        vec![name.expect("validated above").to_string()]
    };

    let mut results = Vec::new();
    let mut any_update = false;
    for target in targets {
        let entry = find_entry(&index, &target)?;
        let meta = read_meta(&entry.name);
        let installed = meta.as_ref().map(|meta| meta.version.clone());
        let update_available =
            force || installed.is_none() || version_newer(installed.as_deref(), &entry.version);
        any_update |= update_available;

        if check {
            results.push(serde_json::json!({
                "name": entry.name,
                "installed": installed,
                "available": entry.version,
                "update_available": update_available,
            }));
            continue;
        }

        if !update_available {
            results.push(serde_json::json!({
                "name": entry.name,
                "installed": installed,
                "available": entry.version,
                "update_available": false,
                "status": "up-to-date",
            }));
            continue;
        }

        let content = fetch_theme(&index, entry)?;
        write_theme(&theme_path(&entry.name), &content)?;
        write_meta(entry).map_err(XFetchError::Config)?;
        results.push(serde_json::json!({
            "name": entry.name,
            "installed": installed,
            "available": entry.version,
            "update_available": true,
            "status": "updated",
        }));
    }

    if json {
        output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "check": check,
            "themes": results,
        }))?;
    } else if check {
        if results.is_empty() {
            runtime::info("No matching themes to check.");
        }
        for result in &results {
            let name = result["name"].as_str().unwrap_or("?");
            let installed = result["installed"].as_str().unwrap_or("not installed");
            let available = result["available"].as_str().unwrap_or("?");
            let flag = if result["update_available"].as_bool().unwrap_or(false) {
                "update available"
            } else {
                "up to date"
            };
            println!("{name}: {installed} -> {available} ({flag})");
        }
    } else {
        for result in &results {
            let name = result["name"].as_str().unwrap_or("?");
            let status = result["status"].as_str().unwrap_or("updated");
            runtime::info(format!("{name}: {status}"));
        }
    }

    if check && any_update {
        return Err(XFetchError::Exit(crate::error::EXIT_GENERIC));
    }
    Ok(())
}

fn fetch_index() -> Result<ThemeIndex, XFetchError> {
    if runtime::offline() {
        return Err(XFetchError::Network(
            "network access is disabled by --offline".to_string(),
        ));
    }
    let source = std::env::var("XFETCH_THEME_INDEX").unwrap_or_else(|_| THEMES_INDEX.to_string());
    let text = fetch_text(&source, THEME_INDEX_LIMIT).map_err(XFetchError::Network)?;
    serde_json::from_str(&text)
        .map_err(|err| XFetchError::Network(format!("invalid theme index: {err}")))
}

fn fetch_theme(index: &ThemeIndex, entry: &ThemeEntry) -> Result<String, XFetchError> {
    if runtime::offline() {
        return Err(XFetchError::Network(
            "network access is disabled by --offline".to_string(),
        ));
    }
    let source = if entry.source.starts_with("http://") || entry.source.starts_with("https://") {
        entry.source.clone()
    } else {
        format!("{}/{}", index.registry.trim_end_matches('/'), entry.source)
    };
    let content = fetch_text(&source, THEME_FILE_LIMIT).map_err(XFetchError::Network)?;
    validate_theme(&content)?;
    Ok(content)
}

fn validate_theme(content: &str) -> Result<(), XFetchError> {
    let value = parse_jsonc_str_strict(content).map_err(XFetchError::Config)?;
    let Some(object) = value.as_object() else {
        return Err(XFetchError::Config(
            "theme file must contain a JSON object".to_string(),
        ));
    };
    if object.contains_key("icons") {
        return Err(XFetchError::Config(
            "theme files must not contain icons".to_string(),
        ));
    }
    Ok(())
}

fn fetch_text(url: &str, limit: usize) -> Result<String, String> {
    let mut response = ureq::get(url)
        .config()
        .timeout_global(Some(Duration::from_secs(15)))
        .build()
        .call()
        .map_err(|err| format!("failed to fetch '{}': {err}", url))?;
    let status = response.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("failed to fetch '{}': HTTP {status}", url));
    }
    response
        .body_mut()
        .with_config()
        .limit(limit as u64)
        .read_to_string()
        .map_err(|err| format!("failed to read '{}': {err}", url))
}

fn find_entry<'a>(index: &'a ThemeIndex, name: &str) -> Result<&'a ThemeEntry, XFetchError> {
    index
        .themes
        .iter()
        .find(|entry| entry.name.eq_ignore_ascii_case(name))
        .ok_or_else(|| {
            XFetchError::NotFound(format!("theme '{}' was not found in the registry", name))
        })
}

fn theme_path(name: &str) -> PathBuf {
    default_themes_dir().join(format!("{name}.jsonc"))
}

fn meta_path(name: &str) -> PathBuf {
    default_themes_dir().join(format!(".{name}.xfetch-meta.json"))
}

fn read_meta(name: &str) -> Option<ThemeMeta> {
    let text = fs::read_to_string(meta_path(name)).ok()?;
    serde_json::from_str(&text).ok()
}

fn write_meta(entry: &ThemeEntry) -> Result<(), String> {
    let meta = ThemeMeta {
        version: entry.version.clone(),
        source: entry.source.clone(),
        installed_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0),
    };
    let text = serde_json::to_string_pretty(&meta)
        .map_err(|err| format!("cannot serialize theme metadata: {err}"))?;
    let path = meta_path(&entry.name);
    crate::atomic_fs::write_atomic(&path, text.as_bytes())
        .map_err(|err| format!("cannot write '{}': {}", path.display(), err))
}

fn write_theme(path: &Path, content: &str) -> Result<(), XFetchError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            XFetchError::Config(format!("cannot create '{}': {}", parent.display(), err))
        })?;
    }
    crate::atomic_fs::write_atomic(path, content.as_bytes())
        .map_err(|err| XFetchError::Config(format!("cannot write '{}': {}", path.display(), err)))
}

fn version_newer(installed: Option<&str>, available: &str) -> bool {
    let Some(installed) = installed else {
        return true;
    };
    match (
        Version::parse(installed.trim_start_matches('v')),
        Version::parse(available.trim_start_matches('v')),
    ) {
        (Ok(installed), Ok(available)) => available > installed,
        _ => installed != available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison_handles_unknown_versions() {
        assert!(version_newer(None, "1.0.0"));
        assert!(version_newer(Some("0.9.0"), "1.0.0"));
        assert!(!version_newer(Some("1.0.0"), "1.0.0"));
        assert!(version_newer(Some("custom"), "1.0.0"));
    }

    #[test]
    fn validate_theme_rejects_icons() {
        assert!(validate_theme("{\"layout\":\"section\"}").is_ok());
        assert!(validate_theme("{\"icons\":{}}").is_err());
    }
}
