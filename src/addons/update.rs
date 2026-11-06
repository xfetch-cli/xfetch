//! Shared install-all, update and rollback flow for the three addon families.
//!
//! Native addons are still built from source. Update reads the install receipt
//! written by the installer, compares the recorded version with the catalog and
//! rebuilds from the original source. The previous artifact is copied to
//! `<binary>.bak` before replacement; a failed build leaves it untouched.

use crate::addons::catalog::{self, CatalogEntry};
use crate::addons::model::AddonKind;
use crate::addons::receipt::{self, InstallReceipt};
use crate::runtime;
use semver::Version;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Options for `plugin/effects/extension update`.
#[derive(Debug, Default)]
pub struct UpdateOptions {
    /// Update every installed addon of this family.
    pub all: bool,
    /// Report only; do not install.
    pub check: bool,
    /// Reinstall even when the catalog version is not newer.
    pub force: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct UpdateCheck {
    pub name: String,
    pub installed: Option<String>,
    pub available: Option<String>,
    pub update_available: bool,
    pub compatible: bool,
    pub min_xfetch_version: Option<String>,
}

/// Machine-readable update check used by `--json`.
pub fn check(kind: AddonKind, name: Option<&str>, all: bool) -> Result<Vec<UpdateCheck>, String> {
    if all && name.is_some() {
        return Err("Use either an addon name or --all, not both.".to_string());
    }
    if !all && name.is_none() {
        return Err(format!("Specify a {} name or use --all.", kind.label()));
    }

    let targets = if all {
        installed_names(kind)?
    } else {
        vec![name.expect("validated above").to_string()]
    };
    let entries = catalog::all(kind)?;
    let mut checks = Vec::new();

    for target in targets {
        let entry = entries
            .iter()
            .find(|entry| entry.name == target || entry.install_name() == target)
            .cloned();
        let receipt = InstallReceipt::load(kind, &target);
        let installed = receipt
            .as_ref()
            .and_then(|receipt| receipt.version.clone())
            .or_else(|| installed_manifest_version(kind, &target));
        let available = entry.as_ref().and_then(|entry| entry.version.clone());
        let compatible = entry.as_ref().map(meets_min_core).unwrap_or(true);
        let update_available = compatible && is_newer(installed.as_deref(), available.as_deref());
        checks.push(UpdateCheck {
            name: target,
            installed,
            available,
            update_available,
            compatible,
            min_xfetch_version: entry.and_then(|entry| entry.min_xfetch_version),
        });
    }

    Ok(checks)
}

/// Installs every addon advertised by the catalog, skipping installed ones
/// unless `force` is set. Returns the names that were installed.
pub fn install_all(kind: AddonKind, force: bool, json: bool) -> Result<Vec<String>, String> {
    let entries = catalog::all(kind)?;
    if entries.is_empty() {
        if !json {
            runtime::info(format!(
                "No {} available in the catalog.",
                kind.plural_label()
            ));
        }
        return Ok(Vec::new());
    }

    let mut installed = 0usize;
    let mut skipped = 0usize;
    let mut installed_names = Vec::new();
    for entry in entries {
        let name = entry.install_name().to_string();
        if !force && kind.installed_path(&name).is_some() {
            if !json {
                runtime::info(format!(
                    "Skipping {} '{}' (already installed).",
                    kind.label(),
                    name
                ));
            }
            skipped += 1;
            continue;
        }

        install_one(kind, &name, entry.repo.as_deref(), None)?;
        receipt::record(
            kind,
            &name,
            "catalog",
            entry.repo.as_deref(),
            entry.path.as_deref(),
            Some(&entry),
        )?;
        if !json {
            runtime::info(format!("Installed {} '{}'.", kind.label(), name));
        }
        installed += 1;
        installed_names.push(name);
    }

    if !json {
        runtime::info(format!(
            "Installed {} {}; skipped {} already installed.",
            installed,
            kind.plural_label(),
            skipped
        ));
    }
    Ok(installed_names)
}

/// Updates one addon, or every installed addon when `--all` is set.
///
/// Returns `true` when at least one update was available (used by `--check`).
pub fn run(kind: AddonKind, name: Option<&str>, options: &UpdateOptions) -> Result<bool, String> {
    if options.all && name.is_some() {
        return Err("Use either an addon name or --all, not both.".to_string());
    }
    if !options.all && name.is_none() {
        return Err(format!("Specify a {} name or use --all.", kind.label()));
    }

    let targets = if options.all {
        installed_names(kind)?
    } else {
        vec![name.expect("validated above").to_string()]
    };
    if targets.is_empty() {
        runtime::info(format!("No {} installed.", kind.plural_label()));
        return Ok(false);
    }

    let entries = catalog::all(kind)?;
    let mut any = false;

    for target in targets {
        let entry = entries
            .iter()
            .find(|entry| entry.name == target || entry.install_name() == target)
            .cloned();
        let receipt = InstallReceipt::load(kind, &target);
        let installed_version = receipt
            .as_ref()
            .and_then(|receipt| receipt.version.clone())
            .or_else(|| installed_manifest_version(kind, &target));

        let available_version = entry.as_ref().and_then(|entry| entry.version.as_deref());
        if !options.force && !is_newer(installed_version.as_deref(), available_version) {
            runtime::info(format!(
                "{} '{}' is up to date ({}).",
                title(kind),
                target,
                installed_version.as_deref().unwrap_or("unknown")
            ));
            continue;
        }

        if let Some(entry) = &entry
            && !meets_min_core(entry)
        {
            runtime::info(format!(
                "{} '{}' requires xfetch {}, skipping.",
                title(kind),
                target,
                entry.min_xfetch_version.as_deref().unwrap_or("?")
            ));
            continue;
        }

        any = true;
        let from = installed_version.as_deref().unwrap_or("unknown");
        let to = available_version.unwrap_or("unknown");
        if options.check {
            runtime::info(format!("{} '{}': {} -> {}", title(kind), target, from, to));
            continue;
        }

        if receipt.is_none() && !options.force {
            eprintln!(
                "{} '{}' has no install receipt; re-run with --force to rebuild it.",
                title(kind),
                target
            );
            continue;
        }

        let source = match resolve_source(kind, &target, receipt.as_ref(), entry.as_ref()) {
            Ok(source) => source,
            Err(err) => {
                eprintln!("{} '{}' skipped: {}", title(kind), target, err);
                continue;
            }
        };

        backup_installed(kind, &target)?;
        if let Err(err) = source.install(kind, &target) {
            eprintln!(
                "{} '{}' update failed: {}. Keeping the previous installation.",
                title(kind),
                target,
                err
            );
            continue;
        }

        receipt::record(
            kind,
            &target,
            source.label(),
            source.repo(),
            source.path(),
            entry.as_ref(),
        )?;
        runtime::info(format!(
            "Updated {} '{}' ({} -> {}).",
            kind.label(),
            target,
            from,
            to
        ));
    }

    Ok(any)
}

/// Restores the previous artifact saved by the last update.
pub fn rollback(kind: AddonKind, name: &str) -> Result<(), String> {
    let path = kind
        .installed_path(name)
        .ok_or_else(|| format!("{} '{}' is not installed.", title(kind), name))?;
    let backup = backup_path(&path);
    if !backup.is_file() {
        return Err(format!(
            "No previous version for {} '{}' (expected '{}').",
            kind.label(),
            name,
            backup.display()
        ));
    }
    crate::atomic_fs::copy_atomic(&backup, &path)
        .map_err(|err| format!("Failed to restore '{}': {}", path.display(), err))?;

    if crate::wasm::is_wasm_file(&path) {
        restore_if_present(kind, &kind.directory().join(kind.manifest_name(name)))?;
    }
    restore_if_present(kind, &kind.receipt_path(name))?;

    Ok(())
}

fn restore_if_present(kind: AddonKind, path: &Path) -> Result<(), String> {
    let backup = kind.backup_path(path);
    if backup.is_file() {
        crate::atomic_fs::copy_atomic(&backup, path)
            .map_err(|err| format!("Failed to restore '{}': {}", path.display(), err))?;
    }
    Ok(())
}

fn installed_names(kind: AddonKind) -> Result<Vec<String>, String> {
    let entries = match kind {
        AddonKind::Plugin => crate::plugins::list_plugins()
            .map(|entries| entries.into_iter().map(|(name, _)| name).collect())?,
        AddonKind::Effect => crate::effects::list_effects()
            .map(|entries| entries.into_iter().map(|(name, _)| name).collect())?,
        AddonKind::Extension => crate::extensions::list_extensions()
            .map(|entries| entries.into_iter().map(|(name, _)| name).collect())?,
        AddonKind::Command => crate::commands::list_commands()
            .map(|entries| entries.into_iter().map(|(name, _)| name).collect())?,
    };
    Ok(entries)
}

fn installed_manifest_version(kind: AddonKind, name: &str) -> Option<String> {
    let path = kind.installed_path(name)?;
    if !crate::wasm::is_wasm_file(&path) {
        return None;
    }
    crate::wasm::manifest::load(&path)
        .ok()
        .and_then(|loaded| loaded.manifest.version)
}

fn is_newer(installed: Option<&str>, available: Option<&str>) -> bool {
    let Some(available) = available else {
        return false;
    };
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

fn meets_min_core(entry: &CatalogEntry) -> bool {
    let Some(minimum) = entry.min_xfetch_version.as_deref() else {
        return true;
    };
    let (Ok(current), Ok(minimum)) = (
        Version::parse(env!("CARGO_PKG_VERSION")),
        Version::parse(minimum.trim_start_matches('v')),
    ) else {
        return true;
    };
    current >= minimum
}

fn install_one(
    kind: AddonKind,
    source: &str,
    repo: Option<&str>,
    sha256: Option<&str>,
) -> Result<(), String> {
    match kind {
        AddonKind::Plugin => crate::plugins::install_plugin(source, repo, sha256),
        AddonKind::Effect => crate::effects::install_effect(source, repo, sha256),
        AddonKind::Extension => crate::extensions::install_extension(source, repo, sha256),
        AddonKind::Command => crate::commands::install_command(source, repo, sha256),
    }
}

/// Where the next version should be built from.
enum UpdateSource {
    Catalog(Box<CatalogEntry>),
    Local(String),
    Url(String),
    Repo { name: String, repo: Option<String> },
}

impl UpdateSource {
    fn install(&self, kind: AddonKind, name: &str) -> Result<(), String> {
        match self {
            Self::Catalog(entry) => {
                install_one(kind, entry.install_name(), entry.repo.as_deref(), None)
            }
            Self::Local(path) => install_one(kind, path, None, None),
            Self::Url(url) => install_one(kind, url, None, None),
            Self::Repo {
                name: source_name,
                repo,
            } => install_one(kind, source_name, repo.as_deref(), None).or_else(|_| {
                // Some receipts store the installed name separately from the
                // package name; retry with the requested name.
                install_one(kind, name, repo.as_deref(), None)
            }),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Self::Catalog(_) => "catalog",
            Self::Local(_) => "local",
            Self::Url(_) => "url",
            Self::Repo { .. } => "repo",
        }
    }

    fn repo(&self) -> Option<&str> {
        match self {
            Self::Catalog(entry) => entry.repo.as_deref(),
            Self::Repo { repo, .. } => repo.as_deref(),
            Self::Local(_) | Self::Url(_) => None,
        }
    }

    fn path(&self) -> Option<&str> {
        match self {
            Self::Catalog(entry) => entry.path.as_deref(),
            Self::Local(path) | Self::Url(path) => Some(path),
            Self::Repo { name, .. } => Some(name),
        }
    }
}

fn resolve_source(
    _kind: AddonKind,
    name: &str,
    receipt: Option<&InstallReceipt>,
    entry: Option<&CatalogEntry>,
) -> Result<UpdateSource, String> {
    if let Some(receipt) = receipt {
        match receipt.source.as_str() {
            "local" => {
                let path = receipt
                    .path
                    .clone()
                    .ok_or_else(|| "receipt has no local path".to_string())?;
                return Ok(UpdateSource::Local(path));
            }
            "url" => {
                let url = receipt
                    .path
                    .clone()
                    .ok_or_else(|| "receipt has no URL".to_string())?;
                return Ok(UpdateSource::Url(url));
            }
            "repo" => {
                return Ok(UpdateSource::Repo {
                    name: receipt.path.clone().unwrap_or_else(|| name.to_string()),
                    repo: receipt.repo.clone(),
                });
            }
            _ => {}
        }
    }

    if let Some(entry) = entry {
        return Ok(UpdateSource::Catalog(Box::new(entry.clone())));
    }

    Ok(UpdateSource::Repo {
        name: name.to_string(),
        repo: receipt.and_then(|receipt| receipt.repo.clone()),
    })
}

fn backup_installed(kind: AddonKind, name: &str) -> Result<(), String> {
    let Some(path) = kind.installed_path(name) else {
        return Ok(());
    };
    let backup = backup_path(&path);
    fs::copy(&path, &backup)
        .map_err(|err| format!("Failed to back up '{}': {}", path.display(), err))?;

    if crate::wasm::is_wasm_file(&path) {
        backup_if_present(kind, &kind.directory().join(kind.manifest_name(name)))?;
    }
    backup_if_present(kind, &kind.receipt_path(name))?;
    Ok(())
}

fn backup_if_present(kind: AddonKind, path: &Path) -> Result<(), String> {
    if path.is_file() {
        fs::copy(path, kind.backup_path(path))
            .map_err(|err| format!("Failed to back up '{}': {}", path.display(), err))?;
    }
    Ok(())
}

fn backup_path(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("addon");
    path.with_file_name(format!("{}.bak", file_name))
}

fn title(kind: AddonKind) -> &'static str {
    match kind {
        AddonKind::Plugin => "Plugin",
        AddonKind::Effect => "Effect",
        AddonKind::Extension => "Extension",
        AddonKind::Command => "Command",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison_handles_versions_and_fallbacks() {
        assert!(is_newer(Some("0.2.0"), Some("0.3.0")));
        assert!(!is_newer(Some("0.3.0"), Some("0.2.0")));
        assert!(!is_newer(Some("0.3.0"), Some("0.3.0")));
        assert!(is_newer(None, Some("0.3.0")));
        assert!(!is_newer(Some("0.3.0"), None));
        assert!(is_newer(Some("dev"), Some("release")));
    }

    #[test]
    fn backup_path_is_next_to_the_artifact() {
        let path = Path::new("/tmp/xfetch-plugin-weather");
        assert_eq!(
            backup_path(path),
            PathBuf::from("/tmp/xfetch-plugin-weather.bak")
        );
    }
}
