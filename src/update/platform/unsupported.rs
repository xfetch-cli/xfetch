//! Fallback policy for operating systems without a native update path.

use crate::update::UpdateOutcome;

pub(crate) fn release_target() -> Option<&'static str> {
    None
}

pub(crate) const fn binary_name() -> &'static str {
    "xfetch"
}

pub(crate) fn cargo_failure(command: &str) -> Result<UpdateOutcome, String> {
    Err(format!("cargo install failed. Run:\n  {}", command))
}

pub(crate) fn cargo_spawn_failure(_command: &str, err: &str) -> Result<UpdateOutcome, String> {
    Err(format!("Failed to run cargo: {}", err))
}
