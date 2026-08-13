use crate::extensions::run_config_provider;
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
    pub layout: Option<String>,
    pub header_icons: Option<Vec<String>>,
    pub footer_text: Option<String>,
    pub palette_style: Option<String>,
    pub logo_animation: Option<LogoAnimationConfig>,
    pub info_plugins: Vec<InfoPluginConfig>,
    pub config_providers: Vec<ConfigProviderConfig>,
    pub disable_ip_fetching: Option<bool>,
    pub disable_cache: Option<bool>,
    pub logo_width: Option<u32>,
    pub logo_height: Option<u32>,
    pub logo_gap: Option<u32>,
    pub logo_kitty: Option<bool>,
    pub daemon: bool,
    pub daemon_min_rows: Option<u32>,
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
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct InfoPluginConfig {
    pub plugin: String,
    pub args: Option<Value>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct ConfigProviderConfig {
    pub extension: String,
    pub args: Option<Value>,
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
            layout: None,
            header_icons: None,
            footer_text: None,
            palette_style: None,
            logo_animation: None,
            info_plugins: Vec::new(),
            config_providers: Vec::new(),
            disable_ip_fetching: None,
            disable_cache: None,
            logo_width: None,
            logo_height: None,
            logo_gap: None,
            logo_kitty: None,
            daemon: false,
            daemon_min_rows: None,
        }
    }
}

fn parse_jsonc_file(path: &Path) -> Option<Value> {
    let file = fs::File::open(path).ok()?;
    let mut stripped = StripComments::new(file);
    let mut content = String::new();
    stripped.read_to_string(&mut content).ok()?;
    serde_json::from_str(&content).ok()
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
        (Value::String(base_str), Value::String(overlay_str)) if overlay_str.is_empty() && !base_str.is_empty() => {
            // Don't replace a non-empty string with an empty one
        }
        (base, overlay) => *base = overlay.clone(),
    }
}

pub fn resolve_theme_path(theme: &str) -> Option<PathBuf> {
    let theme_path = PathBuf::from(theme);
    if theme_path.is_file() {
        return Some(theme_path);
    }

    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let in_themes_dir = config_dir
        .join("xfetch")
        .join("themes")
        .join(format!("{}.jsonc", theme));
    if in_themes_dir.is_file() {
        return Some(in_themes_dir);
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
            if let Ok(modified) = run_config_provider(&provider.extension, provider.args.clone(), &current) {
                current = modified;
            }
        }
        config = serde_json::from_value(current).unwrap_or(config);
    }

    config
}

pub fn default_config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("xfetch").join("config.jsonc")
}

pub fn default_themes_dir() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("xfetch").join("themes")
}

pub fn generate_config(path: Option<String>) -> std::io::Result<PathBuf> {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        default_config_path()
    };

    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let template = include_str!("../configs/layout_pacman.jsonc");
    fs::write(&config_path, template)?;

    Ok(config_path)
}
