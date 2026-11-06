use crate::addons::model::AddonKind;
use crate::config::{
    Config, ConfigOverrides, config_dir, default_config_path, load_config_with_overrides,
    parse_jsonc_str_strict, resolve_theme_path,
};
use crate::error::XFetchError;
use crate::runtime;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCHEMA_VERSION: u32 = 1;
const BACKUP_PREFIX: &str = "xfetch-backup-";
const SAFETY_PREFIX: &str = "pre-restore-";
const MAX_ASSET_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ADDON_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_ADDON_BYTES: u64 = 512 * 1024 * 1024;
const ENCRYPTION_MAGIC: &[u8] = b"XFBACKUP1";

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupBundle {
    pub schema_version: u32,
    pub created_unix: u64,
    #[serde(default)]
    pub created_unix_ms: u64,
    pub created_utc: String,
    pub xfetch_version: String,
    pub config_path: String,
    pub theme: Option<String>,
    pub theme_path: Option<String>,
    pub config_providers: Vec<String>,
    pub files: Vec<BackupFile>,
    pub missing: Vec<MissingFile>,
    pub warnings: Vec<String>,
    pub resolved_config: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BackupFile {
    pub role: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encoding: Option<String>,
    pub content: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MissingFile {
    pub role: String,
    pub path: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct Route {
    #[serde(rename = "_name", default)]
    name: Option<String>,
    path: String,
}

#[derive(Debug)]
struct BackupMeta {
    path: PathBuf,
    created_unix: u64,
    created_unix_ms: u64,
    created_utc: String,
    files: usize,
    missing: usize,
    size: u64,
    compressed: bool,
}

pub fn default_backup_dir() -> PathBuf {
    config_dir().join("xfetch").join("backups")
}

pub fn backup(
    config_path: Option<String>,
    output: Option<String>,
    with_addons: bool,
    compress: bool,
    encrypt: bool,
    passphrase_file: Option<String>,
    json: bool,
) -> Result<(), XFetchError> {
    let bundle = create_bundle(config_path, with_addons)?;
    let passphrase = if encrypt {
        Some(read_passphrase(passphrase_file.as_deref())?)
    } else {
        None
    };
    let path = write_bundle(
        &bundle,
        output.as_deref(),
        BACKUP_PREFIX,
        compress,
        encrypt,
        passphrase.as_deref(),
    )?;

    if json {
        return crate::cli::output::print_json(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "path": path.display().to_string(),
            "files": bundle.files.len(),
            "missing": bundle.missing.len(),
            "warnings": bundle.warnings,
            "with_addons": with_addons,
            "compressed": compress,
            "encrypted": encrypt,
        }));
    }

    runtime::info(format!("Backup written to {}", path.display()));
    runtime::info(format!(
        "Files: {} backed up, {} missing.",
        bundle.files.len(),
        bundle.missing.len()
    ));
    Ok(())
}

pub fn restore(
    config_path: Option<String>,
    from: Option<String>,
    dry_run: bool,
    root: Option<String>,
    passphrase_file: Option<String>,
    json: bool,
) -> Result<(), XFetchError> {
    let backup_path = match from {
        Some(value) => {
            let path = expand_path(&value);
            if path.is_dir() {
                latest_in_dir(&path)?
            } else {
                path
            }
        }
        None => latest_backup()?,
    };
    let passphrase = read_optional_passphrase(passphrase_file.as_deref())?;
    let bundle = load_bundle(&backup_path, passphrase.as_deref())?;

    let safety = if dry_run {
        None
    } else {
        safety_backup(config_path.clone())?
    };

    let original_root = Path::new(&bundle.config_path)
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let remap_root = root.as_ref().map(|value| expand_path(value));

    let mut restored = Vec::new();
    let mut main_target = None;
    for file in &bundle.files {
        let original = PathBuf::from(&file.path);
        let target = if file.role == "config"
            && let Some(config_path) = &config_path
        {
            expand_path(config_path)
        } else if let Some(root) = &remap_root
            && let Ok(relative) = original.strip_prefix(&original_root)
        {
            root.join(relative)
        } else {
            original
        };

        if file.role == "config" {
            main_target = Some(target.clone());
        }
        restored.push(target.display().to_string());
        if dry_run {
            continue;
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                XFetchError::Config(format!("cannot create '{}': {}", parent.display(), err))
            })?;
        }
        let bytes = if file.encoding.as_deref() == Some("base64") {
            BASE64.decode(file.content.as_bytes()).map_err(|err| {
                XFetchError::Config(format!(
                    "cannot decode '{}' from the backup: {}",
                    file.path, err
                ))
            })?
        } else {
            file.content.as_bytes().to_vec()
        };
        crate::atomic_fs::write_atomic(&target, &bytes).map_err(|err| {
            XFetchError::Config(format!("cannot write '{}': {}", target.display(), err))
        })?;
    }

    let validation = if dry_run {
        None
    } else {
        main_target
            .as_ref()
            .map(|target| validate_restored_config(target))
    };

    if json {
        return crate::cli::output::print_json(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "backup": backup_path.display().to_string(),
            "dry_run": dry_run,
            "root": root,
            "restored": restored,
            "missing": bundle.missing,
            "safety_backup": safety.as_ref().map(|path| path.display().to_string()),
            "validation": validation,
        }));
    }

    if dry_run {
        runtime::info(format!(
            "Would restore {} files from {}",
            restored.len(),
            backup_path.display()
        ));
    } else {
        runtime::info(format!(
            "Restored {} files from {}",
            restored.len(),
            backup_path.display()
        ));
    }
    for path in &restored {
        println!("  {path}");
    }
    if let Some(safety) = &safety {
        runtime::info(format!("Safety backup: {}", safety.display()));
    }
    if let Some(validation) = &validation
        && !validation["valid"].as_bool().unwrap_or(true)
    {
        runtime::info(format!(
            "Warning: the restored config is invalid: {}",
            validation["error"].as_str().unwrap_or("unknown error")
        ));
    }
    Ok(())
}

