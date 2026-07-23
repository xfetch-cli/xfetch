use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{Config, default_themes_dir, resolve_theme_path};

fn extract_theme_name(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
}

pub fn list_themes() -> Result<Vec<(String, PathBuf)>, String> {
    let mut themes = Vec::new();

    let themes_dir = default_themes_dir();
    if themes_dir.is_dir() {
        let entries = fs::read_dir(&themes_dir)
            .map_err(|e| format!("Failed to read themes directory: {}", e))?;

        for entry in entries {
            let entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("jsonc")
                && let Some(name) = extract_theme_name(&path)
            {
                themes.push((name, path));
            }
        }
    }

    themes.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(themes)
}

pub fn remove_theme(name: &str) -> Result<(), String> {
    if let Some(theme_path) = resolve_theme_path(name) {
        if theme_path.starts_with(default_themes_dir()) {
            fs::remove_file(&theme_path)
                .map_err(|e| format!("Failed to remove theme '{}': {}", name, e))
        } else {
            Err(format!(
                "Theme '{}' is in a protected location and cannot be removed",
                name
            ))
        }
    } else {
        Err(format!("Theme '{}' not found", name))
    }
}

pub fn set_active_theme(config_path: &Path, theme_name: &str) -> Result<(), String> {
    if resolve_theme_path(theme_name).is_none() {
        return Err(format!("Theme '{}' not found", theme_name));
    }

    let content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read config: {}", e))?;

    let mut config: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse config: {}", e))?;

    if let serde_json::Value::Object(ref mut obj) = config {
        obj.insert(
            "theme".to_string(),
            serde_json::Value::String(theme_name.to_string()),
        );
    }

    let updated = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(config_path, updated).map_err(|e| format!("Failed to write config: {}", e))
}

pub fn export_current_theme(config: &Config, name: &str) -> Result<PathBuf, String> {
    let mut theme = serde_json::Map::new();

    if let Some(ref layout) = config.layout {
        theme.insert(
            "layout".to_string(),
            serde_json::Value::String(layout.clone()),
        );
    }

    if !config.colors.is_empty() {
        let colors: serde_json::Value = serde_json::to_value(&config.colors)
            .map_err(|e| format!("Failed to serialize colors: {}", e))?;
        theme.insert("colors".to_string(), colors);
    }

    if !config.icons.is_empty() {
        let icons: serde_json::Value = serde_json::to_value(&config.icons)
            .map_err(|e| format!("Failed to serialize icons: {}", e))?;
        theme.insert("icons".to_string(), icons);
    }

    if let Some(ref style) = config.palette_style {
        theme.insert(
            "palette_style".to_string(),
            serde_json::Value::String(style.clone()),
        );
    }

    if let Some(ref hdr) = config.header_icons {
        let hdr_val: serde_json::Value = serde_json::to_value(hdr)
            .map_err(|e| format!("Failed to serialize header_icons: {}", e))?;
        theme.insert("header_icons".to_string(), hdr_val);
    }

    if let Some(ref footer) = config.footer_text {
        theme.insert(
            "footer_text".to_string(),
            serde_json::Value::String(footer.clone()),
        );
    }

    if let Some(ref logo) = config.logo_path {
        theme.insert(
            "logo_path".to_string(),
            serde_json::Value::String(logo.clone()),
        );
    }

    if config.show_colors {
        theme.insert("show_colors".to_string(), serde_json::Value::Bool(true));
    }

    let theme_json = serde_json::to_string_pretty(&theme)
        .map_err(|e| format!("Failed to serialize theme: {}", e))?;

    let themes_dir = default_themes_dir();
    fs::create_dir_all(&themes_dir)
        .map_err(|e| format!("Failed to create themes directory: {}", e))?;

    let theme_path = themes_dir.join(format!("{}.jsonc", name));
    fs::write(&theme_path, theme_json).map_err(|e| format!("Failed to write theme file: {}", e))?;

    Ok(theme_path)
}
