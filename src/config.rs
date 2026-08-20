use crate::extensions::run_config_provider;
use crate::ui::custom_x::CustomX;
use json_comments::StripComments;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ModuleConfig {
    Simple(String),
    Group {
        title: String,
        modules: Vec<ModuleConfig>,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub theme: Option<String>,
    pub ascii: Option<String>,
    pub logo_path: Option<String>,
    pub modules: Vec<ModuleConfig>,
    pub show_colors: bool,
    pub icons: HashMap<String, String>,
    pub colors: HashMap<String, String>,
    /// Optional per-module display labels for the row key shown in a line.
    /// An entry renames the key; an empty string hides it (icon-only row).
    /// Defaults to the module name when absent.
    pub labels: HashMap<String, String>,
    /// Optional per-module value templates. `{field}` placeholders are
    /// substituted with the module's fields (every module exposes `{value}`
    /// and `{key}`; structured modules add more, e.g. CPU adds `{brand}`,
    /// `{model}`, `{cores}` and `{freq}`). Unknown placeholders render empty;
    /// `{{` and `}}` escape literal braces. An empty map keeps the default
    /// output unchanged.
    pub formats: HashMap<String, String>,
    pub layout: Option<String>,
    pub header_icons: Option<Vec<String>>,
    pub footer_text: Option<String>,
    pub palette_style: Option<String>,
    pub logo_animation: Option<LogoAnimationConfig>,
    /// Intro effects applied to the content lines when the fetch starts
    /// (opt-in: no-op when the effect binary is not installed). Accepts a
    /// single effect or a list — effects play in sequence. Installed as
    /// `xfetch-effect-<plugin>` under the config dir or PATH.
    pub effects: Option<EffectConfigList>,
    pub info_plugins: Vec<InfoPluginConfig>,
    pub config_providers: Vec<ConfigProviderConfig>,
    pub disable_ip_fetching: Option<bool>,
    pub disable_cache: Option<bool>,
    /// WSL OS presentation: "off" | "minimal" | "full" (Linux only).
    pub os_wsl_style: Option<String>,
    pub logo_width: Option<u32>,
    pub logo_height: Option<u32>,
    pub logo_gap: Option<u32>,
    pub logo_kitty: Option<bool>,
    pub logo_color: Option<String>,
    /// Per-row logo colors: row `i` uses `logo_colors[i % len]`; takes
    /// precedence over `logo_color`.
    pub logo_colors: Option<Vec<String>>,
    pub logo_padding: Option<usize>,
    pub logo_type: Option<String>,
    pub show_keys: bool,
    pub key_width: Option<usize>,
    pub daemon: bool,
    pub daemon_min_rows: Option<u32>,
    /// Live stats daemon: pins the fetch block at the top of the terminal and
    /// re-probes a lightweight module subset every `daemon_live_refresh`
    /// seconds. Sibling of `daemon` (the animated-logo daemon); the existing
    /// daemon is untouched. Disable from the terminal with `--no-daemon-live`,
    /// stop it with `--daemon-live-stop`.
    pub daemon_live: bool,
    /// Seconds between live refreshes (0 disables the interval floor; the
    /// platform policy supplies the default).
    pub daemon_live_refresh: Option<u64>,
    /// Modules shown by the live daemon. Defaults to the platform's live set
    /// (`platform/<os>/live.rs`).
    pub daemon_live_modules: Option<Vec<String>>,
    /// Hot reload for the live daemon: watch the config (and the active theme)
    /// and re-apply changes — modules, colors, layout, logo, refresh cadence —
    /// without restarting. Force-enable from the terminal with
    /// `--daemon-live-reload`.
    pub daemon_live_reload: bool,
    pub custom_x: Option<CustomX>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum FramePaths {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct LogoAnimationConfig {
    pub plugin: Option<String>,
    pub fps: Option<u64>,
    pub duration_ms: Option<u64>,
    #[serde(rename = "loop")]
    pub loop_enabled: Option<bool>,
    pub style: Option<String>,
    pub frames_path: Option<FramePaths>,
    /// Safety net in seconds: the core kills the plugin process if it runs
    /// longer. The plugin's own `with_timeout` budget is the primary control;
    /// this caps even uncooperative plugins.
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct InfoPluginConfig {
    pub plugin: String,
    pub args: Option<Value>,
    /// Safety net in seconds: the core kills the plugin process if it runs
    /// longer. The plugin's own `with_timeout` budget is the primary control;
    /// this caps even uncooperative plugins.
    pub timeout_secs: Option<u64>,
}

/// Intro effect applied to the content lines (`"effects"` in config). The
/// effect plugin receives the rendered lines and returns per-frame variants
/// (`xfetch-effect-api`); opt-in — nothing happens when the binary is missing.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct EffectConfig {
    pub plugin: Option<String>,
    pub style: Option<String>,
    pub duration_ms: Option<u64>,
    pub fps: Option<u64>,
    pub args: Option<Value>,
    /// Safety net in seconds: the core kills the effect process if it runs
    /// longer. The effect's own `with_timeout` budget is the primary control.
    pub timeout_secs: Option<u64>,
}

/// The `"effects"` config value: a single effect or a list of effects played
/// in sequence. Kept untagged so existing single-object configs keep working.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum EffectConfigList {
    Single(EffectConfig),
    Multiple(Vec<EffectConfig>),
}

impl Config {
    /// The configured effects in play order (empty when none configured).
    pub fn effects_list(&self) -> Vec<&EffectConfig> {
        match &self.effects {
            None => Vec::new(),
            Some(EffectConfigList::Single(cfg)) => vec![cfg],
            Some(EffectConfigList::Multiple(list)) => list.iter().collect(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConfigProviderConfig {
    pub extension: String,
    pub args: Option<Value>,
    /// Safety net in seconds: the core kills the extension process if it
    /// runs longer. The extension's own `with_timeout` budget is the primary
    /// control; this caps even uncooperative extensions.
    pub timeout_secs: Option<u64>,
}

impl Default for Config {
    fn default() -> Self {
        let mut icons = HashMap::new();
        icons.insert("os".to_string(), "\u{f17c}".to_string());
        icons.insert("kernel".to_string(), "\u{f17c}".to_string());
        icons.insert("hostname".to_string(), "\u{f109}".to_string());
        icons.insert("wm".to_string(), "\u{f08e}".to_string());
        icons.insert("packages".to_string(), "\u{f187}".to_string());
        icons.insert("shell".to_string(), "\u{f0e7}".to_string());
        icons.insert("cpu".to_string(), "\u{f2db}".to_string());
        icons.insert("gpu".to_string(), "\u{f0b9}".to_string());
        icons.insert("memory".to_string(), "\u{e266}".to_string());
        icons.insert("disk".to_string(), "\u{f0a0}".to_string());
        icons.insert("battery".to_string(), "\u{f240}".to_string());
        icons.insert("uptime".to_string(), "\u{f253}".to_string());
        icons.insert("terminal".to_string(), "\u{f0e7}".to_string());
        icons.insert("palette".to_string(), "\u{f0eb}".to_string());

        let mut colors = HashMap::new();
        colors.insert("os".to_string(), "Cyan".to_string());
        colors.insert("kernel".to_string(), "White".to_string());
        colors.insert("wm".to_string(), "Blue".to_string());

        Self {
            theme: None,
            ascii: None,
            logo_path: None,
            modules: vec![
                ModuleConfig::Simple("os".to_string()),
                ModuleConfig::Simple("kernel".to_string()),
                ModuleConfig::Simple("uptime".to_string()),
                ModuleConfig::Simple("packages".to_string()),
                ModuleConfig::Simple("wm".to_string()),
                ModuleConfig::Simple("shell".to_string()),
                ModuleConfig::Simple("disk".to_string()),
                ModuleConfig::Simple("cpu".to_string()),
                ModuleConfig::Simple("gpu".to_string()),
                ModuleConfig::Simple("memory".to_string()),
                ModuleConfig::Simple("battery".to_string()),
            ],
            show_colors: true,
            icons,
            colors,
            labels: HashMap::new(),
            formats: HashMap::new(),
            layout: None,
            header_icons: None,
            footer_text: None,
            palette_style: None,
            logo_animation: None,
            effects: None,
            info_plugins: Vec::new(),
            config_providers: Vec::new(),
            disable_ip_fetching: None,
            disable_cache: None,
            os_wsl_style: None,
            logo_width: None,
            logo_height: None,
            logo_gap: None,
            logo_kitty: None,
            logo_color: None,
            logo_colors: None,
            logo_padding: None,
            logo_type: None,
            show_keys: false,
            key_width: None,
            daemon: false,
            daemon_min_rows: None,
            daemon_live: false,
            daemon_live_refresh: None,
            daemon_live_modules: None,
            daemon_live_reload: false,
            custom_x: None,
        }
    }
}

fn parse_jsonc_file(path: &Path) -> Option<Value> {
    let content = fs::read_to_string(path).ok()?;
    parse_jsonc_str(&content)
}

fn parse_jsonc_str(content: &str) -> Option<Value> {
    let mut stripped = StripComments::new(content.as_bytes());
    let mut clean = String::new();
    stripped.read_to_string(&mut clean).ok()?;
    serde_json::from_str(&clean).ok()
}

fn deep_merge(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base), Value::Object(overlay)) => {
            for (key, value) in overlay {
                if let Some(existing) = base.get_mut(key) {
                    deep_merge(existing, value);
                } else {
                    base.insert(key.clone(), value.clone());
                }
            }
        }
        (Value::String(base_str), Value::String(overlay_str))
            if overlay_str.is_empty() && !base_str.is_empty() =>
        {
            // Don't replace a non-empty string with an empty one
        }
        (base, overlay) => *base = overlay.clone(),
    }
}

pub fn config_search_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
    {
        dirs.push(xdg);
    }
    let legacy = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    if !dirs.contains(&legacy) {
        dirs.push(legacy);
    }
    dirs
}

pub fn config_dir() -> PathBuf {
    config_search_dirs()
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve_theme_path(theme: &str) -> Option<PathBuf> {
    let theme_path = PathBuf::from(theme);
    if theme_path.is_file() {
        return Some(theme_path);
    }

    for config_dir in config_search_dirs() {
        let in_themes_dir = config_dir
            .join("xfetch")
            .join("themes")
            .join(format!("{}.jsonc", theme));
        if in_themes_dir.is_file() {
            return Some(in_themes_dir);
        }
    }

    None
}

pub fn load_config(path: Option<String>) -> Config {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        default_config_path()
    };

    if !config_path.exists() {
        return Config::default();
    }

    let config_value = match parse_jsonc_file(&config_path) {
        Some(v) => v,
        None => return Config::default(),
    };

    let mut config: Config = serde_json::from_value(config_value.clone()).unwrap_or_default();

    if let Some(ref theme_name) = config.theme.clone()
        && let Some(theme_path) = resolve_theme_path(theme_name)
        && let Some(theme_value) = parse_jsonc_file(&theme_path)
    {
        let mut merged = serde_json::json!({});
        deep_merge(
            &mut merged,
            &serde_json::to_value(Config::default()).unwrap_or_default(),
        );
        deep_merge(&mut merged, &config_value);
        deep_merge(&mut merged, &theme_value);

        if let Ok(merged_config) = serde_json::from_value(merged) {
            config = merged_config;
        }
    }

    let config_providers = config.config_providers.clone();
    if !config_providers.is_empty() {
        let mut current = serde_json::to_value(&config).unwrap_or_default();
        for provider in &config_providers {
            if let Ok(modified) = run_config_provider(provider, &current) {
                current = modified;
            }
        }
        config = serde_json::from_value(current).unwrap_or(config);
    }

    config
}

pub fn default_config_path() -> PathBuf {
    config_dir().join("xfetch").join("config.jsonc")
}

pub fn default_themes_dir() -> PathBuf {
    config_dir().join("xfetch").join("themes")
}

/// Template for `--gen-config`, embedded so the binary has no file
/// dependencies. Ships as the `section` layout (the same default the
/// installers used to copy); `--layout` swaps the layout key.
/// Template for `--gen-config`, embedded so the binary has no file
/// dependencies. Ships as the `section` layout (the same default the
/// installers used to copy); `--layout` swaps the layout key. The
/// template file lives in `src/templates/`.
const GEN_CONFIG_TEMPLATE: &str = include_str!("templates/config.jsonc");

pub fn generate_config(
    path: Option<String>,
    logo: Option<&str>,
    layout: Option<&str>,
) -> std::io::Result<PathBuf> {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        default_config_path()
    };

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut out = GEN_CONFIG_TEMPLATE.to_string();

    // `--layout`: swap the layout key for a known layout name. The template
    // ships as `section`, so that stays the default.
    if let Some(layout_name) = layout {
        if crate::ui::is_known_layout(layout_name) {
            out = out.replacen(
                "\"layout\": \"section\"",
                &format!("\"layout\": \"{}\"", layout_name),
                1,
            );
        } else {
            eprintln!(
                "Warning: unknown layout '{}'; keeping 'section'.",
                layout_name
            );
        }
    }

    // Best-effort: fetch the ASCII logo of the detected OS/distro (or the
    // explicit `--logo` choice) from the xfetch-cli/logos catalog, persist it
    // locally and point `ascii` at it. On any failure (no network, catalog
    // error, invalid art) the template is written unchanged, keeping the
    // previous behavior — with a warning when the failure was an explicit
    // user request.
    if let Some((distro_id, art)) = crate::logos::fetch_distro_logo(logo) {
        let logos_dir = config_dir().join("xfetch").join("logos");
        fs::create_dir_all(&logos_dir)?;
        let art_path = logos_dir.join(format!("{distro_id}.txt"));
        fs::write(&art_path, art)?;
        if let Ok(art_json) = serde_json::to_string(&art_path.to_string_lossy()) {
            out = out.replacen('{', &format!("{{\n    \"ascii\": {art_json},"), 1);
        }
    } else if let Some(logo_id) = logo {
        eprintln!(
            "Warning: could not fetch logo '{}' (no network or unknown id); using the default config.",
            logo_id
        );
    }

    fs::write(&config_path, out)?;

    Ok(config_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeout_secs_parses_from_jsonc() {
        let jsonc = r#"{
            // plugins declare their own budget via with_timeout; the core
            // safety net is opt-in per plugin:
            "info_plugins": [
                { "plugin": "weather", "timeout_secs": 30 }
            ],
            "config_providers": [
                { "extension": "layout-override", "timeout_secs": 5 }
            ],
            "logo_animation": {
                "plugin": "animate-logo",
                "timeout_secs": 10
            }
        }"#;
        let parsed = parse_jsonc_str(jsonc).expect("parse jsonc");
        let config: Config = serde_json::from_value(parsed).expect("deserialize config");
        assert_eq!(config.info_plugins[0].timeout_secs, Some(30));
        assert_eq!(config.config_providers[0].timeout_secs, Some(5));
        let anim = config.logo_animation.as_ref().expect("logo animation");
        assert_eq!(anim.timeout_secs, Some(10));
    }

    #[test]
    fn test_timeout_secs_defaults_to_none() {
        let jsonc = r#"{ "info_plugins": [ { "plugin": "weather" } ] }"#;
        let parsed = parse_jsonc_str(jsonc).expect("parse jsonc");
        let config: Config = serde_json::from_value(parsed).expect("deserialize config");
        assert_eq!(config.info_plugins[0].timeout_secs, None);
    }
}
