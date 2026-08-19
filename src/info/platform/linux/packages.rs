use std::path::Path;

use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, PackageCheck, SNAP_CHECK_TIMEOUT, format_package_count,
    run_package_checks,
};

const PACMAN_CMD: &str = "pacman";
const YAY_CMD: &str = "yay";
const PARU_CMD: &str = "paru";
const DPKG_CMD: &str = "dpkg";
const RPM_CMD: &str = "rpm";
const FLATPAK_CMD: &str = "flatpak";
const SNAP_CMD: &str = "snap";
const APK_CMD: &str = "apk";
const NIX_ENV_CMD: &str = "nix-env";
const XBPS_CMD: &str = "xbps-query";
const PORTAGE_LABEL: &str = "portage";

/// Databases that mirror what `dpkg --get-selections`, `pacman -Qq`,
/// `apk info` and `flatpak list --app` report, world-readable on every distro.
/// Reading them counts packages in microseconds instead of spawning a process
/// (which on WSL also pays for the execvp PATH search across the slow
/// `/mnt/c` mounts).
const DPKG_DB: &str = "/var/lib/dpkg/status";
const PACMAN_DB_DIR: &str = "/var/lib/pacman/local";
const APK_DB: &str = "/var/lib/apk/db/installed";
const FLATPAK_SYSTEM_APP_DIR: &str = "/var/lib/flatpak/app";
const FLATPAK_USER_APP_DIR: &str = ".local/share/flatpak/app";
const VOID_DB_DIR: &str = "/var/db/xbps";
const PORTAGE_DB_DIR: &str = "/var/db/pkg";

/// `snap` gets a short timeout: when snapd is not running, `snap list` blocks
/// forever on the snapd socket instead of failing.
const CHECKS: &[PackageCheck] = &[
    (PACMAN_CMD, &["-Qq"], PACKAGE_CHECK_TIMEOUT),
    (YAY_CMD, &["-Qq"], PACKAGE_CHECK_TIMEOUT),
    (PARU_CMD, &["-Qq"], PACKAGE_CHECK_TIMEOUT),
    (DPKG_CMD, &["--get-selections"], PACKAGE_CHECK_TIMEOUT),
    (RPM_CMD, &["-qa"], PACKAGE_CHECK_TIMEOUT),
    (FLATPAK_CMD, &["list", "--app"], PACKAGE_CHECK_TIMEOUT),
    (SNAP_CMD, &["list"], SNAP_CHECK_TIMEOUT),
    (APK_CMD, &["info"], PACKAGE_CHECK_TIMEOUT),
    (NIX_ENV_CMD, &["-q"], PACKAGE_CHECK_TIMEOUT),
    (XBPS_CMD, &["-l"], PACKAGE_CHECK_TIMEOUT),
];

/// When snapd is not running, `snap list` blocks forever on the snapd socket.
/// The socket only exists while the daemon is up, so probing it avoids spawning
/// `snap` (and its 3 s timeout) on systems without snapd — the count would be
/// zero anyway.
fn snapd_running() -> bool {
    Path::new("/run/snapd.socket").exists() || Path::new("/run/snapd-snap.socket").exists()
}

fn count_lines_with_prefix(path: &str, prefix: &str) -> Option<usize> {
    let content = std::fs::read_to_string(path).ok()?;
    Some(content.lines().filter(|l| l.starts_with(prefix)).count())
}

fn count_dirs(path: &str) -> Option<usize> {
    Some(
        std::fs::read_dir(path)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .count(),
    )
}

/// Partial installs in Void's and Portage's databases are kept under
/// dot-prefixed entries (e.g. `.libfoo-1.0_1`) and must not be counted.
fn is_dot_entry(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|s| s.starts_with('.'))
}

/// Void: one directory per installed package under `/var/db/xbps/`
/// (matches `xbps-query -l`).
fn count_void_packages() -> Option<usize> {
    Some(
        std::fs::read_dir(VOID_DB_DIR)
            .ok()?
            .filter_map(|e| e.ok())
            .filter(|e| {
                !is_dot_entry(&e.file_name()) && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
            })
            .count(),
    )
}

