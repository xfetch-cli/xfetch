//! Addon identity, installed metadata and configuration snippets.

use crate::config::{Config, EffectConfigList, ModuleConfig};
use crate::effects::{
    default_effect_dir, effect_binary_name, effect_manifest_name, effect_wasm_name,
};
use crate::extensions::{
    default_extension_dir, extension_binary_name, extension_manifest_name, extension_wasm_name,
};
use crate::plugins::{
    default_plugin_dir, plugin_binary_name, plugin_manifest_name, plugin_wasm_name,
};
use crate::wasm::detect::is_wasm_file;
use crate::wasm::manifest::{self, Manifest, ManifestSource};
use std::path::{Path, PathBuf};

/// The three installable addon families supported by the core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonKind {
    Plugin,
    Effect,
    Extension,
}

impl AddonKind {
    /// Singular label used in messages.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Effect => "effect",
            Self::Extension => "extension",
        }
    }

    /// Plural label used in list headers.
    pub const fn plural_label(self) -> &'static str {
        match self {
            Self::Plugin => "plugins",
            Self::Effect => "effects",
            Self::Extension => "extensions",
        }
    }

    /// Installed binary prefix (`xfetch-plugin-`, ...).
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::Plugin => "xfetch-plugin-",
            Self::Effect => "xfetch-effect-",
            Self::Extension => "xfetch-extension-",
        }
    }

    /// Default installation directory.
    pub fn directory(self) -> PathBuf {
        match self {
            Self::Plugin => default_plugin_dir(),
            Self::Effect => default_effect_dir(),
            Self::Extension => default_extension_dir(),
        }
    }

    /// Native executable name for `name`.
    pub fn binary_name(self, name: &str) -> String {
        match self {
            Self::Plugin => plugin_binary_name(name),
            Self::Effect => effect_binary_name(name),
            Self::Extension => extension_binary_name(name),
        }
    }

    /// Wasm artifact name for `name`.
    pub fn wasm_name(self, name: &str) -> String {
        match self {
            Self::Plugin => plugin_wasm_name(name),
            Self::Effect => effect_wasm_name(name),
            Self::Extension => extension_wasm_name(name),
        }
    }

    /// Sidecar manifest name for `name`.
    pub fn manifest_name(self, name: &str) -> String {
        match self {
            Self::Plugin => plugin_manifest_name(name),
            Self::Effect => effect_manifest_name(name),
            Self::Extension => extension_manifest_name(name),
        }
    }

    /// Install receipt name for `name`.
    pub fn receipt_name(self, name: &str) -> String {
        format!("{}{}.install.json", self.prefix(), name)
    }

    /// Install receipt path for `name`.
    pub fn receipt_path(self, name: &str) -> PathBuf {
        self.directory().join(self.receipt_name(name))
    }

    /// Backup path used by update/rollback for an installed artifact.
    pub fn backup_path(self, path: &Path) -> PathBuf {
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("addon");
        path.with_file_name(format!("{}.bak", file_name))
    }

    /// Finds the installed native or wasm artifact in the default directory.
    pub fn installed_path(self, name: &str) -> Option<PathBuf> {
        let directory = self.directory();
        let binary = directory.join(self.binary_name(name));
        if binary.is_file() {
            return Some(binary);
        }
        let wasm = directory.join(self.wasm_name(name));
        wasm.is_file().then_some(wasm)
    }
}

/// How an installed addon runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddonRuntime {
    Native,
    Wasm,
}

impl AddonRuntime {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Wasm => "wasm",
        }
    }
}

/// Installed addon metadata assembled from the artifact and its manifest.
#[derive(Debug, Clone)]
pub struct AddonEntry {
    pub kind: AddonKind,
    pub name: String,
    pub path: PathBuf,
    pub runtime: AddonRuntime,
    pub manifest: Option<Manifest>,
    pub manifest_source: Option<String>,
    pub receipt: Option<crate::addons::receipt::InstallReceipt>,
    pub enabled: bool,
}

