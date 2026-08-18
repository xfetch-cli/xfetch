use std::thread;

use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, PackageCheck, format_package_count, run_package_check_with_timeout,
};

const SCOOP_CMD: &str = "scoop";
const CHOCO_CMD: &str = "choco";

const CHECKS: &[PackageCheck] = &[
    (SCOOP_CMD, &["list"], PACKAGE_CHECK_TIMEOUT),
    (CHOCO_CMD, &["list", "--local-only"], PACKAGE_CHECK_TIMEOUT),
];

fn run_package_checks_with_adjustment(
    checks: &[PackageCheck],
    adjust: fn(&str, usize) -> usize,
) -> Vec<(String, String)> {
    thread::scope(|s| {
        let handles: Vec<_> = checks
            .iter()
            .map(|(cmd, args, timeout)| {
                s.spawn(move || {
                    run_package_check_with_timeout(cmd, args, *timeout)
                        .map(|c| (cmd.to_string(), format_package_count(adjust(cmd, c), cmd)))
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()?).collect()
    })
}

fn scoop_adjust(cmd: &str, count: usize) -> usize {
    if cmd == SCOOP_CMD {
        count.saturating_sub(4)
    } else {
        count
    }
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    run_package_checks_with_adjustment(CHECKS, scoop_adjust)
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

    #[test]
    fn test_adjust_scoop_count() {
        assert_eq!(scoop_adjust(SCOOP_CMD, 10), 6);
        assert_eq!(scoop_adjust(SCOOP_CMD, 4), 0);
        assert_eq!(scoop_adjust(SCOOP_CMD, 0), 0);
        assert_eq!(scoop_adjust(SCOOP_CMD, 3), 0);
        assert_eq!(scoop_adjust(CHOCO_CMD, 10), 10);
    }

    #[test]
    fn test_run_package_checks_with_adjustment_noop() {
        let checks: &[PackageCheck] =
            &[(CHOCO_CMD, &["list", "--local-only"], PACKAGE_CHECK_TIMEOUT)];
        fn noop(_: &str, c: usize) -> usize {
            c
        }
        let results = run_package_checks_with_adjustment(checks, noop);
        assert!(results.is_empty() || results[0].1.contains("choco"));
    }
}
