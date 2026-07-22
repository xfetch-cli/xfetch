use std::env;
use std::path::Path;
use std::process::Command;
use std::thread;

const ENV_SHELL: &str = "SHELL";
const ENV_PS_MODULE_PATH: &str = "PSModulePath";
const ENV_TERM_PROGRAM: &str = "TERM_PROGRAM";
const ENV_WT_SESSION: &str = "WT_SESSION";
const ENV_TERM: &str = "TERM";
const ENV_XDG_DESKTOP: &str = "XDG_CURRENT_DESKTOP";
const ENV_DESKTOP_SESSION: &str = "DESKTOP_SESSION";
const ENV_USER: &str = "USER";
const ENV_USERNAME: &str = "USERNAME";

const SHELL_POWERSHELL: &str = "PowerShell";
const SHELL_CMD: &str = "cmd";
const TERMINAL_WT: &str = "Windows Terminal";
const DESKTOP_EXPLORER: &str = "Explorer";
const DESKTOP_AQUA: &str = "Aqua";

const PACMAN_CMD: &str = "pacman";
const DPKG_CMD: &str = "dpkg";
const RPM_CMD: &str = "rpm";
const FLATPAK_CMD: &str = "flatpak";
const SNAP_CMD: &str = "snap";
const APK_CMD: &str = "apk";
const NIX_ENV_CMD: &str = "nix-env";
const BREW_CMD: &str = "brew";
const SCOOP_CMD: &str = "scoop";
const CHOCO_CMD: &str = "choco";

pub fn get_shell_info() -> String {
    if let Ok(shell) = env::var(ENV_SHELL) {
        let path = Path::new(&shell);
        if let Some(name) = path.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    if cfg!(target_os = "windows") {
        if env::var(ENV_PS_MODULE_PATH).is_ok() {
            return SHELL_POWERSHELL.to_string();
        }
        return SHELL_CMD.to_string();
    }
    super::unknown()
}

pub fn get_terminal_info() -> String {
    if let Ok(term) = env::var(ENV_TERM_PROGRAM) {
        return term;
    }
    if env::var(ENV_WT_SESSION).is_ok() {
        return TERMINAL_WT.to_string();
    }
    if let Ok(term) = env::var(ENV_TERM) {
        return term;
    }
    super::unknown()
}

fn run_package_check(cmd: &str, args: &[&str]) -> Option<usize> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().count())
}

fn format_package_count(count: usize, cmd: &str) -> String {
    format!("{} ({})", count, cmd)
}

fn run_package_checks(checks: &[(&str, &[&str])]) -> Vec<(String, String)> {
    thread::scope(|s| {
        checks
            .iter()
            .filter_map(|(cmd, args)| {
                let handle = s.spawn(|| {
                    run_package_check(cmd, args)
                        .map(|c| (cmd.to_string(), format_package_count(c, cmd)))
                });
                handle.join().ok()?
            })
            .collect()
    })
}

fn count_packages_linux() -> Vec<(String, String)> {
    let checks: &[(&str, &[&str])] = &[
        (PACMAN_CMD, &["-Qq"]),
        (DPKG_CMD, &["--get-selections"]),
        (RPM_CMD, &["-qa"]),
        (FLATPAK_CMD, &["list", "--app"]),
        (SNAP_CMD, &["list"]),
        (APK_CMD, &["info"]),
        (NIX_ENV_CMD, &["-q"]),
    ];
    run_package_checks(checks)
}

fn adjust_scoop_count(count: usize) -> usize {
    count.saturating_sub(4)
}

fn run_package_checks_with_adjustment(
    checks: &[(&str, &[&str])],
    adjust: fn(&str, usize) -> usize,
) -> Vec<(String, String)> {
    thread::scope(|s| {
        checks
            .iter()
            .filter_map(|(cmd, args)| {
                let handle = s.spawn(|| {
                    run_package_check(cmd, args)
                        .map(|c| (cmd.to_string(), format_package_count(adjust(cmd, c), cmd)))
                });
                handle.join().ok()?
            })
            .collect()
    })
}

