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

    let updated = set_theme_key(&content, theme_name)
        .ok_or_else(|| "Failed to update config: no root object found".to_string())?;

    fs::write(config_path, updated).map_err(|e| format!("Failed to write config: {}", e))
}

/// Inserts or replaces the `"theme"` key in a JSONC document without touching
/// anything else: comments, formatting and the other keys stay exactly as
/// they were. Supports double quotes, single quotes and bare keys.
fn set_theme_key(content: &str, theme_name: &str) -> Option<String> {
    let mut found = false;
    let mut result = String::new();
    for line in content.lines() {
        let t = line.trim_start();
        let is_theme_key = t.starts_with("\"theme\"")
            || t.starts_with("'theme'")
            || t.starts_with("theme:");
        if !found && is_theme_key {
            let indent: String = line.chars().take_while(|c| c.is_whitespace()).collect();
            // Replace only the quoted value, keeping the rest of the line
            // (trailing comma, comments, spacing) untouched.
            if let Some(colon) = line.find(':') {
                let head = &line[colon + 1..];
                let trimmed = head.trim_start();
                let skip = head.len() - trimmed.len();
                if let Some(q) = trimmed.chars().next()
                    && (q == '"' || q == '\'')
                {
                    let open = colon + 1 + skip;
                    if let Some(rel) = line[open + 1..].find(q) {
                        let close = open + 1 + rel;
                        result.push_str(&indent);
                        result.push_str(&line[..open]);
                        result.push('"');
                        result.push_str(theme_name);
                        result.push('"');
                        result.push_str(&line[close + 1..]);
                        found = true;
                    }
                }
            }
            if !found {
                // Fallback: the value is not a simple quoted string; rebuild
                // the whole line without any value.
                result.push_str(&indent);
                result.push_str("\"theme\": \"");
                result.push_str(theme_name);
                result.push('"');
                found = true;
            }
        } else if !is_theme_key {
            result.push_str(line);
        }
        result.push('\n');
    }
    if found {
        return Some(result);
    }

    // No theme key yet: insert it right after the root object's opening brace.
    let idx = content.find('{')?;
    let mut out = String::with_capacity(content.len() + 32);
    out.push_str(&content[..=idx]);
    out.push_str("\n    \"theme\": \"");
    out.push_str(theme_name);
    out.push_str("\",");
    out.push_str(&content[idx + 1..]);
    Some(out)
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

    // Icons are intentionally not exported: they are a per-user font choice,
    // not part of a theme's identity. The merge fills them from the defaults.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_set_theme_replaces_existing_key() {
        let cfg = "{\n    \"ascii\": \"fox.txt\",\n    \"theme\": \"nord\", // current theme\n    \"show_colors\": true\n}\n";
        let out = set_theme_key(cfg, "dracula").unwrap();
        assert!(out.contains("\"theme\": \"dracula\""));
        assert!(out.contains("// current theme"), "comment must survive");
        assert!(out.contains("\"ascii\": \"fox.txt\""));
        assert!(out.contains("\"show_colors\": true"));
    }

    #[test]
    fn test_set_theme_inserts_when_missing() {
        let cfg = "{\n    // my config\n    \"ascii\": \"fox.txt\",\n    \"show_colors\": true\n}\n";
        let out = set_theme_key(cfg, "nord").unwrap();
        assert!(out.contains("\"theme\": \"nord\""));
        assert!(out.contains("// my config"), "comment must survive");
        assert!(out.contains("\"ascii\": \"fox.txt\""));
        // The inserted key is the first one (right after the root brace).
        assert!(out.starts_with("{\n    \"theme\": \"nord\","));
    }

    #[test]
    fn test_set_theme_handles_single_quotes() {
        let cfg = "{\n  'theme': 'nord',\n  'ascii': 'fox.txt'\n}\n";
        let out = set_theme_key(cfg, "dracula").unwrap();
        // The key keeps its original quoting; only the value is replaced.
        assert!(out.contains("'theme': \"dracula\""));
        assert!(out.contains("'ascii': 'fox.txt'"));
    }

    #[test]
    fn test_set_theme_missing_root_object() {
        assert_eq!(set_theme_key("not json at all", "dracula"), None);
    }
}
