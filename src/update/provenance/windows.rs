//! Windows path-based install provenance.
//!
//! The classifier is deliberately conservative. Windows has no single
//! package-ownership database for portable binaries, so unknown locations are
//! never treated as writable prebuilt installs. The PowerShell installers
//! write a receipt that resolves the default prebuilt directory without
//! guessing.

use super::{InstallPaths, InstallSource};
use std::path::{Path, PathBuf};

/// Classifies a Windows executable path.
pub(crate) fn classify(executable: &Path, paths: &InstallPaths) -> InstallSource {
    if is_winget_path(executable, paths) {
        return InstallSource::Winget;
    }

    if is_under(executable, paths.home.as_deref(), ".cargo/bin") {
        return InstallSource::Cargo;
    }

    // `install-prebuilt.ps1` uses this directory, but so can a manually copied
    // build. Without an install receipt this is intentionally unknown: the
    // user can force an in-place update with `--prebuilt`.
    if is_under(
        executable,
        paths.local_app_data.as_deref(),
        "Programs/xfetch/bin",
    ) || is_under(executable, paths.home.as_deref(), ".local/bin")
    {
        return InstallSource::Unknown;
    }

    InstallSource::Unknown
}

fn is_winget_path(executable: &Path, paths: &InstallPaths) -> bool {
    let candidates = [
        (paths.local_app_data.as_deref(), "Microsoft/WinGet/Links"),
        (paths.local_app_data.as_deref(), "Microsoft/WinGet/Packages"),
        (paths.program_files.as_deref(), "WinGet/Links"),
        (paths.program_files.as_deref(), "WinGet/Packages"),
        (paths.program_files_x86.as_deref(), "WinGet/Packages"),
    ];
    candidates
        .iter()
        .any(|(base, rest)| is_under(executable, *base, rest))
}

fn is_under(executable: &Path, base: Option<&Path>, rest: &str) -> bool {
    let Some(base) = base else {
        return false;
    };
    let mut prefix: PathBuf = base.to_path_buf();
    for component in rest.split('/') {
        prefix.push(component);
    }
    starts_with_ignore_case(executable, &prefix)
}

fn starts_with_ignore_case(path: &Path, prefix: &Path) -> bool {
    let path = normalized(path);
    let prefix = normalized(prefix);
    path == prefix || path.starts_with(&format!("{prefix}\\"))
}

fn normalized(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        text = format!("\\\\{}", rest);
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        text = rest.to_string();
    }
    text.make_ascii_lowercase();
    text.trim_end_matches('\\').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paths() -> InstallPaths {
        InstallPaths {
            home: Some(PathBuf::from("C:\\Users\\u")),
            local_app_data: Some(PathBuf::from("C:\\Users\\u\\AppData\\Local")),
            program_files: Some(PathBuf::from("C:\\Program Files")),
            program_files_x86: Some(PathBuf::from("C:\\Program Files (x86)")),
        }
    }

    fn classify_path(path: &str) -> InstallSource {
        classify(Path::new(path), &paths())
    }

    #[test]
    fn detects_winget_links_and_packages() {
        assert_eq!(
            classify_path("C:\\Users\\u\\AppData\\Local\\Microsoft\\WinGet\\Links\\xfetch.exe"),
            InstallSource::Winget
        );
        assert_eq!(
            classify_path(
                "C:\\Users\\u\\AppData\\Local\\Microsoft\\WinGet\\Packages\\Xfetch-cli.Xfetch_Microsoft.Winget.Source_8wekyb3d8bbwe\\xfetch.exe"
            ),
            InstallSource::Winget
        );
        assert_eq!(
            classify_path("C:\\Program Files\\WinGet\\Links\\xfetch.exe"),
            InstallSource::Winget
        );
        assert_eq!(
            classify_path("C:\\Program Files\\WinGet\\Packages\\Xfetch-cli.Xfetch_foo\\xfetch.exe"),
            InstallSource::Winget
        );
    }

    #[test]
    fn detects_cargo_installs() {
        assert_eq!(
            classify_path("C:\\Users\\u\\.cargo\\bin\\xfetch.exe"),
            InstallSource::Cargo
        );
    }

    #[test]
    fn does_not_guess_for_prebuilt_directories_without_a_receipt() {
        assert_eq!(
            classify_path("C:\\Users\\u\\AppData\\Local\\Programs\\xfetch\\bin\\xfetch.exe"),
            InstallSource::Unknown
        );
        assert_eq!(
            classify_path("C:\\Users\\u\\.local\\bin\\xfetch.exe"),
            InstallSource::Unknown
        );
    }

    #[test]
    fn leaves_local_and_manual_builds_unknown() {
        assert_eq!(
            classify_path("C:\\Users\\u\\Documents\\repos\\xfetch\\target\\release\\xfetch.exe"),
            InstallSource::Unknown
        );
        assert_eq!(
            classify_path("C:\\Users\\u\\Downloads\\xfetch.exe"),
            InstallSource::Unknown
        );
    }

    #[test]
    fn path_matching_is_case_insensitive_and_handles_verbatim_paths() {
        assert_eq!(
            classify_path("c:\\users\\U\\appdata\\local\\microsoft\\winget\\links\\XFETCH.EXE"),
            InstallSource::Winget
        );
        assert_eq!(
            classify_path("\\\\?\\C:\\Users\\u\\.cargo\\bin\\xfetch.exe"),
            InstallSource::Cargo
        );
        assert_eq!(
            classify_path("C:/Users/u/AppData/Local/Microsoft/WinGet/Links/xfetch.exe"),
            InstallSource::Winget
        );
    }
}
