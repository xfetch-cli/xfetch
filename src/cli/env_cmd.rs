use crate::cli::{build_info_cmd, paths_cmd};
use crate::error::XFetchError;
use crate::runtime;
use serde_json::{Map, Value, json};

pub fn collect(config_path: Option<String>) -> Value {
    let mut environment = Map::new();
    for name in [
        "TERM",
        "TERM_PROGRAM",
        "KITTY_WINDOW_ID",
        "COLORTERM",
        "SHELL",
        "XDG_SESSION_TYPE",
        "XDG_CURRENT_DESKTOP",
        "WAYLAND_DISPLAY",
        "DISPLAY",
        "XDG_CONFIG_HOME",
        "XDG_CACHE_HOME",
        "XDG_STATE_HOME",
    ] {
        if let Ok(value) = std::env::var(name) {
            environment.insert(name.to_string(), Value::String(value));
        }
    }

    json!({
        "schema_version": crate::cli::output::JSON_SCHEMA_VERSION,
        "build": build_info_cmd::collect(),
        "platform": crate::cli::doctor::platform::collect(),
        "environment": environment,
        "settings": {
            "log_level": runtime::log_level().as_str(),
            "log_json": runtime::log_json(),
            "offline": runtime::offline(),
            "no_color": runtime::no_color(),
            "quiet": runtime::quiet(),
            "verbose": runtime::verbose(),
        },
        "paths": paths_cmd::collect(config_path),
    })
}

pub fn run(config_path: Option<String>, json: bool) -> Result<(), XFetchError> {
    let value = collect(config_path);
    if json {
        return crate::cli::output::print_json(&value);
    }

    println!("[build]");
    println!(
        "version:        {}",
        value["build"]["version"].as_str().unwrap_or("?")
    );
    println!(
        "target:         {} {}",
        value["build"]["target"]["os"].as_str().unwrap_or("?"),
        value["build"]["target"]["arch"].as_str().unwrap_or("?")
    );
    println!(
        "profile:        {}",
        value["build"]["profile"].as_str().unwrap_or("?")
    );

    println!();
    println!("[platform]");
    if let Some(details) = value["platform"]["details"].as_array() {
        for detail in details {
            println!(
                "  {:<22} {}",
                detail["name"].as_str().unwrap_or("?"),
                detail["value"].as_str().unwrap_or("?")
            );
        }
    }

    println!();
    println!("[environment]");
    if let Some(environment) = value["environment"].as_object() {
        for (key, value) in environment {
            println!("  {:<22} {}", key, value.as_str().unwrap_or("?"));
        }
    }

    println!();
    println!("[settings]");
    if let Some(settings) = value["settings"].as_object() {
        for (key, value) in settings {
            println!("  {:<22} {}", key, value);
        }
    }

    println!();
    println!("[paths]");
    if let Some(paths) = value["paths"].as_object() {
        for (key, value) in paths {
            if key == "schema_version" {
                continue;
            }
            println!("  {:<22} {}", key, value.as_str().unwrap_or("-"));
        }
    }
    Ok(())
}
