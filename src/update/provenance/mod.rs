//! Detects how the running binary was installed.
//!
//! The updater never guesses when it can avoid it: install receipts written by
//! the PowerShell installers win over path heuristics, and `cargo install
//! --list` distinguishes a crates.io install from a source checkout. An
//! unrecognized path is never replaced automatically.

#[cfg(unix)]
mod unix;
#[cfg(any(windows, test))]
mod windows;

use crate::update::receipt::InstallReceipt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Where the current executable appears to come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallSource {
    /// Installed with `cargo install xfetch-cli` (`~/.cargo/bin`).
    Cargo,
    /// Built from a source checkout, usually by `install.ps1`.
    SourceBuild,
    /// Installed with winget on Windows.
    #[cfg_attr(not(windows), allow(dead_code))]
    Winget,
    /// Owned by a Unix package manager (`/usr/bin`, Homebrew, ...).
    #[cfg_attr(not(unix), allow(dead_code))]
    PackageManager,
    /// Installed by the prebuilt installer (`~/.local/bin`, `%LOCALAPPDATA%`).
    Prebuilt,
    /// A local build, a manual copy, or an unrecognized location.
    Unknown,
}

impl InstallSource {
    /// Short label used in reports.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Cargo => "cargo install",
            Self::SourceBuild => "source build",
            Self::Winget => "winget",
            Self::PackageManager => "package manager",
            Self::Prebuilt => "prebuilt install",
            Self::Unknown => "local build or unknown",
        }
    }

    /// Sources whose files belong to a package manager or build tool.
    pub const fn is_package_managed(self) -> bool {
        matches!(self, Self::Cargo | Self::SourceBuild | Self::Winget)
    }
}

/// Result of provenance detection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedInstall {
    /// Detected installation channel.
    pub source: InstallSource,
    /// Source checkout for [`InstallSource::SourceBuild`], when known.
    pub source_path: Option<PathBuf>,
}

impl DetectedInstall {
    fn new(source: InstallSource) -> Self {
        Self {
            source,
            source_path: None,
        }
    }
}

/// Paths used by the platform classifiers.
pub(crate) struct InstallPaths {
    pub home: Option<PathBuf>,
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub local_app_data: Option<PathBuf>,
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub program_files: Option<PathBuf>,
    #[cfg_attr(not(any(windows, test)), allow(dead_code))]
    pub program_files_x86: Option<PathBuf>,
}

impl InstallPaths {
    fn from_env() -> Self {
        Self {
            home: dirs::home_dir(),
            local_app_data: dirs::data_local_dir(),
            program_files: std::env::var_os("ProgramFiles").map(PathBuf::from),
            program_files_x86: std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
        }
    }
}

/// Classifies `executable` using the real home directory and environment.
pub fn detect(executable: &Path) -> DetectedInstall {
    detect_with(
        executable,
        &InstallPaths::from_env(),
        InstallReceipt::load(),
    )
}

fn detect_with(
    executable: &Path,
    paths: &InstallPaths,
    receipt: Option<InstallReceipt>,
) -> DetectedInstall {
    if let Some(receipt) = receipt
        && receipt.matches_executable(executable)
    {
        let source = match receipt.method {
            crate::update::receipt::Method::Prebuilt => InstallSource::Prebuilt,
            crate::update::receipt::Method::Source => InstallSource::SourceBuild,
            crate::update::receipt::Method::Cargo => InstallSource::Cargo,
            crate::update::receipt::Method::Winget => InstallSource::Winget,
            crate::update::receipt::Method::Manual => InstallSource::Unknown,
        };
        return DetectedInstall {
            source,
            source_path: receipt.source_path,
        };
    }

    let source = classify_path(executable, paths);
    if source == InstallSource::Cargo {
        return refine_cargo_install();
    }
    DetectedInstall::new(source)
}

fn classify_path(executable: &Path, paths: &InstallPaths) -> InstallSource {
    #[cfg(unix)]
    {
        unix::classify(executable, paths)
    }
    #[cfg(windows)]
    {
        windows::classify(executable, paths)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (executable, paths);
        InstallSource::Unknown
    }
}

/// The two shapes reported by `cargo install --list` for `xfetch-cli`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CargoInstallKind {
    /// Installed from crates.io.
    Registry,
    /// Installed from a checkout (`cargo install --path`, `install.ps1`).
    Source(PathBuf),
}

fn refine_cargo_install() -> DetectedInstall {
    match cargo_install_kind() {
        Some(CargoInstallKind::Source(path)) => DetectedInstall {
            source: InstallSource::SourceBuild,
            source_path: Some(path),
        },
        Some(CargoInstallKind::Registry) | None => DetectedInstall::new(InstallSource::Cargo),
    }
}

