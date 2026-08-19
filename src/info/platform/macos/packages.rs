use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, count_brew_output, format_package_count, run_package_check_stdout,
};

const BREW_CMD: &str = "brew";

const CHECKS: &[(&str, &[&str])] = &[(BREW_CMD, &["list", "--formula"])];

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    CHECKS
        .iter()
        .filter_map(|(cmd, args)| {
            run_package_check_stdout(cmd, args, PACKAGE_CHECK_TIMEOUT).map(|out| {
                (
                    cmd.to_string(),
                    format_package_count(count_brew_output(&out), cmd),
                )
            })
        })
        .collect()
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