pub fn safety_backup(config_path: Option<String>) -> Result<Option<PathBuf>, XFetchError> {
    match create_bundle(config_path, false) {
        Ok(bundle) => match write_bundle(&bundle, None, SAFETY_PREFIX, false, false, None) {
            Ok(path) => Ok(Some(path)),
            Err(err) => {
                runtime::info(format!("Warning: could not write the safety backup: {err}"));
                Ok(None)
            }
        },
        Err(err) => {
            runtime::info(format!(
                "Warning: could not back up the current config: {err}"
            ));
            Ok(None)
        }
    }
}

pub fn list_backups(json: bool) -> Result<(), XFetchError> {
    let mut backups = collect_meta(&default_backup_dir())?;
    backups.sort_by_key(|meta| std::cmp::Reverse((meta.created_unix_ms, meta.created_unix)));

    if json {
        let entries: Vec<serde_json::Value> = backups
            .iter()
            .map(|meta| {
                serde_json::json!({
                    "path": meta.path.display().to_string(),
                    "created_unix": meta.created_unix,
                    "created_utc": meta.created_utc,
                    "files": meta.files,
                    "missing": meta.missing,
                    "size": meta.size,
                    "compressed": meta.compressed,
                })
            })
            .collect();
        return crate::cli::output::print_json(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "backups": entries,
        }));
    }

    if backups.is_empty() {
        println!("No backups found.");
        return Ok(());
    }
    println!(
        "{:<20} {:<8} {:<8} {:<10} PATH",
        "CREATED", "FILES", "MISSING", "SIZE"
    );
    for meta in &backups {
        println!(
            "{:<20} {:<8} {:<8} {:<10} {}",
            meta.created_utc,
            meta.files,
            meta.missing,
            format_size(meta.size),
            meta.path.display()
        );
    }
    Ok(())
}

