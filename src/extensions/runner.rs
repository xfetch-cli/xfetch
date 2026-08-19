use crate::config::ConfigProviderConfig;
use crate::extensions::find_extension_binary;
use crate::extensions::types::{ConfigProviderRequest, ConfigProviderResponse};
use crate::subprocess::run_cmd_with_stdin_timeout;
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

    let timeout = config.timeout_secs.map(Duration::from_secs);
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