impl AddonEntry {
    /// Loads an installed addon. Missing manifests are valid: the runtime then
    /// applies the deny-by-default defaults.
    pub fn load(kind: AddonKind, name: &str, path: &Path, config: &Config) -> Self {
        let runtime = if is_wasm_file(path) {
            AddonRuntime::Wasm
        } else {
            AddonRuntime::Native
        };
        // Native addons have no embedded manifest; avoid reading large
        // binaries just to look for one.
        let loaded = if runtime == AddonRuntime::Wasm {
            manifest::load(path).ok()
        } else {
            None
        };
        let manifest = loaded.as_ref().and_then(|loaded| match loaded.source {
            ManifestSource::Default => None,
            _ => Some(loaded.manifest.clone()),
        });
        let manifest_source = loaded.map(|loaded| loaded.source.as_str().to_string());
        let receipt = crate::addons::receipt::InstallReceipt::load(kind, name);
        Self {
            kind,
            name: name.to_string(),
            path: path.to_path_buf(),
            runtime,
            manifest,
            manifest_source,
            receipt,
            enabled: is_enabled(kind, name, config),
        }
    }

    /// Manifest kind, falling back to the addon family.
    pub fn manifest_kind(&self) -> &str {
        self.manifest
            .as_ref()
            .and_then(|manifest| manifest.kind.as_deref())
            .unwrap_or_else(|| fallback_kind(self.kind))
    }

    pub fn version(&self) -> Option<&str> {
        self.manifest
            .as_ref()
            .and_then(|manifest| manifest.version.as_deref())
            .or_else(|| {
                self.receipt
                    .as_ref()
                    .and_then(|receipt| receipt.version.as_deref())
            })
    }

    pub fn description(&self) -> Option<&str> {
        self.manifest
            .as_ref()
            .and_then(|manifest| manifest.description.as_deref())
    }

    /// Human-readable capability summary. Native addons report the process
    /// boundary; wasm addons report the manifest allowlists.
    pub fn capability_lines(&self) -> Vec<String> {
        let Some(manifest) = &self.manifest else {
            return match self.runtime {
                AddonRuntime::Native => vec![
                    "Native process: runs unsandboxed with the current user's privileges."
                        .to_string(),
                ],
                AddonRuntime::Wasm => vec![
                    "No manifest: HTTP, exec, filesystem, environment and args are denied by default."
                        .to_string(),
                ],
            };
        };

        let capabilities = &manifest.capabilities;
        let mut lines = Vec::new();
        lines.push(format!(
            "HTTP allow: {}",
            join_or_none(&capabilities.http.allow)
        ));
        lines.push(format!(
            "Exec allow: {}",
            join_or_none(&capabilities.exec.allow)
        ));
        lines.push(format!(
            "Filesystem: {}",
            filesystem_summary(&capabilities.fs)
        ));
        lines.push(format!("Environment: {}", join_or_none(&capabilities.env)));
        lines.push(format!(
            "Args: {}",
            if capabilities.args {
                "allowed"
            } else {
                "denied"
            }
        ));
        lines
    }

    /// Runtime limits in a single line.
    pub fn limits_line(&self) -> Option<String> {
        let limits = self.manifest.as_ref()?.limits.clone();
        Some(format!(
            "timeout {} ms, memory {} MiB, output {} KiB, host calls {} KiB",
            limits.timeout_ms, limits.memory_mb, limits.output_kb, limits.host_call_kb
        ))
    }

    /// JSONC snippet that enables the addon in the user's config.
    pub fn config_snippet(&self) -> Vec<String> {
        config_snippet(self.kind, &self.name, Some(self.manifest_kind()))
    }
}

/// Snippet for an addon whose manifest kind may be unknown (for example a
/// native plugin installed without a sidecar).
pub fn config_snippet(kind: AddonKind, name: &str, manifest_kind: Option<&str>) -> Vec<String> {
    match kind {
        AddonKind::Plugin => plugin_snippet(name, manifest_kind),
        AddonKind::Effect => vec![
            "\"effects\": [".to_string(),
            format!("    {{ \"plugin\": \"{name}\" }}"),
            "]".to_string(),
        ],
        AddonKind::Extension => vec![
            "\"config_providers\": [".to_string(),
            format!("    {{ \"extension\": \"{name}\" }}"),
            "]".to_string(),
        ],
    }
}