fn cargo_install_kind() -> Option<CargoInstallKind> {
    let output = crate::subprocess::run_cmd_with_stdin_timeout(
        Path::new("cargo"),
        &["install", "--list"],
        None,
        Some(Duration::from_secs(15)),
    )?;
    if !output.status.success() {
        return None;
    }
    parse_cargo_install_list(&String::from_utf8_lossy(&output.stdout))
}

/// Parses the `cargo install --list` output for the `xfetch-cli` entry.
pub(crate) fn parse_cargo_install_list(output: &str) -> Option<CargoInstallKind> {
    for line in output.lines() {
        let line = line.trim();
        if !line.starts_with("xfetch-cli v") {
            continue;
        }
        if let (Some(start), Some(end)) = (line.find('('), line.rfind(')'))
            && end > start
        {
            return Some(CargoInstallKind::Source(PathBuf::from(
                &line[start + 1..end],
            )));
        }
        return Some(CargoInstallKind::Registry);
    }
    None
}

#[cfg(test)]
fn detect_with_home(executable: &Path, home: Option<&Path>) -> DetectedInstall {
    let paths = InstallPaths {
        home: home.map(Path::to_path_buf),
        local_app_data: None,
        program_files: None,
        program_files_x86: None,
    };
    detect_with(executable, &paths, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn classify(path: &str, home: &str) -> InstallSource {
        detect_with_home(Path::new(path), Some(Path::new(home))).source
    }

    #[test]
    fn detects_cargo_installs() {
        assert_eq!(
            classify("/home/u/.cargo/bin/xfetch", "/home/u"),
            InstallSource::Cargo
        );
    }

    #[cfg(unix)]
    #[test]
    fn detects_prebuilt_installs() {
        assert_eq!(
            classify("/home/u/.local/bin/xfetch", "/home/u"),
            InstallSource::Prebuilt
        );
    }

    #[test]
    fn detects_local_builds() {
        assert_eq!(
            classify("/home/u/repos/xfetch/target/release/xfetch", "/home/u"),
            InstallSource::Unknown
        );
    }

    #[cfg(unix)]
    #[test]
    fn detects_package_manager_paths() {
        assert_eq!(
            classify("/usr/bin/xfetch", "/home/u"),
            InstallSource::PackageManager
        );
        assert_eq!(
            classify("/bin/xfetch", "/home/u"),
            InstallSource::PackageManager
        );
        assert_eq!(
            classify("/opt/homebrew/bin/xfetch", "/home/u"),
            InstallSource::PackageManager
        );
    }

    #[test]
    fn missing_home_falls_back_to_unknown() {
        assert_eq!(
            detect_with_home(Path::new("/random/xfetch"), None).source,
            InstallSource::Unknown
        );
    }

    #[test]
    fn parses_registry_and_path_cargo_installs() {
        let registry = "xfetch-cli v1.0.3:\n    xfetch\n";
        assert_eq!(
            parse_cargo_install_list(registry),
            Some(CargoInstallKind::Registry)
        );

        let source = "xfetch-cli v1.0.3 (C:\\Users\\u\\repos\\xfetch):\n    xfetch.exe\n";
        assert_eq!(
            parse_cargo_install_list(source),
            Some(CargoInstallKind::Source(PathBuf::from(
                "C:\\Users\\u\\repos\\xfetch"
            )))
        );

        assert_eq!(parse_cargo_install_list("other v1.0.0:\n"), None);
    }

    #[test]
    fn package_managed_sources_are_flagged() {
        assert!(InstallSource::Cargo.is_package_managed());
        assert!(InstallSource::SourceBuild.is_package_managed());
        assert!(InstallSource::Winget.is_package_managed());
        assert!(!InstallSource::Prebuilt.is_package_managed());
        assert!(!InstallSource::Unknown.is_package_managed());
    }

    #[test]
    fn receipt_wins_over_path_heuristics() {
        let receipt = InstallReceipt {
            schema_version: 1,
            method: crate::update::receipt::Method::Prebuilt,
            path: PathBuf::from("/home/u/.cargo/bin/xfetch"),
            source_path: None,
            version: None,
            source: None,
            repo: None,
            commit: None,
            installed_at: None,
        };
        let paths = InstallPaths {
            home: Some(PathBuf::from("/home/u")),
            local_app_data: None,
            program_files: None,
            program_files_x86: None,
        };
        let detected = detect_with(
            Path::new("/home/u/.cargo/bin/xfetch"),
            &paths,
            Some(receipt),
        );
        assert_eq!(detected.source, InstallSource::Prebuilt);
    }
}
