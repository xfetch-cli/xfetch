//! WebAssembly support for plugins, effects and extensions.
//!
//! This module is the single place where xfetch understands WebAssembly. It
//! is split into three layers:
//!
//! - Pure data layers (`detect`, `manifest`, `policy`) always compile; they
//!   need no runtime and power `xfetch wasm inspect`.
//! - The execution layer (`engine`, `host`, `runner`) is gated behind the
//!   `wasm` feature (enabled by default) and links wasmtime plus wasmtime-wasi.
//! - Tooling (`install`, `inspect`) wires artifacts into the config directory
//!   and presents them to the user.
//!
//! The authoritative user documentation lives in `docs/WASM.md`; the wire
//! contract for component guests lives in `wit/xfetch-runtime.wit`.

#[cfg(not(feature = "wasm"))]
use std::path::Path;
#[cfg(not(feature = "wasm"))]
use std::time::Duration;

pub mod detect;
pub mod inspect;
pub mod install;
pub mod manifest;
pub mod policy;

#[cfg(feature = "wasm")]
mod engine;
#[cfg(feature = "wasm")]
mod host;
#[cfg(feature = "wasm")]
mod runner;

pub use detect::is_wasm_file;

/// Which contract the guest is being invoked for. Selects the component world
/// and shapes diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuestKind {
    Plugin,
    Effect,
    Extension,
    Command,
}

#[cfg_attr(not(feature = "wasm"), allow(dead_code))]
impl GuestKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Plugin => "plugin",
            Self::Effect => "effect",
            Self::Extension => "extension",
            Self::Command => "command",
        }
    }
}

/// Embedded copy of the component protocol, printable with `xfetch wasm wit`.
///
/// The canonical file lives in the `xfetch-cli/api` repository
/// (`api/wit/xfetch-runtime.wit`); `cargo test` fails when the vendored copy
/// drifts from it in a multi-repo checkout.
pub const WIT: &str = include_str!("../../wit/xfetch-runtime.wit");

/// Runs a wasm artifact with the JSON request and returns its raw response.
/// Plugin, effect and extension responses are JSON; command responses are
/// plain text lines.
///
/// `timeout` is the user-configured safety cap (`timeout_secs`); when absent,
/// the manifest limit applies, falling back to the runtime default.
#[cfg(feature = "wasm")]
pub use runner::run_request;

/// Stub used when xfetch is built without the `wasm` feature: wasm artifacts
/// are detected and reported clearly instead of being spawned as binaries.
#[cfg(not(feature = "wasm"))]
pub fn run_request(
    path: &Path,
    _request: &[u8],
    _timeout: Option<Duration>,
    _kind: GuestKind,
) -> Result<Vec<u8>, String> {
    Err(format!(
        "'{}' is a WebAssembly artifact, but this xfetch binary was built without the 'wasm' \
         feature; rebuild with default features or install a prebuilt release",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The WIT package is canonical in the api repository; the core keeps a
    /// vendored copy for standalone builds. In a multi-repo checkout this test
    /// fails when the copies drift.
    #[test]
    fn vendored_wit_matches_the_api_repository() {
        let canonical_path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../api/wit/xfetch-runtime.wit");
        if !canonical_path.is_file() {
            // Standalone checkout: nothing to compare against.
            return;
        }

        let canonical = std::fs::read_to_string(&canonical_path).expect("read canonical WIT");
        assert_eq!(
            WIT, canonical,
            "wit/xfetch-runtime.wit drifted from api/wit/xfetch-runtime.wit; run `cp` to sync"
        );
    }
}