pub fn prune(keep: usize, dry_run: bool, json: bool) -> Result<(), XFetchError> {
    let dir = default_backup_dir();
    let mut backups = collect_meta(&dir)?;
    backups.retain(|meta| {
        meta.path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(BACKUP_PREFIX))
    });
    backups.sort_by_key(|meta| std::cmp::Reverse((meta.created_unix_ms, meta.created_unix)));

    let deleted: Vec<&BackupMeta> = backups.iter().skip(keep).collect();
    if !dry_run {
        for meta in &deleted {
            fs::remove_file(&meta.path).map_err(|err| {
                XFetchError::Config(format!("cannot delete '{}': {}", meta.path.display(), err))
            })?;
        }
    }

    if json {
        return crate::cli::output::print_json(&serde_json::json!({
            "schema_version": SCHEMA_VERSION,
            "kept": backups.len().saturating_sub(deleted.len()),
            "deleted": deleted
                .iter()
                .map(|meta| meta.path.display().to_string())
                .collect::<Vec<_>>(),
            "dry_run": dry_run,
        }));
    }

    if deleted.is_empty() {
        runtime::info("No backups to prune.");
    } else if dry_run {
        runtime::info(format!("Would delete {} old backups.", deleted.len()));
        for meta in &deleted {
            println!("  {}", meta.path.display());
        }
    } else {
        runtime::info(format!("Deleted {} old backups.", deleted.len()));
        for meta in &deleted {
            println!("  {}", meta.path.display());
        }
    }
    Ok(())
}

fn collect_meta(dir: &Path) -> Result<Vec<BackupMeta>, XFetchError> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => {
            return Err(XFetchError::Config(format!(
                "cannot read '{}': {}",
                dir.display(),
                err
            )));
        }
    };

    let mut metas = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default();
        if !(name.starts_with(BACKUP_PREFIX) || name.starts_with(SAFETY_PREFIX)) {
            continue;
        }
        if !name.ends_with(".json") && !name.ends_with(".json.gz") {
            continue;
        }
        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|meta| meta.len()).unwrap_or(0);
        let compressed = name.ends_with(".gz");
        match load_bundle(&path, None) {
            Ok(bundle) => metas.push(BackupMeta {
                path,
                created_unix: bundle.created_unix,
                created_unix_ms: if bundle.created_unix_ms == 0 {
                    bundle.created_unix * 1000
                } else {
                    bundle.created_unix_ms
                },
                created_utc: bundle.created_utc,
                files: bundle.files.len(),
                missing: bundle.missing.len(),
                size,
                compressed,
            }),
            Err(_) => {
                let created = metadata
                    .and_then(|meta| meta.modified().ok())
                    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                    .map(|duration| duration.as_secs())
                    .unwrap_or(0);
                metas.push(BackupMeta {
                    path,
                    created_unix: created,
                    created_unix_ms: created * 1000,
                    created_utc: format_utc(created),
                    files: 0,
                    missing: 0,
                    size,
                    compressed,
                });
            }
        }
    }
    Ok(metas)
}

fn validate_restored_config(path: &Path) -> serde_json::Value {
    match fs::read_to_string(path) {
        Ok(content) => match parse_jsonc_str_strict(&content) {
            Ok(value) => match serde_json::from_value::<Config>(value) {
                Ok(_) => serde_json::json!({"valid": true}),
                Err(err) => serde_json::json!({"valid": false, "error": err.to_string()}),
            },
            Err(err) => serde_json::json!({"valid": false, "error": err}),
        },
        Err(err) => serde_json::json!({"valid": false, "error": err.to_string()}),
    }
}

fn create_bundle(
    config_path: Option<String>,
    with_addons: bool,
) -> Result<BackupBundle, XFetchError> {
    let config_path = config_path
        .map(|value| expand_path(&value))
        .unwrap_or_else(default_config_path);

    if !config_path.is_file() {
        return Err(XFetchError::Config(format!(
            "no config file found at '{}'",
            config_path.display()
        )));
    }

    let main_content = fs::read_to_string(&config_path).map_err(|err| {
        XFetchError::Config(format!("cannot read '{}': {}", config_path.display(), err))
    })?;

    let mut files = Vec::new();
    let mut missing = Vec::new();
    let mut warnings = Vec::new();
    let mut seen = HashSet::new();

    add_file_text(
        &mut files,
        &mut seen,
        "config",
        &config_path,
        None,
        &main_content,
    );

    let mut config: Option<Config> = None;
    match parse_jsonc_str_strict(&main_content) {
        Ok(value) => match serde_json::from_value(value) {
            Ok(parsed) => config = Some(parsed),
            Err(err) => warnings.push(format!("config schema could not be read: {err}")),
        },
        Err(err) => warnings.push(format!("config could not be parsed: {err}")),
    }

    let theme_path = config
        .as_ref()
        .and_then(|config| config.theme.as_ref())
        .and_then(|theme| resolve_theme_path(theme))
        .map(|path| path.display().to_string());

    let mut providers = Vec::new();
    if let Some(config) = &config {
        collect_config_references(
            config,
            &config_path,
            &mut files,
            &mut missing,
            &mut warnings,
            &mut seen,
            &mut providers,
            0,
        );
    }

    if with_addons {
        collect_addons(&mut files, &mut missing, &mut warnings, &mut seen);
    }

    let resolved = load_config_with_overrides(
        Some(config_path.to_string_lossy().into_owned()),
        &ConfigOverrides::default(),
    );
    let resolved_config = serde_json::to_value(&resolved).unwrap_or_else(|_| serde_json::json!({}));
    let created_unix = now_unix();
    let created_unix_ms = now_unix_ms();

    Ok(BackupBundle {
        schema_version: SCHEMA_VERSION,
        created_unix,
        created_unix_ms,
        created_utc: format_utc(created_unix),
        xfetch_version: env!("CARGO_PKG_VERSION").to_string(),
        config_path: config_path.display().to_string(),
        theme: config.as_ref().and_then(|config| config.theme.clone()),
        theme_path,
        config_providers: providers,
        files,
        missing,
        warnings,
        resolved_config,
    })
}

