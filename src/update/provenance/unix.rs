//! Unix path-based install provenance.

use super::{InstallPaths, InstallSource};
use std::path::Path;

/// Classifies a Unix executable path.
pub(crate) fn classify(executable: &Path, paths: &InstallPaths) -> InstallSource {
    if let Some(home) = &paths.home
        && executable.starts_with(home.join(".cargo").join("bin"))
    {
        return InstallSource::Cargo;
    }

    if is_package_manager_path(executable) {
        return InstallSource::PackageManager;
    }

    if let Some(home) = &paths.home
        && executable.starts_with(home.join(".local").join("bin"))
    {
        return InstallSource::Prebuilt;
    }

    InstallSource::Unknown
}

/// Paths owned by common package managers.
fn is_package_manager_path(executable: &Path) -> bool {
    ["/usr/bin", "/bin", "/opt/homebrew", "/usr/local/Cellar"]
        .iter()
        .any(|prefix| executable.starts_with(prefix))
}
