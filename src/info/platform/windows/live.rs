//! Live-refresh policy for Windows (used by `ui::live`).
//!
//! Battery on Windows runs `wmic`/PowerShell subprocesses, which are heavy to
//! spawn every tick, so it is excluded from the defaults (add it back with
//! `daemon_live_modules` if you want it). The default tick is slower than the
//! other platforms for the same reason.

use crate::info::platform::LivePolicy;

/// Default refresh cadence (seconds) for the live daemon on Windows.
pub const DEFAULT_LIVE_REFRESH_SECS: u64 = 5;

pub fn live_policy() -> LivePolicy {
    LivePolicy {
        modules: &["cpu", "memory", "swap", "disks", "uptime", "datetime"],
        default_refresh_secs: DEFAULT_LIVE_REFRESH_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_live_modules_exclude_battery() {
        let policy = live_policy();
        assert!(
            !policy.modules.contains(&"battery"),
            "battery is heavy on Windows and must be opt-in"
        );
        assert!(policy.default_refresh_secs >= 5);
    }
}
