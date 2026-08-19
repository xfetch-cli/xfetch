use std::thread;
use std::time::Duration;

use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, count_scoop_output, count_winget_output, format_package_count,
    run_package_check_stdout,
};

const SCOOP_CMD: &str = "scoop";
const WINGET_CMD: &str = "winget";

/// `winget list` can be slow on first runs (source agreement/update).
const WINGET_TIMEOUT: Duration = Duration::from_secs(20);

/// Only the `winget` source counts: `winget list` also reports every app
/// registered in the registry (ARP/MSIX), which were not installed via
/// winget. Chocolatey is handled by a plugin, not here.
const CHECKS: &[(&str, &[&str], Duration)] = &[
    (SCOOP_CMD, &["list"], PACKAGE_CHECK_TIMEOUT),
    (
        WINGET_CMD,
        &[
            "list",
            "--source",
            "winget",
            "--disable-interactivity",
            "--accept-source-agreements",
        ],
        WINGET_TIMEOUT,
    ),
];

fn count_for(cmd: &str, stdout: &str) -> usize {
    match cmd {
        SCOOP_CMD => count_scoop_output(stdout),
        WINGET_CMD => count_winget_output(stdout),
        _ => stdout.lines().count(),
    }
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    thread::scope(|s| {
        let handles: Vec<_> = CHECKS
            .iter()
            .map(|(cmd, args, timeout)| {
                s.spawn(move || {
                    run_package_check_stdout(cmd, args, *timeout).map(|out| {
                        (
                            cmd.to_string(),
                            format_package_count(count_for(cmd, &out), cmd),
                        )
                    })
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()?).collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_detectors_safe() {
        let windows = get_packages_breakdown();
        for (_, v) in &windows {
            assert!(v.contains('('));
        }
    }
}
