use crate::cli::output;
use crate::error::XFetchError;
use serde_json::{Value, json};

pub fn collect() -> Value {
    json!({
        "schema_version": output::JSON_SCHEMA_VERSION,
        "name": "xfetch",
        "package": env!("CARGO_PKG_NAME"),
        "version": env!("CARGO_PKG_VERSION"),
        "target": {
            "os": std::env::consts::OS,
            "arch": std::env::consts::ARCH,
            "family": std::env::consts::FAMILY,
        },
        "profile": if cfg!(debug_assertions) { "debug" } else { "release" },
        "features": {
            "wasm": cfg!(feature = "wasm"),
            "update": cfg!(feature = "update"),
            "encryption": true,
            "color": true,
        },
        "rust_version": option_env!("CARGO_PKG_RUST_VERSION")
            .filter(|value| !value.is_empty())
            .unwrap_or("unknown"),
    })
}

pub fn run(json: bool) -> Result<(), XFetchError> {
    let value = collect();
    if json {
        return output::print_json(&value);
    }

    println!("name:           {}", value["name"].as_str().unwrap_or("?"));
    println!(
        "package:        {}",
        value["package"].as_str().unwrap_or("?")
    );
    println!(
        "version:        {}",
        value["version"].as_str().unwrap_or("?")
    );
    println!(
        "target:         {} {} ({})",
        value["target"]["os"].as_str().unwrap_or("?"),
        value["target"]["arch"].as_str().unwrap_or("?"),
        value["target"]["family"].as_str().unwrap_or("?")
    );
    println!(
        "profile:        {}",
        value["profile"].as_str().unwrap_or("?")
    );
    println!("wasm support:   {}", value["features"]["wasm"]);
    println!("update support: {}", value["features"]["update"]);
    println!("encryption:     {}", value["features"]["encryption"]);
    println!(
        "rust version:   {}",
        value["rust_version"].as_str().unwrap_or("unknown")
    );
    Ok(())
}
