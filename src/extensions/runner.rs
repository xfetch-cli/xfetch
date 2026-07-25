use crate::extensions::types::{ConfigProviderRequest, ConfigProviderResponse};
use crate::extensions::find_extension_binary;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

pub fn run_config_provider(
    extension_name: &str,
    args: Option<Value>,
    current_config: &Value,
) -> Result<Value, String> {
    let extension_path = find_extension_binary(extension_name).ok_or_else(|| {
        format!("Extension not found: {}", extension_name)
    })?;

    let request = ConfigProviderRequest::new(current_config.clone(), args);

    let payload = serde_json::to_vec(&request)
        .map_err(|err| format!("Failed to serialize extension request: {}", err))?;

    let mut child = Command::new(&extension_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to start extension: {}", err))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&payload)
            .map_err(|err| format!("Failed to send extension request: {}", err))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to read extension output: {}", err))?;

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
