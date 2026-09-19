use crate::config::ConfigProviderConfig;
use crate::extensions::find_extension_binary;
use crate::extensions::types::{ConfigProviderRequest, ConfigProviderResponse};
use crate::subprocess::{guest_timeout, run_cmd_with_stdin_timeout};
use crate::wasm::{self, GuestKind};
use std::time::Duration;

pub fn run_config_provider(
    config: &ConfigProviderConfig,
    current_config: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let extension_path = find_extension_binary(&config.extension)
        .ok_or_else(|| format!("Extension not found: {}", config.extension))?;

    let request = ConfigProviderRequest::new(current_config.clone(), config.args.clone());

    let payload = serde_json::to_vec(&request)
        .map_err(|err| format!("Failed to serialize extension request: {}", err))?;

    // Wasm guests reuse the JSON protocol through the sandboxed runtime; an
    // absent `timeout_secs` lets the manifest's own `timeout_ms` apply.
    if wasm::is_wasm_file(&extension_path) {
        let timeout = config.timeout_secs.map(Duration::from_secs);
        let stdout = wasm::run_request(&extension_path, &payload, timeout, GuestKind::Extension)?;
        let response: ConfigProviderResponse = serde_json::from_slice(&stdout)
            .map_err(|err| format!("Failed to parse extension output: {}", err))?;
        return Ok(response.config);
    }

    let timeout = guest_timeout(config.timeout_secs);
    let output = run_cmd_with_stdin_timeout(&extension_path, &[], Some(&payload), timeout)
        .ok_or_else(|| match timeout {
            Some(d) => format!(
                "Extension '{}' exceeded its timeout of {}s",
                config.extension,
                d.as_secs()
            ),
            None => format!("Failed to run extension '{}'", config.extension),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = if stderr.trim().is_empty() {
            "Extension exited with error".to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(msg);
    }

    let response: ConfigProviderResponse = serde_json::from_slice(&output.stdout)
        .map_err(|err| format!("Failed to parse extension output: {}", err))?;

    Ok(response.config)
}
