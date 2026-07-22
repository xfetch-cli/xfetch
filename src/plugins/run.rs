use crate::config::{InfoPluginConfig, LogoAnimationConfig};
use crate::plugins::find_plugin_binary;
use std::io::Write;
use std::process::{Command, Stdio};
use xfetch_plugin_api::{
    AnimationFrame, InfoPluginRequest, InfoPluginResponse, LogoAnimationArgs, LogoAnimationRequest,
    LogoAnimationResponse, parse_json_slice, to_json_vec,
};

fn run_plugin_raw(plugin_name: &str, payload: &[u8]) -> Result<Vec<u8>, String> {
    let plugin_path = find_plugin_binary(plugin_name)
        .ok_or_else(|| format!("Plugin not found: {}", plugin_name))?;

    let mut child = Command::new(plugin_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("Failed to start plugin: {}", err))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(payload)
            .map_err(|err| format!("Failed to send plugin request: {}", err))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|err| format!("Failed to read plugin output: {}", err))?;

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

    let stdout = run_plugin_raw(plugin_name, &payload)?;

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

    let stdout = run_plugin_raw(&config.plugin, &payload)?;

    let response: InfoPluginResponse = parse_json_slice(&stdout)
        .map_err(|err| format!("Failed to parse plugin output: {}", err))?;

    Ok(response.lines)
}
