use crate::extensions::types::{ConfigProviderRequest, ConfigProviderResponse};
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const EXTENSION_PREFIX: &str = "xfetch-extension-";
const PLUGINS_DIR: &str = "plugins";

fn extension_binary_name(name: &str) -> String {
    if cfg!(target_os = "windows") {
        format!("{}{}.exe", EXTENSION_PREFIX, name)
    } else {
        format!("{}{}", EXTENSION_PREFIX, name)
    }
}

fn find_extension_binary(name: &str) -> Option<PathBuf> {
    let binary_name = extension_binary_name(name);

    let config_dir = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
    let in_plugins_dir = config_dir.join("xfetch").join(PLUGINS_DIR).join(&binary_name);
    if in_plugins_dir.is_file() {
        return Some(in_plugins_dir);
    }

    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let candidate = dir.join(&binary_name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }

    None
}

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
