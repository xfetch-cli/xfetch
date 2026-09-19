//! Live-refresh policy for Windows (`daemon_live`).
//!
//! The Windows daemon engine lives in `ui/win_daemon.rs`; this module only
//! declares the module set and cadence it defaults to. Battery is excluded
//! because it used to spawn `wmic`/PowerShell every tick — add it back with
//! `daemon_live_modules` if wanted.

/// Modules refreshed by the live daemon on Windows by default.
pub const LIVE_MODULES: &[&str] = &["cpu", "memory", "swap", "disk", "uptime", "datetime"];

/// Default refresh cadence (seconds) for the live daemon on Windows.
pub const DEFAULT_LIVE_REFRESH_SECS: u64 = 5;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_live_modules_exclude_battery() {
        assert!(
            !LIVE_MODULES.contains(&"battery"),
            "battery is heavy on Windows and must be opt-in"
        );
    }
}
