//! Human-readable output for addon commands.

use crate::addons::model::{AddonEntry, AddonKind};
use crate::config::Config;
use crate::runtime;

/// Prints the installed addon list. The default layout is unchanged for
/// scripts; `verbose` adds kind, runtime, version, capabilities and state.
pub fn print_list(kind: AddonKind, entries: &[AddonEntry], verbose: bool) {
    if entries.is_empty() {
        println!("No {} installed.", kind.plural_label());
        println!("{} directory: {}", title(kind), kind.directory().display());
        return;
    }

    if !verbose {
        println!("Installed {}:", kind.plural_label());
        for entry in entries {
            println!("  {}  ({})", entry.name, entry.path.display());
        }
        return;
    }

    println!("Installed {}:", kind.plural_label());
    for entry in entries {
        let version = entry.version().unwrap_or("-");
        let state = if entry.enabled {
            "enabled"
        } else {
            "not enabled"
        };
        println!(
            "  {}  ({}, {}, v{}, {})",
            entry.name,
            entry.manifest_kind(),
            entry.runtime.as_str(),
            version,
            state
        );
        println!("    path: {}", entry.path.display());
        if let Some(description) = entry.description() {
            println!("    description: {}", description);
        }
        for line in entry.capability_lines() {
            println!("    {}", line);
        }
        if let Some(limits) = entry.limits_line() {
            println!("    limits: {}", limits);
        }
    }
}

/// Prints one installed addon with its manifest, capabilities and the config
/// snippet required to enable it.
pub fn print_info(entry: &AddonEntry) {
    println!("Name: {}", entry.name);
    println!("Type: {} ({})", entry.kind.label(), entry.manifest_kind());
    println!("Runtime: {}", entry.runtime.as_str());
    if let Some(version) = entry.version() {
        println!("Version: {}", version);
    }
    if let Some(description) = entry.description() {
        println!("Description: {}", description);
    }
    println!("Path: {}", entry.path.display());
    if let Some(source) = &entry.manifest_source {
        println!("Manifest: {}", source);
    }
    if let Some(receipt) = &entry.receipt {
        println!("Install source: {} ({})", receipt.source, receipt.runtime);
        if let Some(repo) = &receipt.repo {
            println!("Repository: {}", repo);
        }
        if let Some(reference) = &receipt.reference {
            println!("Reference: {}", reference);
        }
    }
    println!("Enabled: {}", if entry.enabled { "yes" } else { "no" });

    println!("Capabilities:");
    for line in entry.capability_lines() {
        println!("  {}", line);
    }
    if let Some(limits) = entry.limits_line() {
        println!("Limits: {}", limits);
    }

    println!("Configuration:");
    for line in entry.config_snippet() {
        println!("  {}", line);
    }
}

/// Prints the config snippet and declared capabilities after a successful
/// install. The installer has already printed the artifact path.
pub fn print_install_guidance(kind: AddonKind, name: &str) {
    if runtime::quiet() {
        return;
    }
    let path = kind
        .installed_path(name)
        .unwrap_or_else(|| kind.directory().join(kind.wasm_name(name)));
    let entry = AddonEntry::load(kind, name, &path, &Config::default());

    println!();
    println!("To enable '{}':", name);
    for line in entry.config_snippet() {
        println!("{}", line);
    }
    if let Some(limits) = entry.limits_line() {
        println!();
        println!("Limits: {}", limits);
    }
    let capabilities = entry.capability_lines();
    if !capabilities.is_empty() {
        println!("Capabilities:");
        for line in capabilities {
            println!("  {}", line);
        }
    }
}

fn title(kind: AddonKind) -> &'static str {
    match kind {
        AddonKind::Plugin => "Plugin",
        AddonKind::Effect => "Effect",
        AddonKind::Extension => "Extension",
    }
}
