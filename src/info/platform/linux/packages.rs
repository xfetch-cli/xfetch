use std::path::Path;

use crate::info::platform::shared::packages::{
    PACKAGE_CHECK_TIMEOUT, PackageCheck, SNAP_CHECK_TIMEOUT, format_package_count,
    run_package_checks,
};

const PACMAN_CMD: &str = "pacman";
const AUR_LABEL: &str = "aur";
const DPKG_CMD: &str = "dpkg";
const RPM_CMD: &str = "rpm";
const FLATPAK_CMD: &str = "flatpak";
const SNAP_CMD: &str = "snap";
const APK_CMD: &str = "apk";
const NIX_ENV_CMD: &str = "nix-env";
const XBPS_CMD: &str = "xbps-query";
const PORTAGE_LABEL: &str = "portage";

/// Databases that mirror what `dpkg --get-selections`, `apk info` and
/// `flatpak list --app` report, world-readable on every distro. Reading them
/// counts packages in microseconds instead of spawning a process (which on
/// WSL also pays for the execvp PATH search across the slow `/mnt/c` mounts).
///
/// Arch has no database entry: the pacman DB holds *all* installed packages,
/// official and AUR alike, so the split comes from `pacman -Qn` (official) /
/// `pacman -Qm` (foreign) below instead.
const DPKG_DB: &str = "/var/lib/dpkg/status";
const APK_DB: &str = "/var/lib/apk/db/installed";
const FLATPAK_SYSTEM_APP_DIR: &str = "/var/lib/flatpak/app";
const FLATPAK_USER_APP_DIR: &str = ".local/share/flatpak/app";
const VOID_DB_DIR: &str = "/var/db/xbps";
const PORTAGE_DB_DIR: &str = "/var/db/pkg";

/// `snap` gets a short timeout: when snapd is not running, `snap list` blocks
/// forever on the snapd socket instead of failing.
///
/// On Arch, `pacman -Qn` lists packages found in the sync databases (official)
/// and `pacman -Qm` lists foreign packages (AUR and manual installs); running
/// `pacman -Qq` plus a helper (`yay -Qq` / `paru -Qq`) would double- or
/// triple-count the same set, since every AUR install goes through pacman.
/// `portage` (Gentoo) is database-only, so it needs no probe arguments.
const CHECKS: &[PackageCheck] = &[
    PackageCheck {
        binary: PACMAN_CMD,
        args: &["-Qn"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: PACMAN_CMD,
    },
    PackageCheck {
        binary: PACMAN_CMD,
        args: &["-Qm"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: AUR_LABEL,
    },
    PackageCheck {
        binary: DPKG_CMD,
        args: &["--get-selections"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: DPKG_CMD,
    },
    PackageCheck {
        binary: RPM_CMD,
        args: &["-qa"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: RPM_CMD,
    },
    PackageCheck {
        binary: FLATPAK_CMD,
        args: &["list", "--app"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: FLATPAK_CMD,
    },
    PackageCheck {
        binary: SNAP_CMD,
        args: &["list"],
        timeout: SNAP_CHECK_TIMEOUT,
        label: SNAP_CMD,
    },
    PackageCheck {
        binary: APK_CMD,
        args: &["info"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: APK_CMD,
    },
    PackageCheck {
        binary: NIX_ENV_CMD,
        args: &["-q"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: NIX_ENV_CMD,
    },
    PackageCheck {
        binary: XBPS_CMD,
        args: &["-l"],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: XBPS_CMD,
    },
    PackageCheck {
        binary: PORTAGE_LABEL,
        args: &[],
        timeout: PACKAGE_CHECK_TIMEOUT,
        label: PORTAGE_LABEL,
    },
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
        .filter(|check| {
            (check.binary != SNAP_CMD || snapd_running())
                && !db_counts.iter().any(|(c, _)| *c == check.label)
        })
        .cloned()
        .collect();
    let cmd_counts = run_package_checks(&pending);
    CHECKS
        .iter()
        .filter_map(|check| {
            if let Some((_, n)) = db_counts.iter().find(|(c, _)| *c == check.label) {
                Some((
                    check.label.to_string(),
                    format_package_count(*n, check.label),
                ))
            } else if check.binary == SNAP_CMD && !snapd_running() {
                None
            } else {
                cmd_counts.iter().find(|(c, _)| c == check.label).cloned()
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
            .filter(|check| check.binary != SNAP_CMD || snapd_running())
            .cloned()
            .collect();
        assert!(
            !checks.iter().any(|check| check.binary == SNAP_CMD)
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
        let db_counts: Vec<(&str, usize)> = vec![(DPKG_CMD, 716), (APK_CMD, 42)];
        let output: Vec<String> = CHECKS
            .iter()
            .filter_map(|check| {
                db_counts
                    .iter()
                    .find(|(c, _)| *c == check.label)
                    .map(|(_, n)| format_package_count(*n, check.label))
            })
            .collect();
        assert!(output[0].contains("dpkg"), "dpkg must come first");
        assert!(output[1].contains("apk"), "apk must come second");
    }
}