#[allow(clippy::too_many_arguments)]
fn collect_config_references(
    config: &Config,
    config_path: &Path,
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
    providers: &mut Vec<String>,
    depth: usize,
) {
    if depth > 5 {
        warnings.push("config provider nesting is too deep; stopping".to_string());
        return;
    }

    collect_theme(config, files, missing, seen);
    collect_config_asset(config, files, missing, warnings, seen);

    for provider in &config.config_providers {
        if !providers.iter().any(|name| name == &provider.extension) {
            providers.push(provider.extension.clone());
        }

        let routes = provider
            .args
            .as_ref()
            .and_then(|args| args.get("routes"))
            .and_then(|value| value.as_str())
            .map(str::to_string)
            .or_else(|| {
                provider
                    .extension
                    .to_ascii_lowercase()
                    .contains("roulette")
                    .then(|| "~/.config/xfetch/routes.json".to_string())
            });

        if let Some(routes) = routes {
            let routes_path = resolve_relative(config_path.parent(), &routes);
            match fs::read_to_string(&routes_path) {
                Ok(content) => {
                    add_file_text(
                        files,
                        seen,
                        "routes",
                        &routes_path,
                        Some(&provider.extension),
                        &content,
                    );
                    match serde_json::from_str::<Vec<Route>>(&content) {
                        Ok(route_list) => {
                            let base = routes_path
                                .parent()
                                .unwrap_or_else(|| Path::new("."))
                                .to_path_buf();
                            for route in route_list {
                                let path = resolve_relative(Some(&base), &route.path);
                                match fs::read_to_string(&path) {
                                    Ok(content) => {
                                        let is_new = !seen.contains(&path.display().to_string());
                                        add_file_text(
                                            files,
                                            seen,
                                            "route-config",
                                            &path,
                                            route.name.as_deref(),
                                            &content,
                                        );
                                        if !is_new {
                                            continue;
                                        }
                                        match parse_jsonc_str_strict(&content) {
                                            Ok(value) => {
                                                match serde_json::from_value::<Config>(value) {
                                                    Ok(route_config) => {
                                                        collect_config_references(
                                                            &route_config,
                                                            &path,
                                                            files,
                                                            missing,
                                                            warnings,
                                                            seen,
                                                            providers,
                                                            depth + 1,
                                                        );
                                                    }
                                                    Err(err) => warnings.push(format!(
                                                        "route config '{}' schema could not be read: {err}",
                                                        path.display()
                                                    )),
                                                }
                                            }
                                            Err(err) => warnings.push(format!(
                                                "route config '{}' could not be parsed: {err}",
                                                path.display()
                                            )),
                                        }
                                    }
                                    Err(err) => missing.push(MissingFile {
                                        role: "route-config".to_string(),
                                        path: path.display().to_string(),
                                        reason: err.to_string(),
                                    }),
                                }
                            }
                        }
                        Err(err) => warnings.push(format!(
                            "routes file '{}' could not be parsed: {}",
                            routes_path.display(),
                            err
                        )),
                    }
                }
                Err(err) => missing.push(MissingFile {
                    role: "routes".to_string(),
                    path: routes_path.display().to_string(),
                    reason: err.to_string(),
                }),
            }
        }

        if let Some(args) = &provider.args {
            collect_arg_paths(args, config_path.parent(), files, missing, warnings, seen);
        }
    }
}

