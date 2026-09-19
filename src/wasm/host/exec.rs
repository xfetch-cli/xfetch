//! Process execution host operation.
//!
//! Only programs matched by the `exec.allow` manifest patterns can run, the
//! environment is cleared and repopulated exclusively from the allowlist, and
//! argv is passed verbatim (no shell is involved at any point). Reuses the
//! shared subprocess machinery so timeouts kill the whole process tree, same
//! as native plugins.

use super::{
    HostCallError, HostCallResult, HostContext, parse_bytes, parse_timeout, require_str, to_base64,
};
use serde_json::{Value, json};
use std::path::Path;
use std::time::Instant;

/// Runs an allowlisted program with the provided stdin and environment.
pub fn run(args: &Value, ctx: &HostContext) -> HostCallResult {
    let program = require_str(args, "program")?;
    if !ctx.policy.exec_allowed(program) {
        return Err(HostCallError::denied(format!(
            "program '{}' is not allowed by the manifest",
            program
        )));
    }

    let argv: Vec<String> = args
        .get("args")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    let stdin = parse_bytes(args, "stdin_base64")?;
    let timeout = parse_timeout(args, ctx)?;
    let env = parse_env(args, ctx);

    let arg_refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let started = Instant::now();
    let output = crate::subprocess::run_cmd_env_with_stdin_timeout(
        Path::new(program),
        &arg_refs,
        Some(&env),
        stdin.as_deref(),
        Some(timeout),
    )
    .ok_or_else(|| execution_error(program, started, timeout))?;

    let cap = ctx.policy.host_call_bytes;
    if output.stdout.len() > cap {
        return Err(HostCallError::too_large(format!(
            "'{}' stdout exceeds the {} KiB host_call limit",
            program,
            cap / 1024
        )));
    }
    if output.stderr.len() > cap {
        return Err(HostCallError::too_large(format!(
            "'{}' stderr exceeds the {} KiB host_call limit",
            program,
            cap / 1024
        )));
    }

    Ok(json!({
        "code": output.status.code().unwrap_or(-1),
        "stdout_base64": to_base64(&output.stdout),
        "stderr_base64": to_base64(&output.stderr),
    }))
}

/// Builds the environment passed to the child: only names present in the
/// manifest `exec.env` allowlist survive.
fn parse_env(args: &Value, ctx: &HostContext) -> Vec<(String, String)> {
    let Some(map) = args.get("env").and_then(Value::as_object) else {
        return Vec::new();
    };

    map.iter()
        .filter(|(name, _)| ctx.policy.exec_env_allowed(name))
        .filter_map(|(name, value)| {
            value
                .as_str()
                .map(|value| (name.clone(), value.to_string()))
        })
        .collect()
}

/// Distinguishes a deadline hit from a spawn failure.
fn execution_error(program: &str, started: Instant, timeout: std::time::Duration) -> HostCallError {
    let elapsed = started.elapsed();
    if elapsed >= timeout.saturating_sub(std::time::Duration::from_millis(100)) {
        HostCallError::timeout(format!("'{}' timed out after {:?}", program, timeout))
    } else {
        HostCallError::failed(format!("failed to start '{}'", program))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wasm::host::{HostContext, HostErrorKind};
    use crate::wasm::manifest::Manifest;
    use crate::wasm::policy::Policy;
    use std::time::{Duration, Instant};

    fn context(json: &str) -> HostContext {
        let manifest: Manifest = serde_json::from_str(json).expect("manifest");
        HostContext {
            policy: Policy::from_manifest(&manifest),
            deadline: Instant::now() + Duration::from_secs(5),
            name: "test".to_string(),
            kind: crate::wasm::GuestKind::Plugin,
        }
    }

    #[test]
    fn denied_program_is_rejected() {
        let ctx = context("{}");
        let err = run(&json!({ "program": "curl" }), &ctx).expect_err("denied");
        assert_eq!(err.kind, HostErrorKind::Denied);
    }

    #[cfg(unix)]
    #[test]
    fn allowlisted_program_returns_output() {
        let ctx = context(r#"{ "capabilities": { "exec": { "allow": ["echo"] } } }"#);
        let value = run(&json!({ "program": "echo", "args": ["hello"] }), &ctx).expect("run");
        assert_eq!(value["code"], 0);
        let stdout = crate::wasm::host::parse_bytes(&value, "stdout_base64")
            .expect("decode")
            .unwrap_or_default();
        assert_eq!(String::from_utf8_lossy(&stdout).trim(), "hello");
    }

    /// `echo` is a cmd builtin, not an executable, so the Windows
    /// counterpart goes through `cmd /c`.
    #[cfg(windows)]
    #[test]
    fn allowlisted_program_returns_output() {
        let ctx = context(r#"{ "capabilities": { "exec": { "allow": ["cmd"] } } }"#);
        let value = run(
            &json!({ "program": "cmd", "args": ["/c", "echo", "hello"] }),
            &ctx,
        )
        .expect("run");
        assert_eq!(value["code"], 0);
        let stdout = crate::wasm::host::parse_bytes(&value, "stdout_base64")
            .expect("decode")
            .unwrap_or_default();
        assert_eq!(String::from_utf8_lossy(&stdout).trim(), "hello");
    }
}
