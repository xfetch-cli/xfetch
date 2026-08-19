use super::commands::run_cmd_with_timeout;
#[cfg(target_os = "linux")]
use std::thread;
use std::time::Duration;

/// A package-manager probe: command name, arguments and its own timeout.
///
/// Timeouts are per command so each platform tunes its probes independently
/// (e.g. `snap` gets a short timeout because it hangs forever when the snapd
/// daemon is not running). Used by the Linux runner; macOS and Windows keep
/// their own.
#[cfg(target_os = "linux")]
pub type PackageCheck<'a> = (&'a str, &'a [&'a str], Duration);

pub const PACKAGE_CHECK_TIMEOUT: Duration = Duration::from_secs(10);
#[cfg(target_os = "linux")]
pub const SNAP_CHECK_TIMEOUT: Duration = Duration::from_secs(3);

/// Runs a package probe and returns its stdout when the command succeeds.
/// Per-OS counters parse this raw output (e.g. choco's summary line,
/// winget's table header, brew's notices) instead of naively counting lines.
pub fn run_package_check_stdout(cmd: &str, args: &[&str], timeout: Duration) -> Option<String> {
    run_cmd_with_timeout(cmd, args, timeout)
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
}

#[cfg(target_os = "linux")]
pub fn run_package_check_with_timeout(
    cmd: &str,
    args: &[&str],
    timeout: Duration,
) -> Option<usize> {
    run_package_check_stdout(cmd, args, timeout).map(|out| out.lines().count())
}

/// Parsers for package-manager output. They are pure text logic so they live
/// here (testable on any platform via `cfg(test)`); each OS folder decides
/// which commands to run and maps them to the matching parser.
///
/// `scoop list` prints a header block ("Installed apps:", "Name Version ...",
/// a dashes row) before the actual apps — counts "name version ..." rows.
#[cfg(any(target_os = "windows", test))]
pub fn count_scoop_output(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|line| {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            tokens.len() >= 2
                && tokens[0] != "Name"
                && tokens[0] != "Installed"
                && !tokens[0].chars().all(|c| c == '-')
        })
        .count()
}

/// `choco list --local-only` / `choco list` prints one "name version" row per
/// package and ends with a "X packages installed." summary line. The banner
/// ("Chocolatey vX.Y.Z") is excluded by requiring the version to start with a
/// digit (choco versions always do).
#[cfg(any(target_os = "windows", test))]
pub fn count_choco_output(stdout: &str) -> usize {
    stdout
        .lines()
        .filter(|line| {
            let tokens: Vec<&str> = line.split_whitespace().collect();
            tokens.len() == 2
                && !line.contains("packages installed")
                && tokens[1].starts_with(|c: char| c.is_ascii_digit())
        })
        .count()
}

/// `winget list` prints a table: header row (localized: "Name"/"Nombre"/...),
/// a dashes separator, then the actual entries. Only rows after the separator
/// count; when no separator is present (older winget), the first line is
/// treated as the header.
#[cfg(any(target_os = "windows", test))]
pub fn count_winget_output(stdout: &str) -> usize {
    let lines: Vec<&str> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if let Some(idx) = lines.iter().position(|l| l.chars().all(|c| c == '-')) {
        lines[idx + 1..]
            .iter()
            .filter(|l| l.split_whitespace().count() >= 2)
            .count()
    } else {
        lines
            .iter()
            .skip(1)
            .filter(|l| l.split_whitespace().count() >= 2)
            .count()
    }
}

/// `brew list --formula` output — counts package names, ignoring empty lines,
/// `==> ...` notices and any header/whitespace noise Homebrew may emit.
#[cfg(any(target_os = "macos", test))]
pub fn count_brew_output(stdout: &str) -> usize {
    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with("==>") && !l.contains(char::is_whitespace))
        .count()
}

pub fn format_package_count(count: usize, cmd: &str) -> String {
    format!("{} ({})", count, cmd)
}

#[cfg(target_os = "linux")]
pub fn run_package_checks(checks: &[PackageCheck]) -> Vec<(String, String)> {
    thread::scope(|s| {
        let handles: Vec<_> = checks
            .iter()
            .map(|(cmd, args, timeout)| {
                s.spawn(move || {
                    run_package_check_with_timeout(cmd, args, *timeout)
                        .map(|c| (cmd.to_string(), format_package_count(c, cmd)))
                })
            })
            .collect();
        handles.into_iter().filter_map(|h| h.join().ok()?).collect()
    })
}

pub fn packages_info_from_breakdown(breakdown: &[(String, String)]) -> String {
    if breakdown.is_empty() {
        crate::info::unknown()
    } else {
        breakdown
            .iter()
            .map(|(_, v)| v.as_str())
            .collect::<Vec<_>>()
            .join(" + ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "linux")]
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
    fn test_count_scoop_output() {
        let stdout = "\nInstalled apps:\n\nName Version Source Updated Info\n---- ------- ------ ------- ----\nfoo 1.0     main\ngit 2.45.0  main\n";
        assert_eq!(count_scoop_output(stdout), 2);
        assert_eq!(count_scoop_output(""), 0);
    }

    #[test]
    fn test_count_choco_output_skips_summary_line() {
        let stdout = "git 2.45.0\nnodejs 20.11.0\n2 packages installed.\n";
        assert_eq!(count_choco_output(stdout), 2);
        assert_eq!(count_choco_output("0 packages installed.\n"), 0);
    }

    #[test]
    fn test_count_choco_output_skips_banner() {
        let stdout = "Chocolatey v2.6.0\nchocolatey 2.6.0\npython 3.14.3\n20 packages installed.\n";
        assert_eq!(count_choco_output(stdout), 2);
    }

    #[test]
    fn test_count_winget_output() {
        let stdout = "Name             Id                       Version\n-----------------------------------------------------\ngit              Git.Git                  2.45.0\nPowerShell       Microsoft.PowerShell    7.5.0\n";
        assert_eq!(count_winget_output(stdout), 2);
        assert_eq!(count_winget_output("Name Id Version\n"), 0);
    }

    #[test]
    fn test_count_winget_output_localized_header() {
        let stdout = "Nombre        Id                  Version\n-----------------------------------------------\ngit           Git.Git             2.45.0\n";
        assert_eq!(count_winget_output(stdout), 1);
    }

    #[test]
    fn test_count_brew_output_ignores_noise() {
        let stdout = "\n==> Updating Homebrew...\n\nvim\n\npython@3.12\ngh\n";
        assert_eq!(count_brew_output(stdout), 3);
        assert_eq!(count_brew_output("==> notice only\n\n"), 0);
    }

    #[cfg(target_os = "linux")]
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