fn collect_arg_paths(
    value: &serde_json::Value,
    base: Option<&Path>,
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    match value {
        serde_json::Value::String(text) if looks_like_path(text) => {
            let path = resolve_relative(base, text);
            if path.is_file() {
                add_asset_file(files, missing, warnings, seen, "provider-file", &path, None);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_arg_paths(item, base, files, missing, warnings, seen);
            }
        }
        serde_json::Value::Object(map) => {
            for (key, item) in map {
                if key == "routes" {
                    continue;
                }
                collect_arg_paths(item, base, files, missing, warnings, seen);
            }
        }
        _ => {}
    }
}

fn looks_like_path(value: &str) -> bool {
    if value.starts_with("http://") || value.starts_with("https://") {
        return false;
    }
    value.starts_with('~')
        || value.starts_with('/')
        || value.starts_with("./")
        || value.starts_with("../")
        || [
            ".json", ".jsonc", ".txt", ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".ico", ".webp",
            ".tiff", ".svg",
        ]
        .iter()
        .any(|extension| value.ends_with(extension))
}

fn collect_theme(
    config: &Config,
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    seen: &mut HashSet<String>,
) {
    let Some(theme) = &config.theme else {
        return;
    };
    if let Some(path) = resolve_theme_path(theme) {
        match fs::read_to_string(&path) {
            Ok(content) => add_file_text(files, seen, "theme", &path, Some(theme), &content),
            Err(err) => missing.push(MissingFile {
                role: "theme".to_string(),
                path: path.display().to_string(),
                reason: err.to_string(),
            }),
        }
    } else {
        missing.push(MissingFile {
            role: "theme".to_string(),
            path: theme.clone(),
            reason: "theme not found".to_string(),
        });
    }
}

fn collect_config_asset(
    config: &Config,
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let (value, role) = if let Some(path) = &config.logo_path {
        (Some(path.as_str()), "logo")
    } else if let Some(path) = &config.ascii {
        (Some(path.as_str()), "ascii")
    } else {
        (None, "asset")
    };
    let Some(value) = value else {
        return;
    };
    let path = expand_path(value);
    if path.is_file() {
        add_asset_file(files, missing, warnings, seen, role, &path, None);
    } else {
        missing.push(MissingFile {
            role: role.to_string(),
            path: path.display().to_string(),
            reason: "file not found".to_string(),
        });
    }
}

fn collect_addons(
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    let mut total = 0u64;
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
        let installed = match installed {
            Ok(installed) => installed,
            Err(err) => {
                warnings.push(format!("could not list {}: {}", kind.plural_label(), err));
                continue;
            }
        };

        for (name, path) in installed {
            let label = format!("{}:{}", kind.label(), name);
            if let Ok(metadata) = fs::metadata(&path) {
                if metadata.len() > MAX_ADDON_BYTES {
                    warnings.push(format!(
                        "skipping {} artifact ({} > {} bytes)",
                        label,
                        metadata.len(),
                        MAX_ADDON_BYTES
                    ));
                } else if total + metadata.len() > MAX_TOTAL_ADDON_BYTES {
                    warnings.push(format!(
                        "skipping {} artifact (addon backup size limit reached)",
                        label
                    ));
                } else if let Ok(bytes) = fs::read(&path) {
                    total += bytes.len() as u64;
                    add_file_bytes(files, seen, "addon-artifact", &path, Some(&label), &bytes);
                }
            }

            let manifest = kind.directory().join(kind.manifest_name(&name));
            if manifest.is_file()
                && let Ok(content) = fs::read_to_string(&manifest)
            {
                add_file_text(
                    files,
                    seen,
                    "addon-manifest",
                    &manifest,
                    Some(&label),
                    &content,
                );
            }
            let receipt = kind.receipt_path(&name);
            if receipt.is_file()
                && let Ok(content) = fs::read_to_string(&receipt)
            {
                add_file_text(
                    files,
                    seen,
                    "addon-receipt",
                    &receipt,
                    Some(&label),
                    &content,
                );
            } else {
                missing.push(MissingFile {
                    role: "addon-receipt".to_string(),
                    path: receipt.display().to_string(),
                    reason: "receipt not found".to_string(),
                });
            }
        }
    }
}

