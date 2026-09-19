use crate::config::{InfoPluginConfig, LogoAnimationConfig};
use crate::plugins::find_plugin_binary;
use crate::subprocess::{guest_timeout, run_cmd_with_stdin_timeout};
use crate::wasm::{self, GuestKind};
use std::time::Duration;
use xfetch_plugin_api::{
    AnimationFrame, InfoPluginRequest, InfoPluginResponse, LogoAnimationArgs, LogoAnimationRequest,
    LogoAnimationResponse, parse_json_slice, to_json_vec,
};

fn run_plugin_raw(
    plugin_name: &str,
    payload: &[u8],
    timeout_secs: Option<u64>,
) -> Result<Vec<u8>, String> {
    let plugin_path = find_plugin_binary(plugin_name)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_name))?;

    // Wasm guests reuse the JSON protocol; the runtime enforces the manifest
    // capabilities and limits instead of spawning a native process, and falls
    // back to the manifest's own `timeout_ms` when the config sets none.
    if wasm::is_wasm_file(&plugin_path) {
        let timeout = timeout_secs.map(Duration::from_secs);
        return wasm::run_request(&plugin_path, payload, timeout, GuestKind::Plugin);
    }

    let timeout = guest_timeout(timeout_secs);
    let output =
        run_cmd_with_stdin_timeout(&plugin_path, &[], Some(payload), timeout).ok_or_else(|| {
            match timeout {
                Some(d) => format!(
                    "Plugin '{}' exceeded its timeout of {}s",
                    plugin_name,
                    d.as_secs()
                ),
                None => format!("Failed to run plugin '{}'", plugin_name),
            }
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let msg = if stderr.trim().is_empty() {
            "Plugin exited with error".to_string()
        } else {
            stderr.trim().to_string()
        };
        return Err(msg);
    }

    Ok(output.stdout)
}

pub fn run_logo_animation_plugin(
    plugin_name: &str,
    config: &LogoAnimationConfig,
    lines: &[String],
    frames: Option<Vec<Vec<String>>>,
) -> Result<Vec<AnimationFrame>, String> {
    let request = LogoAnimationRequest::new(
        lines.to_vec(),
        frames,
        LogoAnimationArgs {
            fps: config.fps,
            duration_ms: config.duration_ms,
            loop_enabled: config.loop_enabled,
            style: config.style.clone(),
        },
    );

    let payload = to_json_vec(&request)
        .map_err(|err| format!("Failed to serialize plugin request: {}", err))?;

    let stdout = run_plugin_raw(plugin_name, &payload, config.timeout_secs)?;

    let response: LogoAnimationResponse = parse_json_slice(&stdout)
        .map_err(|err| format!("Failed to parse plugin output: {}", err))?;

    response
        .validate()
        .map_err(|err| format!("Invalid plugin response: {}", err))?;

    Ok(response.frames)
}

pub fn run_info_plugin(config: &InfoPluginConfig) -> Result<Vec<String>, String> {
    let request = InfoPluginRequest::new(config.args.clone());

    let payload = to_json_vec(&request)
        .map_err(|err| format!("Failed to serialize plugin request: {}", err))?;

    let stdout = run_plugin_raw(&config.plugin, &payload, config.timeout_secs)?;

    let response: InfoPluginResponse = parse_json_slice(&stdout)
        .map_err(|err| format!("Failed to parse plugin output: {}", err))?;

    Ok(response.lines)
}
