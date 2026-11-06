use crate::cli::output;
use crate::config::{default_config_path, default_themes_dir};
use crate::error::XFetchError;
use serde_json::{Value, json};
use std::path::PathBuf;

pub fn collect(config_path: Option<String>) -> Value {
    let config = config_path
        .map(PathBuf::from)
        .unwrap_or_else(default_config_path);
    let config_dir = crate::config::config_dir().join("xfetch");
    let cache = dirs::cache_dir().map(|dir| dir.join("xfetch").join("cache.json"));
    let data = dirs::data_dir().map(|dir| dir.join("xfetch"));

    json!({
        "schema_version": output::JSON_SCHEMA_VERSION,
        "config": config.display().to_string(),
        "config_dir": config_dir.display().to_string(),
        "themes": default_themes_dir().display().to_string(),
        "plugins": crate::plugins::default_plugin_dir().display().to_string(),
        "effects": crate::effects::default_effect_dir().display().to_string(),
        "extensions": crate::extensions::default_extension_dir().display().to_string(),
        "backups": crate::cli::backup_cmd::default_backup_dir().display().to_string(),
        "cache": cache.map(|path| path.display().to_string()),
        "data": data.map(|path| path.display().to_string()),
    })
}

pub fn run(config_path: Option<String>, json: bool) -> Result<(), XFetchError> {
    let paths = collect(config_path);
    if json {
        return output::print_json(&paths);
    }

    let entries = [
        ("config", "config"),
        ("config_dir", "config_dir"),
        ("themes", "themes"),
        ("plugins", "plugins"),
        ("effects", "effects"),
        ("extensions", "extensions"),
        ("backups", "backups"),
        ("cache", "cache"),
        ("data", "data"),
    ];
    println!("{:<14} PATH", "KEY");
    for (key, field) in entries {
        let value = paths[field].as_str().unwrap_or("-");
        println!("{key:<14} {value}");
    }
    Ok(())
}