fn add_asset_file(
    files: &mut Vec<BackupFile>,
    missing: &mut Vec<MissingFile>,
    warnings: &mut Vec<String>,
    seen: &mut HashSet<String>,
    role: &str,
    path: &Path,
    name: Option<&str>,
) {
    if seen.contains(&path.display().to_string()) {
        return;
    }
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) => {
            missing.push(MissingFile {
                role: role.to_string(),
                path: path.display().to_string(),
                reason: err.to_string(),
            });
            return;
        }
    };
    if metadata.len() > MAX_ASSET_BYTES {
        warnings.push(format!(
            "skipping asset '{}' ({} > {} bytes)",
            path.display(),
            metadata.len(),
            MAX_ASSET_BYTES
        ));
        return;
    }
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) => {
            missing.push(MissingFile {
                role: role.to_string(),
                path: path.display().to_string(),
                reason: err.to_string(),
            });
            return;
        }
    };
    if is_text_asset(path)
        && let Ok(text) = String::from_utf8(bytes.clone())
    {
        add_file_text(files, seen, role, path, name, &text);
        return;
    }
    add_file_bytes(files, seen, role, path, name, &bytes);
}

fn is_text_asset(path: &Path) -> bool {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    !matches!(
        extension.as_str(),
        "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "webp" | "tiff"
    )
}

fn add_file_text(
    files: &mut Vec<BackupFile>,
    seen: &mut HashSet<String>,
    role: &str,
    path: &Path,
    name: Option<&str>,
    content: &str,
) {
    let key = path.display().to_string();
    if seen.insert(key.clone()) {
        files.push(BackupFile {
            role: role.to_string(),
            path: key,
            name: name.map(str::to_string),
            encoding: None,
            content: content.to_string(),
        });
    }
}

fn add_file_bytes(
    files: &mut Vec<BackupFile>,
    seen: &mut HashSet<String>,
    role: &str,
    path: &Path,
    name: Option<&str>,
    bytes: &[u8],
) {
    let key = path.display().to_string();
    if seen.insert(key.clone()) {
        files.push(BackupFile {
            role: role.to_string(),
            path: key,
            name: name.map(str::to_string),
            encoding: Some("base64".to_string()),
            content: BASE64.encode(bytes),
        });
    }
}

