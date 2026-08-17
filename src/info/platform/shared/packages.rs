use super::commands::run_cmd_with_timeout;
use std::thread;
use std::time::Duration;

/// A package-manager probe: command name, arguments and its own timeout.
///
/// Timeouts are per command so each platform tunes its probes independently
/// (e.g. `snap` gets a short timeout because it hangs forever when the snapd
/// daemon is not running).
pub type PackageCheck<'a> = (&'a str, &'a [&'a str], Duration);

pub const PACKAGE_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
pub const SNAP_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

pub fn run_package_check_with_timeout(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Option<usize> {
    run_cmd_with_timeout(cmd, args, timeout)
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
}

pub fn format_package_count(count: usize, cmd: &str) -> String {
    format!("{} ({})", count, cmd)
}

pub fn run_package_checks(checks: &[PackageCheck]) -> Vec<(String, String)> {
    thread::scope(|s| {
        checks
            .iter()
            .filter_map(|(cmd, args, timeout)| {
                let handle = s.spawn(|| {
                    run_package_check_with_timeout(cmd, args, *timeout)
                        .map(|c| (cmd.to_string(), format_package_count(c, cmd)))
                });
                handle.join().ok()?
            })
            .collect()
    })
}

pub fn get_packages_info() -> String {
    let list = crate::info::platform::get_packages_breakdown();
    if list.is_empty() {
        crate::info::unknown()
    } else {
        list.into_iter()
            .map(|(_, v)| v)
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_package_check_missing_cmd() {
        assert_eq!(
            run_package_check_with_timeout(
                "nonexistent_cmd_xyz",
                &["--version"],
                PACKAGE_CHECK_TIMEOUT
            ),
            None
        );
    }

    #[test]
    fn test_format_package_count() {
        assert_eq!(format_package_count(42, "pacman"), "42 (pacman)");
        assert_eq!(format_package_count(0, "brew"), "0 (brew)");
    }

    #[test]
    fn test_multi_manager_format() {
        let checks: &[PackageCheck] = &[
            ("pacman", &["-Qq"], PACKAGE_CHECK_TIMEOUT),
            ("dpkg", &["--get-selections"], PACKAGE_CHECK_TIMEOUT),
        ];
        let results = run_package_checks(checks);

        if results.len() > 1 {
            let joined: Vec<String> = results.iter().map(|(_, v)| v.clone()).collect();
            let combined = joined.join(" + ");
            assert!(combined.contains(" + "));
            assert!(combined.contains("pacman") || combined.contains("dpkg"));
        }
    }
}
