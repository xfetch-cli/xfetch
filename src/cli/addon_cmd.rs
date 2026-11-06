use crate::addons::install::infer_name;
use crate::addons::model::{AddonEntry, AddonKind};
use crate::addons::{self, catalog};
use crate::cli::args::{AddonCommands, Cli};
use crate::cli::output;
use crate::cli::output::{print_addon_info_json, print_addon_list_json, print_json};
use crate::config::{ConfigOverrides, load_config_with_overrides};
use crate::effects::{install_effect, list_effects, remove_effect};
use crate::error::XFetchError;
use crate::extensions::{install_extension, list_extensions, remove_extension};
use crate::plugins::{install_plugin, list_plugins, remove_plugin};
use crate::runtime;

pub fn run(
    kind: AddonKind,
    action: AddonCommands,
    cli: &Cli,
    overrides: &ConfigOverrides,
    config_path: Option<String>,
) -> Result<(), XFetchError> {
    let config = || load_config_with_overrides(config_path.clone(), overrides);

    match action {
        AddonCommands::Install {
            path,
            repo,
            sha256,
            name,
            manifest,
            all,
            force,
        } => install_addon(
            kind,
            AddonInstallRequest {
                path,
                repo,
                sha256,
                name,
                manifest,
                all,
                force,
            },
            cli.json,
        ),
        AddonCommands::Update {
            name,
            all,
            check,
            force,
        } => update_addon(kind, name, all, check, force, cli.json),
        AddonCommands::Rollback { name } => {
            addons::update::rollback(kind, &name).map_err(XFetchError::Addon)?;
            if cli.json {
                return output::print_json(&serde_json::json!({
                    "schema_version": output::JSON_SCHEMA_VERSION,
                    "status": "rolled-back",
                    "kind": kind.label(),
                    "name": name,
                }));
            }
            runtime::info(format!("Rolled back {} '{}'.", kind.label(), name));
            Ok(())
        }
        AddonCommands::List { verbose } => {
            let installed = list_installed(kind)?;
            let config = config();
            let entries = installed
                .into_iter()
                .map(|(name, path)| AddonEntry::load(kind, &name, &path, &config))
                .collect::<Vec<_>>();
            if cli.json {
                print_addon_list_json(kind, &entries)
            } else {
                addons::report::print_list(kind, &entries, verbose);
                Ok(())
            }
        }
        AddonCommands::Info { name } => {
            let path = find_installed(kind, &name).ok_or_else(|| {
                XFetchError::NotFound(format!("{} '{}' is not installed.", kind.label(), name))
            })?;
            let config = config();
            let entry = AddonEntry::load(kind, &name, &path, &config);
            if cli.json {
                print_addon_info_json(&entry)
            } else {
                addons::report::print_info(&entry);
                Ok(())
            }
        }
        AddonCommands::Search { query } => {
            catalog::print_search(kind, query.as_deref(), cli.json).map_err(XFetchError::Network)
        }
        AddonCommands::Remove { name } => {
            remove_addon(kind, &name)?;
            if cli.json {
                return output::print_json(&serde_json::json!({
                    "schema_version": output::JSON_SCHEMA_VERSION,
                    "status": "removed",
                    "kind": kind.label(),
                    "name": name,
                }));
            }
            runtime::info(format!("Removed {} '{}'", kind.label(), name));
            Ok(())
        }
    }
}

fn list_installed(kind: AddonKind) -> Result<Vec<(String, std::path::PathBuf)>, XFetchError> {
    match kind {
        AddonKind::Plugin => list_plugins(),
        AddonKind::Effect => list_effects(),
        AddonKind::Extension => list_extensions(),
    }
    .map_err(XFetchError::Addon)
}

fn find_installed(kind: AddonKind, name: &str) -> Option<std::path::PathBuf> {
    kind.installed_path(name).or_else(|| match kind {
        AddonKind::Plugin => crate::plugins::find_plugin_binary(name),
        AddonKind::Effect => crate::effects::find_effect_binary(name),
        AddonKind::Extension => crate::extensions::find_extension_binary(name),
    })
}

fn remove_addon(kind: AddonKind, name: &str) -> Result<(), XFetchError> {
    match kind {
        AddonKind::Plugin => remove_plugin(name),
        AddonKind::Effect => remove_effect(name),
        AddonKind::Extension => remove_extension(name),
    }
    .map_err(XFetchError::Addon)
}

struct AddonInstallRequest {
    path: Option<String>,
    repo: Option<String>,
    sha256: Option<String>,
    name: Option<String>,
    manifest: Option<String>,
    all: bool,
    force: bool,
}