fn write_bundle(
    bundle: &BackupBundle,
    output: Option<&str>,
    prefix: &str,
    compress: bool,
    encrypt: bool,
    passphrase: Option<&str>,
) -> Result<PathBuf, XFetchError> {
    let path = backup_target(output, prefix, compress, encrypt)?;
    let text = serde_json::to_string_pretty(bundle)
        .map_err(|err| XFetchError::Config(format!("cannot serialize backup: {err}")))?;
    let mut bytes = if compress {
        gzip_compress(text.as_bytes())
            .map_err(|err| XFetchError::Config(format!("cannot compress backup: {err}")))?
    } else {
        text.into_bytes()
    };
    if encrypt {
        let passphrase = passphrase.ok_or_else(|| {
            XFetchError::Usage(
                "encrypted backups need a passphrase via --passphrase-file or XFETCH_BACKUP_PASSPHRASE"
                    .to_string(),
            )
        })?;
        bytes = encrypt_bytes(&bytes, passphrase)
            .map_err(|err| XFetchError::Config(format!("cannot encrypt backup: {err}")))?;
    }
    crate::atomic_fs::write_atomic(&path, &bytes).map_err(|err| {
        XFetchError::Config(format!("cannot write '{}': {}", path.display(), err))
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
    }
    Ok(path)
}

fn backup_target(
    output: Option<&str>,
    prefix: &str,
    compress: bool,
    encrypt: bool,
) -> Result<PathBuf, XFetchError> {
    let created = now_unix();
    let extension = if encrypt {
        if compress {
            ".json.gz.age"
        } else {
            ".json.age"
        }
    } else if compress {
        ".json.gz"
    } else {
        ".json"
    };
    match output {
        Some(value) => {
            let path = expand_path(value);
            if path.is_dir() || value.ends_with('/') || value.ends_with(std::path::MAIN_SEPARATOR) {
                fs::create_dir_all(&path).map_err(|err| {
                    XFetchError::Config(format!("cannot create '{}': {}", path.display(), err))
                })?;
                Ok(unique_path(&path, prefix, created, extension))
            } else {
                if let Some(parent) = path.parent() {
                    fs::create_dir_all(parent).map_err(|err| {
                        XFetchError::Config(format!(
                            "cannot create '{}': {}",
                            parent.display(),
                            err
                        ))
                    })?;
                }
                Ok(path)
            }
        }
        None => {
            let dir = default_backup_dir();
            fs::create_dir_all(&dir).map_err(|err| {
                XFetchError::Config(format!("cannot create '{}': {}", dir.display(), err))
            })?;
            Ok(unique_path(&dir, prefix, created, extension))
        }
    }
}

fn unique_path(dir: &Path, prefix: &str, created: u64, extension: &str) -> PathBuf {
    let mut candidate = dir.join(format!("{prefix}{created}{extension}"));
    let mut counter = 1;
    while candidate.exists() {
        candidate = dir.join(format!("{prefix}{created}-{counter}{extension}"));
        counter += 1;
    }
    candidate
}

fn latest_backup() -> Result<PathBuf, XFetchError> {
    latest_in_dir(&default_backup_dir())
}

fn latest_in_dir(dir: &Path) -> Result<PathBuf, XFetchError> {
    let mut metas = collect_meta(dir)?;
    metas.retain(|meta| {
        meta.path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with(BACKUP_PREFIX))
    });
    metas.sort_by_key(|meta| std::cmp::Reverse((meta.created_unix_ms, meta.created_unix)));
    metas
        .into_iter()
        .next()
        .map(|meta| meta.path)
        .ok_or_else(|| XFetchError::NotFound(format!("no backups found in '{}'", dir.display())))
}

fn load_bundle(path: &Path, passphrase: Option<&str>) -> Result<BackupBundle, XFetchError> {
    let mut bytes = fs::read(path)
        .map_err(|err| XFetchError::Config(format!("cannot read '{}': {}", path.display(), err)))?;
    if bytes.starts_with(ENCRYPTION_MAGIC) {
        let passphrase = passphrase.ok_or_else(|| {
            XFetchError::Config(
                "this backup is encrypted; pass --passphrase-file or set XFETCH_BACKUP_PASSPHRASE"
                    .to_string(),
            )
        })?;
        bytes = decrypt_bytes(&bytes, passphrase).map_err(|err| {
            XFetchError::Config(format!("cannot decrypt '{}': {}", path.display(), err))
        })?;
    }
    let bytes = if bytes.starts_with(&[0x1f, 0x8b]) {
        gzip_decompress(&bytes).map_err(|err| {
            XFetchError::Config(format!("cannot decompress '{}': {}", path.display(), err))
        })?
    } else {
        bytes
    };
    let bundle: BackupBundle = serde_json::from_slice(&bytes).map_err(|err| {
        XFetchError::Config(format!(
            "'{}' is not a valid xfetch backup: {}",
            path.display(),
            err
        ))
    })?;
    if bundle.schema_version != SCHEMA_VERSION {
        return Err(XFetchError::Config(format!(
            "unsupported backup schema version {} (expected {})",
            bundle.schema_version, SCHEMA_VERSION
        )));
    }
    Ok(bundle)
}

fn gzip_compress(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(bytes)?;
    encoder.finish()
}

fn gzip_decompress(bytes: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut decoder = GzDecoder::new(bytes);
    let mut output = Vec::new();
    decoder.read_to_end(&mut output)?;
    Ok(output)
}

fn read_optional_passphrase(file: Option<&str>) -> Result<Option<String>, XFetchError> {
    if let Some(file) = file {
        let text = fs::read_to_string(file).map_err(|err| {
            XFetchError::Config(format!("cannot read passphrase '{}': {}", file, err))
        })?;
        let passphrase = text.trim_end_matches(['\r', '\n']).to_string();
        if passphrase.is_empty() {
            return Err(XFetchError::Config(
                "the passphrase file is empty".to_string(),
            ));
        }
        return Ok(Some(passphrase));
    }
    match std::env::var("XFETCH_BACKUP_PASSPHRASE") {
        Ok(passphrase) if !passphrase.is_empty() => Ok(Some(passphrase)),
        _ => Ok(None),
    }
}

