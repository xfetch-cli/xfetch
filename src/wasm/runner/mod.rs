//! Guest execution: routes an artifact to the right runtime.
//!
//! `run_request` is the single entry point used by the plugin, effect,
//! extension and command runners. It loads the manifest, resolves the policy and deadline
//! once, then delegates to the core-module (WASI preview 1) or component
//! runner. Both runners return the raw JSON response bytes so the existing
//! protocol parsing in the callers is reused unchanged.

mod component_guest;
mod core_guest;

use crate::wasm::GuestKind;
use crate::wasm::detect::{self, WasmKind};
use crate::wasm::host::HostContext;
use crate::wasm::manifest;
use crate::wasm::policy::Policy;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

/// Runs a wasm artifact with the JSON request and returns its raw response.
/// Plugin, effect and extension responses are JSON; command responses are
/// plain text.
///
/// `timeout` is the user-configured safety cap (`timeout_secs`); when absent,
/// the manifest limit applies, falling back to the runtime default.
pub fn run_request(
    path: &Path,
    request: &[u8],
    timeout: Option<Duration>,
    kind: GuestKind,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path)
        .map_err(|err| format!("Failed to read wasm artifact '{}': {}", path.display(), err))?;

    let header = bytes.get(..8).unwrap_or_default();
    let wasm_kind = detect::kind_from_header(header).ok_or_else(|| {
        format!(
            "'{}' does not look like a WebAssembly binary (bad header)",
            path.display()
        )
    })?;

    let loaded = manifest::load(path)?;
    let name = loaded
        .manifest
        .name
        .clone()
        .unwrap_or_else(|| manifest::display_name(path));

    let effective = timeout.unwrap_or(Duration::from_millis(loaded.manifest.limits.timeout_ms));
    let policy = Policy::from_manifest(&loaded.manifest);
    let ctx = HostContext {
        policy,
        deadline: Instant::now() + effective,
        name: name.clone(),
        kind,
    };

    match wasm_kind {
        WasmKind::CoreModule => core_guest::run(&bytes, &loaded.manifest, ctx, request, kind),
        WasmKind::Component => component_guest::run(&bytes, &loaded.manifest, ctx, request, kind),
    }
}

/// Extracts a WASI exit code from a trap, if the guest terminated through
/// `proc_exit`. Go guests call `proc_exit(0)` when `main` returns, so the
/// caller must treat a zero code as success instead of a trap.
fn exit_code(err: &wasmtime::Error) -> Option<i32> {
    err.downcast_ref::<wasmtime_wasi::I32Exit>()
        .map(|exit| exit.0)
}

/// Maps an execution failure to the user-facing error string, distinguishing
/// deadline hits from genuine traps.
fn map_execution_error(
    name: &str,
    deadline: Instant,
    timeout: Duration,
    err: wasmtime::Error,
) -> String {
    if Instant::now() >= deadline {
        format!(
            "Wasm guest '{}' exceeded its timeout of {}s",
            name,
            timeout.as_secs()
        )
    } else {
        format!("Wasm guest '{}' failed: {}", name, err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guest_kind_labels_are_stable() {
        assert_eq!(GuestKind::Plugin.as_str(), "plugin");
        assert_eq!(GuestKind::Effect.as_str(), "effect");
        assert_eq!(GuestKind::Extension.as_str(), "extension");
        assert_eq!(GuestKind::Command.as_str(), "command");
    }
}