fn count_packages_windows() -> Vec<(String, String)> {
    let checks: &[(&str, &[&str])] = &[
        (SCOOP_CMD, &["list"]),
        (CHOCO_CMD, &["list", "--local-only"]),
    ];
    fn scoop_adjust(cmd: &str, count: usize) -> usize {
        if cmd == SCOOP_CMD { adjust_scoop_count(count) } else { count }
    }
    run_package_checks_with_adjustment(checks, scoop_adjust)
}

fn count_packages_macos() -> Vec<(String, String)> {
    let checks: &[(&str, &[&str])] = &[(BREW_CMD, &["list", "--formula"])];
    run_package_checks(checks)
}

pub fn get_packages_info() -> String {
    let list = get_packages_breakdown();
    if list.is_empty() {
        super::unknown()
    } else {
        list.into_iter().map(|(_, v)| v).collect::<Vec<_>>().join(" + ")
    }
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    if cfg!(target_os = "linux") {
        count_packages_linux()
    } else if cfg!(target_os = "windows") {
        count_packages_windows()
    } else if cfg!(target_os = "macos") {
        count_packages_macos()
    } else {
        Vec::new()
    }
}

pub fn get_desktop_info() -> String {
    if let Ok(de) = env::var(ENV_XDG_DESKTOP) {
        return de;
    }
    if let Ok(de) = env::var(ENV_DESKTOP_SESSION) {
        return de;
    }
    if cfg!(target_os = "windows") {
        return DESKTOP_EXPLORER.to_string();
    }
    if cfg!(target_os = "macos") {
        return DESKTOP_AQUA.to_string();
    }
    super::unknown()
}

pub fn get_user_info() -> String {
    env::var(ENV_USER)
        .or_else(|_| env::var(ENV_USERNAME))
        .unwrap_or_else(|_| super::unknown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_info() {
        let user = get_user_info();
        assert!(!user.is_empty(), "user info should not be empty");
        let expected = env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "Unknown".to_string());
        assert_eq!(user, expected);
    }

    #[test]
    fn test_get_desktop_not_empty() {
        let de = get_desktop_info();
        assert!(!de.is_empty(), "desktop info should not be empty");
    }

    #[test]
    fn test_run_package_check_missing_cmd() {
        assert_eq!(run_package_check("nonexistent_cmd_xyz", &["--version"]), None);
    }

    #[test]
    fn test_format_package_count() {
        assert_eq!(format_package_count(42, "pacman"), "42 (pacman)");
        assert_eq!(format_package_count(0, "brew"), "0 (brew)");
    }

    #[test]
    fn test_all_platform_detectors_safe() {
        let linux = count_packages_linux();
        let windows = count_packages_windows();
        let macos = count_packages_macos();

        for (_, v) in &linux { assert!(v.contains('(')); }
        for (_, v) in &windows { assert!(v.contains('(')); }
        for (_, v) in &macos { assert!(v.contains('(')); }
    }

    #[test]
    fn test_multi_manager_format() {
        let checks: &[(&str, &[&str])] = &[
            (PACMAN_CMD, &["-Qq"]),
            (DPKG_CMD, &["--get-selections"]),
        ];
        let results = run_package_checks(checks);

        if results.len() > 1 {
            let joined: Vec<String> = results.iter().map(|(_, v)| v.clone()).collect();
            let combined = joined.join(" + ");
            assert!(combined.contains(" + "));
            assert!(combined.contains("pacman") || combined.contains("dpkg"));
        }
    }

    #[test]
    fn test_adjust_scoop_count() {
        assert_eq!(adjust_scoop_count(10), 6);
        assert_eq!(adjust_scoop_count(4), 0);
        assert_eq!(adjust_scoop_count(0), 0);
        assert_eq!(adjust_scoop_count(3), 0);
    }

    #[test]
    fn test_run_package_checks_with_adjustment_noop() {
        let checks: &[(&str, &[&str])] = &[(BREW_CMD, &["list", "--formula"])];
        fn noop(_: &str, c: usize) -> usize { c }
        let results = run_package_checks_with_adjustment(checks, noop);
        assert!(results.is_empty() || results[0].1.contains("brew"));
    }
}