fn read_passphrase(file: Option<&str>) -> Result<String, XFetchError> {
    read_optional_passphrase(file)?.ok_or_else(|| {
        XFetchError::Usage("set XFETCH_BACKUP_PASSPHRASE or pass --passphrase-file".to_string())
    })
}

fn encrypt_bytes(bytes: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    use argon2::{Algorithm, Argon2, Params, Version};
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};

    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    getrandom::getrandom(&mut salt).map_err(|err| err.to_string())?;
    getrandom::getrandom(&mut nonce).map_err(|err| err.to_string())?;

    let params = Params::new(19 * 1024, 2, 1, Some(32)).map_err(|err| err.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), &salt, &mut key)
        .map_err(|err| err.to_string())?;

    let cipher = XChaCha20Poly1305::new_from_slice(&key).map_err(|err| err.to_string())?;
    let nonce = XNonce::try_from(&nonce[..]).map_err(|_| "invalid nonce".to_string())?;
    let ciphertext = cipher
        .encrypt(&nonce, bytes)
        .map_err(|err| err.to_string())?;

    let mut output =
        Vec::with_capacity(ENCRYPTION_MAGIC.len() + salt.len() + nonce.len() + ciphertext.len());
    output.extend_from_slice(ENCRYPTION_MAGIC);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce);
    output.extend_from_slice(&ciphertext);
    Ok(output)
}

fn decrypt_bytes(bytes: &[u8], passphrase: &str) -> Result<Vec<u8>, String> {
    use argon2::{Algorithm, Argon2, Params, Version};
    use chacha20poly1305::aead::{Aead, KeyInit};
    use chacha20poly1305::{XChaCha20Poly1305, XNonce};

    if !bytes.starts_with(ENCRYPTION_MAGIC) {
        return Err("not an encrypted xfetch backup".to_string());
    }
    let header = ENCRYPTION_MAGIC.len();
    let salt_end = header + 16;
    let nonce_end = salt_end + 24;
    if bytes.len() < nonce_end + 16 {
        return Err("encrypted backup is truncated".to_string());
    }
    let salt = &bytes[header..salt_end];
    let nonce = &bytes[salt_end..nonce_end];
    let ciphertext = &bytes[nonce_end..];

    let params = Params::new(19 * 1024, 2, 1, Some(32)).map_err(|err| err.to_string())?;
    let argon = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let mut key = [0u8; 32];
    argon
        .hash_password_into(passphrase.as_bytes(), salt, &mut key)
        .map_err(|err| err.to_string())?;

    let cipher = XChaCha20Poly1305::new_from_slice(&key).map_err(|err| err.to_string())?;
    let nonce = XNonce::try_from(nonce).map_err(|_| "invalid nonce".to_string())?;
    cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| "decryption failed".to_string())
}

fn resolve_relative(base: Option<&Path>, value: &str) -> PathBuf {
    let expanded = expand_path(value);
    if expanded.is_absolute() {
        return expanded;
    }
    if let Some(base) = base {
        let candidate = base.join(&expanded);
        if candidate.exists() {
            return candidate;
        }
    }
    expanded
}

fn expand_path(value: &str) -> PathBuf {
    if value == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(value));
    }
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    PathBuf::from(value)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

fn format_utc(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let remainder = secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = year + i64::from(month <= 2);
    (year, month as u32, day as u32)
}

fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit + 1 < UNITS.len() {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{size:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_utc_matches_a_known_timestamp() {
        assert_eq!(format_utc(0), "1970-01-01T00:00:00Z");
        assert_eq!(format_utc(1_600_000_000), "2020-09-13T12:26:40Z");
    }

    #[test]
    fn expand_path_handles_home_and_relative_paths() {
        assert!(expand_path("~/x").is_absolute());
        assert_eq!(expand_path("relative"), PathBuf::from("relative"));
    }

    #[test]
    fn text_and_binary_files_roundtrip_through_base64() {
        let original = b"\x00\x01hello";
        let encoded = BASE64.encode(original);
        assert_eq!(BASE64.decode(encoded).unwrap(), original);
    }
}