fn install_addon(
    kind: AddonKind,
    request: AddonInstallRequest,
    json: bool,
) -> Result<(), XFetchError> {
    let AddonInstallRequest {
        path,
        repo,
        sha256,
        name,
        manifest,
        all,
        force,
    } = request;
    if runtime::offline()
        && (all
            || repo.is_some()
            || path
                .as_deref()
                .is_some_and(|value| is_url(value) || !std::path::Path::new(value).exists()))
    {
        return Err(XFetchError::Network(
            "network access is disabled by --offline".to_string(),
        ));
    }
    if all {
        if path.is_some() || name.is_some() || manifest.is_some() || sha256.is_some() {
            return Err(XFetchError::Usage(
                "--all cannot be combined with a path, --name, --manifest or --sha256".to_string(),
            ));
        }
        let names = addons::update::install_all(kind, force, json).map_err(XFetchError::Addon)?;
        if json {
            let installed: Vec<serde_json::Value> = names
                .iter()
                .map(|name| {
                    serde_json::json!({
                        "name": name,
                        "path": kind
                            .installed_path(name)
                            .map(|path| path.display().to_string())
                            .unwrap_or_default(),
                    })
                })
                .collect();
            return output::print_json(&serde_json::json!({
                "schema_version": output::JSON_SCHEMA_VERSION,
                "status": "installed",
                "kind": kind.label(),
                "installed": installed,
            }));
        }
        return Ok(());
    }

    let path = path.ok_or_else(|| {
        XFetchError::Usage(format!(
            "Specify a {} path/name or use --all.",
            kind.label()
        ))
    })?;
    let direct = name.is_some() || manifest.is_some();
    let installed = if direct {
        if repo.is_some() {
            return Err(XFetchError::Usage(
                "--name/--manifest apply to direct wasm files or URLs; omit --repo".to_string(),
            ));
        }
        addons::install::install_direct(
            kind,
            &path,
            sha256.as_deref(),
            name.as_deref(),
            manifest.as_deref(),
        )
        .map_err(XFetchError::Addon)?
    } else {
        match kind {
            AddonKind::Plugin => install_plugin(&path, repo.as_deref(), sha256.as_deref()),
            AddonKind::Effect => install_effect(&path, repo.as_deref(), sha256.as_deref()),
            AddonKind::Extension => install_extension(&path, repo.as_deref(), sha256.as_deref()),
        }
        .map_err(XFetchError::Addon)?;
        infer_name(kind, &path, None).map_err(XFetchError::Addon)?
    };

    let source = if direct {
        if is_url(&path) { "url" } else { "local" }
    } else if is_url(&path) {
        "url"
    } else if std::path::Path::new(&path).exists() {
        "local"
    } else if addons::catalog::find(kind, &installed)
        .ok()
        .flatten()
        .is_some()
    {
        "catalog"
    } else {
        "repo"
    };
    let catalog_entry = if source == "catalog" {
        addons::catalog::find(kind, &installed).ok().flatten()
    } else {
        None
    };
    addons::receipt::record(
        kind,
        &installed,
        source,
        repo.as_deref(),
        Some(&path),
        catalog_entry.as_ref(),
    )
    .map_err(XFetchError::Addon)?;
    if json {
        return output::print_json(&serde_json::json!({
            "schema_version": output::JSON_SCHEMA_VERSION,
            "status": "installed",
            "kind": kind.label(),
            "name": installed,
            "path": kind
                .installed_path(&installed)
                .map(|path| path.display().to_string())
                .unwrap_or_default(),
            "source": source,
        }));
    }
    addons::report::print_install_guidance(kind, &installed);
    Ok(())
}

fn update_addon(
    kind: AddonKind,
    name: Option<String>,
    all: bool,
    check: bool,
    force: bool,
    json: bool,
) -> Result<(), XFetchError> {
    if json && !check {
        return Err(XFetchError::Usage(
            "--json for addon updates requires --check.".to_string(),
        ));
    }
    if runtime::offline() {
        return Err(XFetchError::Network(
            "network access is disabled by --offline".to_string(),
        ));
    }

    if json {
        let checks =
            addons::update::check(kind, name.as_deref(), all).map_err(XFetchError::Network)?;
        return print_json(&serde_json::json!({
            "schema_version": crate::cli::output::JSON_SCHEMA_VERSION,
            "kind": kind.label(),
            "checks": checks,
        }));
    }

    let options = addons::update::UpdateOptions { all, check, force };
    let available =
        addons::update::run(kind, name.as_deref(), &options).map_err(XFetchError::Addon)?;
    if check && available {
        return Err(XFetchError::Exit(crate::error::EXIT_GENERIC));
    }
    Ok(())
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_urls() {
        assert!(is_url("https://example.com/a.wasm"));
        assert!(is_url("http://example.com/a.wasm"));
        assert!(!is_url("/tmp/a.wasm"));
    }
}
