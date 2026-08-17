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

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    run_package_checks(CHECKS)
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
}