/// Gentoo: `/var/db/pkg/<category>/<package>/` — count packages across all
/// category dirs (matches `qlist -I`).
fn count_portage_packages() -> Option<usize> {
    let mut total = 0;
    for category in std::fs::read_dir(PORTAGE_DB_DIR).ok()?.flatten() {
        if is_dot_entry(&category.file_name())
            || !category.file_type().map(|t| t.is_dir()).unwrap_or(false)
        {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(category.path()) {
            total += entries
                .flatten()
                .filter(|e| {
                    !is_dot_entry(&e.file_name())
                        && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                })
                .count();
        }
    }
    Some(total)
}

fn count_flatpak_apps() -> Option<usize> {
    let system = count_dirs(FLATPAK_SYSTEM_APP_DIR);
    let user = std::env::var_os("HOME").and_then(|home| {
        Path::new(&home)
            .join(FLATPAK_USER_APP_DIR)
            .to_str()
            .and_then(count_dirs)
    });
    match (system, user) {
        (Some(s), Some(u)) => Some(s + u),
        (Some(s), None) => Some(s),
        (None, Some(u)) => Some(u),
        (None, None) => None,
    }
}

/// Package counts read from the distro's own databases instead of subprocesses.
fn db_package_counts() -> Vec<(&'static str, usize)> {
    let mut counts = Vec::new();
    if let Some(n) = count_lines_with_prefix(DPKG_DB, "Package:") {
        counts.push((DPKG_CMD, n));
    }
    if let Some(n) = count_dirs(PACMAN_DB_DIR) {
        counts.push((PACMAN_CMD, n));
    }
    if let Some(n) = count_lines_with_prefix(APK_DB, "P:") {
        counts.push((APK_CMD, n));
    }
    if let Some(n) = count_flatpak_apps() {
        counts.push((FLATPAK_CMD, n));
    }
    if let Some(n) = count_void_packages() {
        counts.push((XBPS_CMD, n));
    }
    if let Some(n) = count_portage_packages() {
        counts.push((PORTAGE_LABEL, n));
    }
    counts
}

pub fn get_packages_breakdown() -> Vec<(String, String)> {
    let db_counts = db_package_counts();
    let pending: Vec<PackageCheck> = CHECKS
        .iter()
        .copied()
        .filter(|(cmd, _, _)| {
            (*cmd != SNAP_CMD || snapd_running()) && !db_counts.iter().any(|(c, _)| c == cmd)
        })
        .collect();
    let cmd_counts = run_package_checks(&pending);
    CHECKS
        .iter()
        .filter_map(|(cmd, _, _)| {
            if let Some((_, n)) = db_counts.iter().find(|(c, _)| c == cmd) {
                Some((cmd.to_string(), format_package_count(*n, cmd)))
            } else if *cmd == SNAP_CMD && !snapd_running() {
                None
            } else {
                cmd_counts.iter().find(|(c, _)| c == cmd).cloned()
            }
        })
        .collect()
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

    #[test]
    fn test_count_lines_with_prefix() {
        let dir = std::env::temp_dir().join(format!("xfetch_test_lines_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("status");
        std::fs::write(
            &file,
            "Package: a\nStatus: install ok installed\n\nPackage: b\n",
        )
        .unwrap();
        assert_eq!(
            count_lines_with_prefix(file.to_str().unwrap(), "Package:"),
            Some(2)
        );
        assert_eq!(
            count_lines_with_prefix(file.to_str().unwrap(), "Status:"),
            Some(1)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_count_dirs() {
        let dir = std::env::temp_dir().join(format!("xfetch_test_dirs_{}", std::process::id()));
        std::fs::create_dir_all(dir.join("pkg1")).unwrap();
        std::fs::create_dir_all(dir.join("pkg2")).unwrap();
        std::fs::write(dir.join("db.lck"), "").unwrap();
        assert_eq!(count_dirs(dir.to_str().unwrap()), Some(2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_db_counts_preserve_check_order() {
        let db_counts: Vec<(&str, usize)> = vec![(DPKG_CMD, 716), (PACMAN_CMD, 42)];
        let output: Vec<String> = CHECKS
            .iter()
            .filter_map(|(cmd, _, _)| {
                db_counts
                    .iter()
                    .find(|(c, _)| c == cmd)
                    .map(|(_, n)| format_package_count(*n, cmd))
            })
            .collect();
        assert!(output[0].contains("pacman"), "pacman must come first");
        assert!(output[1].contains("dpkg"), "dpkg must come second");
    }
}
