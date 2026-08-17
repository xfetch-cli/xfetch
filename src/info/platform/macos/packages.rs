use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, PackageCheck, run_package_checks,
};

const BREW_CMD: &str = "brew";

const CHECKS: &[PackageCheck] = &[(BREW_CMD, &["list", "--formula"], PACKAGE_CHECK_TIMEOUT)];

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    run_package_checks(CHECKS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_detectors_safe() {
        let macos = get_packages_breakdown();
        for (_, v) in &macos {
            assert!(v.contains('('));
        }
    }
}
