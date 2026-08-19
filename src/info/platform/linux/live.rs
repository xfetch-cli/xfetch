//! Live-refresh policy for Linux (used by `ui::live`).
//!
//! All default modules are cheap here: battery reads `/sys/class/power_supply`
//! and datetime runs `date`, both fast enough for a 2-second tick.

use crate::info::platform::LivePolicy;

/// Default refresh cadence (seconds) for the live daemon on Linux.
pub const DEFAULT_LIVE_REFRESH_SECS: u64 = 2;

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
    fn test_linux_live_modules_non_empty() {
        let policy = live_policy();
        assert!(!policy.modules.is_empty());
        assert!(policy.default_refresh_secs > 0);
    }
}
