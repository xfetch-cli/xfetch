use std::thread;
use std::time::Duration;

use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, count_scoop_output, count_winget_output, format_package_count,
    run_package_check_stdout,
};

const SCOOP_CMD: &str = "scoop";
/// Scoop is a PowerShell script whose PATH shim is `scoop.cmd`; Windows
/// `CreateProcess` only appends `.exe`, so a bare `scoop` never resolves.
const SCOOP_SHIM: &str = "scoop.cmd";
const WINGET_CMD: &str = "winget";

/// `winget list` can be slow on first runs (source agreement/update).
const WINGET_TIMEOUT: Duration = Duration::from_secs(20);

/// Only the `winget` source counts: `winget list` also reports every app
/// registered in the registry (ARP/MSIX), which were not installed via
/// winget. Chocolatey is handled by a plugin, not here.
const WINGET_ARGS: &[&str] = &[
    "list",
    "--source",
    "winget",
    "--disable-interactivity",
    "--accept-source-agreements",
];

fn count_for(cmd: &str, stdout: &str) -> usize {
    match cmd {
        SCOOP_CMD => count_scoop_output(stdout),
        WINGET_CMD => count_winget_output(stdout),
        _ => stdout.lines().count(),
    }
}

/// `scoop list` through the shim, falling back to the bare name for
/// installs that expose a real `scoop.exe`.
fn run_scoop_list() -> Option<String> {
    for cmd in [SCOOP_SHIM, SCOOP_CMD] {
        if let Some(stdout) = run_package_check_stdout(cmd, &["list"], PACKAGE_CHECK_TIMEOUT) {
            return Some(stdout);
        }
    }
    None
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    thread::scope(|s| {
        let scoop_h = s.spawn(|| {
            run_scoop_list().map(|out| {
                (
                    SCOOP_CMD.to_string(),
                    format_package_count(count_for(SCOOP_CMD, &out), SCOOP_CMD),
                )
            })
        });
        let winget_h = s.spawn(|| {
            run_package_check_stdout(WINGET_CMD, WINGET_ARGS, WINGET_TIMEOUT).map(|out| {
                (
                    WINGET_CMD.to_string(),
                    format_package_count(count_for(WINGET_CMD, &out), WINGET_CMD),
                )
            })
        });
        [scoop_h, winget_h]
            .into_iter()
            .filter_map(|h| h.join().ok()?)
            .collect()
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
