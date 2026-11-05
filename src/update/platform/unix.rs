//! Unix self-update policy.

use crate::update::github::Release;
use crate::update::{UpdateOptions, UpdateOutcome};
use semver::Version;
use std::path::Path;

/// Release target for the running Unix platform.
pub(crate) fn release_target() -> Option<&'static str> {
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "musl"
    )) {
        Some("x86_64-unknown-linux-musl")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

/// Executable name inside the archive and on disk.
pub(crate) const fn binary_name() -> &'static str {
    "xfetch"
}

/// Unix reports a failed `cargo install` as a hard error.
pub(crate) fn cargo_failure(_command: &str) -> Result<UpdateOutcome, String> {
    Err("cargo install failed".to_string())
}

/// Unix reports a `cargo` spawn failure as a hard error.
pub(crate) fn cargo_spawn_failure(_command: &str, err: &str) -> Result<UpdateOutcome, String> {
    Err(format!("Failed to run cargo: {}", err))
}

/// winget is Windows-only; this arm is unreachable on Unix.
pub(crate) fn run_winget_update(
    release: &Release,
    current: Version,
    executable: &Path,
    options: &UpdateOptions,
) -> Result<UpdateOutcome, String> {
    let _ = (release, current, executable, options);
    Ok(UpdateOutcome::Manual {
        message: "winget is only available on Windows.".to_string(),
    })
}
