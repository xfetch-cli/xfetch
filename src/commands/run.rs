use std::path::Path;
use std::time::Duration;

use crate::addons::model::AddonKind;
use crate::commands::find_command_binary;
use crate::wasm::GuestKind;
use crate::wasm::manifest;

/// Plain-text output returned by a command addon.
#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub lines: Vec<String>,
    pub raw: String,
}

impl CommandOutput {
    fn from_bytes(bytes: &[u8]) -> Self {
        let raw = String::from_utf8_lossy(bytes).to_string();
        let lines = raw.lines().map(str::to_string).collect();
        Self { lines, raw }
    }
}

/// Runs an installed command addon with positional arguments.
pub fn run_command(name: &str, args: &[String]) -> Result<CommandOutput, String> {
    let path = find_command_binary(name)
        .or_else(|| AddonKind::Command.installed_path(name))
        .ok_or_else(|| format!("Command '{name}' is not installed"))?;

    let request = serde_json::json!({
        "version": 1,
        "kind": "command",
        "command": name,
        "args": args,
        "cwd": std::env::current_dir().ok().map(|path| path.display().to_string()),
        "env": {},
    });
    let request = serde_json::to_vec(&request)
        .map_err(|err| format!("Failed to serialize command request: {err}"))?;

    if crate::wasm::is_wasm_file(&path) {
        let output = crate::wasm::run_request(&path, &request, None, GuestKind::Command)?;
        return Ok(CommandOutput::from_bytes(&output));
    }

    let timeout = manifest::load(&path)
        .ok()
        .map(|loaded| Duration::from_millis(loaded.manifest.limits.timeout_ms))
        .unwrap_or_else(|| Duration::from_secs(30));
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = crate::subprocess::run_cmd_with_stdin_timeout(
        Path::new(&path),
        &arg_refs,
        Some(&request),
        Some(timeout),
    )
    .ok_or_else(|| format!("Failed to run command '{}'", path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            format!("Command '{name}' exited with status {}", output.status)
        } else {
            stderr
        });
    }

    Ok(CommandOutput::from_bytes(&output.stdout))
}
