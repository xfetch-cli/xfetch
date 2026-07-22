use json_comments::StripComments;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::PathBuf;

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ModuleConfig {
    Simple(String),
    Group {
        title: String,
        modules: Vec<ModuleConfig>, // Recursive
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub ascii: Option<String>,
    pub logo_path: Option<String>,
    pub modules: Vec<ModuleConfig>, // Changed from Vec<String>
    pub show_colors: bool,
    pub icons: HashMap<String, String>,
    pub colors: HashMap<String, String>,
    pub layout: Option<String>,
    pub header_icons: Option<Vec<String>>,
    pub footer_text: Option<String>,
    pub palette_style: Option<String>, // "dots", "squares", "lines", "triangles"
    pub logo_animation: Option<LogoAnimationConfig>,
    pub info_plugins: Vec<InfoPluginConfig>,
    pub disable_ip_fetching: Option<bool>,
    pub disable_cache: Option<bool>,
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

impl Default for Config {
    fn default() -> Self {
        let mut icons = HashMap::new();
        icons.insert("os".to_string(), "".to_string()); // Arch icon default
        icons.insert("kernel".to_string(), "".to_string());
        icons.insert("hostname".to_string(), "".to_string()); // Hostname icon
        icons.insert("wm".to_string(), "".to_string());
        icons.insert("packages".to_string(), "".to_string());
        icons.insert("shell".to_string(), "".to_string());
        icons.insert("cpu".to_string(), "".to_string());
        icons.insert("gpu".to_string(), "󰍹".to_string());
        icons.insert("memory".to_string(), "".to_string());
        icons.insert("disk".to_string(), "󰋊".to_string());
        icons.insert("battery".to_string(), "".to_string());
        icons.insert("uptime".to_string(), "󰔛".to_string());
        icons.insert("terminal".to_string(), "".to_string());
        icons.insert("palette".to_string(), "󰨬".to_string()); // Palette icon

        let mut colors = HashMap::new();
        // Default colors (using crossterm color names as strings)
        colors.insert("os".to_string(), "Cyan".to_string());
        colors.insert("kernel".to_string(), "White".to_string());
        colors.insert("wm".to_string(), "Blue".to_string());

        Self {
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
            disable_ip_fetching: None,
            disable_cache: None,
        }
    }
}

pub fn load_config(path: Option<String>) -> Config {
    let config_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        default_config_path()
    };

    // println!("Debug: Config path is {:?}", config_path);

    if !config_path.exists() {
        // println!("Debug: Config file does not exist");
        return Config::default();
    }

    let file = match fs::File::open(&config_path) {
        Ok(f) => f,
        Err(_) => return Config::default(),
    };

    let mut stripped = StripComments::new(file);
    let mut content = String::new();
    if stripped.read_to_string(&mut content).is_err() {
        return Config::default();
    }

    // println!("Debug: Config content: {}", content);

    serde_json::from_str(&content).unwrap_or_default()
}

pub fn default_config_path() -> PathBuf {
    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    config_dir.join("xfetch").join("config.jsonc")
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

    // Keep generated defaults aligned with the curated pacman preset in the repo.
    let template = include_str!("../configs/layout_pacman.jsonc");
    fs::write(&config_path, template)?;

    Ok(config_path)
}