fn plugin_snippet(name: &str, manifest_kind: Option<&str>) -> Vec<String> {
    let info = vec![
        "\"info_plugins\": [".to_string(),
        format!("    {{ \"plugin\": \"{name}\" }}"),
        "],".to_string(),
        "\"modules\": [".to_string(),
        "    ...,".to_string(),
        format!("    \"plugin:{name}\""),
        "]".to_string(),
    ];
    let logo = vec![
        "\"logo_animation\": {".to_string(),
        format!("    \"plugin\": \"{name}\""),
        "}".to_string(),
    ];

    match manifest_kind {
        Some("logo_animation") => logo,
        Some("info_provider") => info,
        _ => {
            let mut lines = vec!["Info provider:".to_string()];
            lines.extend(info.into_iter().map(|line| format!("  {line}")));
            lines.push(String::new());
            lines.push("Logo animation:".to_string());
            lines.extend(logo.into_iter().map(|line| format!("  {line}")));
            lines
        }
    }
}

fn fallback_kind(kind: AddonKind) -> &'static str {
    match kind {
        AddonKind::Plugin => "plugin",
        AddonKind::Effect => "effect",
        AddonKind::Extension => "config_provider",
    }
}

fn join_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "none".to_string()
    } else {
        values.join(", ")
    }
}

fn filesystem_summary(entries: &[crate::wasm::manifest::FsEntry]) -> String {
    use crate::wasm::manifest::FsEntry;

    if entries.is_empty() {
        return "none".to_string();
    }

    entries
        .iter()
        .map(|entry| match entry {
            FsEntry::Path(path) => format!("{path} (read-only)"),
            FsEntry::Mount { host, guest, mode } => {
                format!("{host} -> {guest} ({mode:?})")
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn is_enabled(kind: AddonKind, name: &str, config: &Config) -> bool {
    match kind {
        AddonKind::Plugin => {
            let info_configured = config
                .info_plugins
                .iter()
                .any(|plugin| plugin.plugin == name);
            let info_rendered = modules_include_plugin(&config.modules, name);
            let logo_configured = config
                .logo_animation
                .as_ref()
                .and_then(|animation| animation.plugin.as_deref())
                == Some(name);
            (info_configured && info_rendered) || logo_configured
        }
        AddonKind::Effect => match &config.effects {
            Some(EffectConfigList::Single(effect)) => effect.plugin.as_deref() == Some(name),
            Some(EffectConfigList::Multiple(effects)) => effects
                .iter()
                .any(|effect| effect.plugin.as_deref() == Some(name)),
            None => false,
        },
        AddonKind::Extension => config
            .config_providers
            .iter()
            .any(|provider| provider.extension == name),
    }
}

fn modules_include_plugin(modules: &[ModuleConfig], name: &str) -> bool {
    let key = format!("plugin:{name}");
    modules.iter().any(|module| match module {
        ModuleConfig::Simple(value) => value == &key,
        ModuleConfig::Group { modules, .. } => modules_include_plugin(modules, name),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config_with_modules(modules: Vec<ModuleConfig>) -> Config {
        Config {
            modules,
            ..Config::default()
        }
    }

    #[test]
    fn info_snippet_includes_module_key() {
        let snippet = config_snippet(AddonKind::Plugin, "weather", Some("info_provider"));
        let text = snippet.join("\n");
        assert!(text.contains("\"info_plugins\""));
        assert!(text.contains("\"plugin:weather\""));
    }

    #[test]
    fn logo_snippet_uses_logo_animation() {
        let snippet = config_snippet(AddonKind::Plugin, "animate-logo", Some("logo_animation"));
        let text = snippet.join("\n");
        assert!(text.contains("\"logo_animation\""));
        assert!(!text.contains("\"info_plugins\""));
    }

    #[test]
    fn effect_and_extension_snippets_use_their_keys() {
        let effect = config_snippet(AddonKind::Effect, "matrix", None).join("\n");
        let extension = config_snippet(AddonKind::Extension, "night", None).join("\n");
        assert!(effect.contains("\"effects\""));
        assert!(extension.contains("\"config_providers\""));
    }

    #[test]
    fn info_plugin_enabled_only_when_module_is_present() {
        let mut config = config_with_modules(vec![ModuleConfig::Simple("os".to_string())]);
        config.info_plugins.push(crate::config::InfoPluginConfig {
            plugin: "weather".to_string(),
            args: None,
            timeout_secs: None,
        });
        assert!(!is_enabled(AddonKind::Plugin, "weather", &config));

        config
            .modules
            .push(ModuleConfig::Simple("plugin:weather".to_string()));
        assert!(is_enabled(AddonKind::Plugin, "weather", &config));
    }
}
