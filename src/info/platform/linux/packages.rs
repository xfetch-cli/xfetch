use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, PackageCheck, SNAP_CHECK_TIMEOUT, run_package_checks,
};

const PACMAN_CMD: &str = "pacman";
const DPKG_CMD: &str = "dpkg";
const RPM_CMD: &str = "rpm";
const FLATPAK_CMD: &str = "flatpak";
const SNAP_CMD: &str = "snap";
const APK_CMD: &str = "apk";
const NIX_ENV_CMD: &str = "nix-env";

/// `snap` gets a short timeout: when snapd is not running, `snap list` blocks
/// forever on the snapd socket instead of failing.
const CHECKS: &[PackageCheck] = &[
    (PACMAN_CMD, &["-Qq"], PACKAGE_CHECK_TIMEOUT),
    (DPKG_CMD, &["--get-selections"], PACKAGE_CHECK_TIMEOUT),
    (RPM_CMD, &["-qa"], PACKAGE_CHECK_TIMEOUT),
    (FLATPAK_CMD, &["list", "--app"], PACKAGE_CHECK_TIMEOUT),
    (SNAP_CMD, &["list"], SNAP_CHECK_TIMEOUT),
    (APK_CMD, &["info"], PACKAGE_CHECK_TIMEOUT),
    (NIX_ENV_CMD, &["-q"], PACKAGE_CHECK_TIMEOUT),
];

/// When snapd is not running, `snap list` blocks forever on the snapd socket.
/// The socket only exists while the daemon is up, so probing it avoids spawning
/// `snap` (and its 3 s timeout) on systems without snapd — the count would be
/// zero anyway.
fn snapd_running() -> bool {
    std::path::Path::new("/run/snapd.socket").exists()
        || std::path::Path::new("/run/snapd-snap.socket").exists()
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    let checks: Vec<PackageCheck> = CHECKS
        .iter()
        .copied()
        .filter(|(cmd, _, _)| *cmd != SNAP_CMD || snapd_running())
        .collect();
    run_package_checks(&checks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linux_detectors_safe() {
        let linux = get_packages_breakdown();
        for (_, v) in &linux {
            assert!(v.contains('('));
        }
    }

    #[test]
    fn test_snapd_socket_precheck_skips_snap() {
        let checks: Vec<PackageCheck> = CHECKS
            .iter()
            .copied()
            .filter(|(cmd, _, _)| *cmd != SNAP_CMD || snapd_running())
            .collect();
        assert!(
            !checks.iter().any(|(cmd, _, _)| *cmd == SNAP_CMD)
                || std::path::Path::new("/run/snapd.socket").exists()
        );
    }
}
