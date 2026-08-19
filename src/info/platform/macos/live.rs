//! Live-refresh policy for macOS (used by `ui::live`).
//!
//! Battery (`pmset -g batt`) is a light subprocess on macOS, so it stays in
//! the defaults with a slightly slower tick than Linux.

use crate::info::platform::LivePolicy;

/// Default refresh cadence (seconds) for the live daemon on macOS.
pub const DEFAULT_LIVE_REFRESH_SECS: u64 = 3;

pub fn live_policy() -> LivePolicy {
    LivePolicy {
        modules: &[
            "cpu", "memory", "swap", "disks", "battery", "uptime", "datetime",
        ],
        default_refresh_secs: DEFAULT_LIVE_REFRESH_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_live_modules_non_empty() {
        let policy = live_policy();
        assert!(!policy.modules.is_empty());
        assert!(policy.default_refresh_secs > 0);
    }
}
