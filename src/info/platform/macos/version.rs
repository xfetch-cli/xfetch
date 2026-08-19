//! macOS version mapping for the logo catalog (`--gen-config`), mirror of
//! `windows/version.rs`.
//!
//! sysinfo's `os_version()` returns version strings like `"24.0"` (macOS 15 /
//! Sequoia); the numeric major version decides the catalog id.

/// Maps a macOS version string to a catalog id (macOS 15 → sequoia, ...).
pub fn macos_version_id(version: &str) -> Option<&'static str> {
    let v = version.trim();
    for (prefix, id) in [
        ("15", "macos-sequoia"),
        ("14", "macos-sonoma"),
        ("13", "macos-ventura"),
        ("12", "macos-monterey"),
        ("11", "macos-bigsur"),
        ("10.15", "macos-catalina"),
        ("10.14", "macos-mojave"),
        ("10.13", "macos-highsierra"),
        ("10.12", "macos-sierra"),
        ("10.11", "osx-elcapitan"),
        ("10.10", "osx-yosemite"),
        ("10.9", "osx-mavericks"),
        ("10.8", "osx-mountainlion"),
        ("10.7", "osx-lion"),
        ("10.6", "osx-snowleopard"),
        ("10.5", "osx-leopard"),
        ("10.4", "osx-tiger"),
        ("10.3", "osx-panther"),
        ("10.2", "osx-jaguar"),
        ("10.1", "osx-puma"),
        ("10.0", "osx-cheetah"),
    ] {
        if v.starts_with(prefix) {
            return Some(id);
        }
    }
    None
}

/// `(ID, ID_LIKE)` for the running OS: the base id `macos` plus a
/// version-specific candidate (e.g. `macos-ventura`) when the version can be
/// mapped.
pub fn detect_os_ids() -> (String, Vec<String>) {
    let mut ids = Vec::new();
    if let Some(v) = sysinfo::System::os_version()
        && let Some(specific) = macos_version_id(&v)
    {
        ids.push(specific.to_string());
    }
    ("macos".to_string(), ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_version_id() {
        assert_eq!(macos_version_id("15.0"), Some("macos-sequoia"));
        assert_eq!(macos_version_id("14.5"), Some("macos-sonoma"));
        assert_eq!(macos_version_id("13.2.1"), Some("macos-ventura"));
        assert_eq!(macos_version_id("12.7"), Some("macos-monterey"));
        assert_eq!(macos_version_id("11.6"), Some("macos-bigsur"));
        assert_eq!(macos_version_id("10.15.7"), Some("macos-catalina"));
        assert_eq!(macos_version_id("10.9"), Some("osx-mavericks"));
        // Prefixes must not be confused: 10.11 is El Capitan, not Big Sur.
        assert_eq!(macos_version_id("10.11.6"), Some("osx-elcapitan"));
        assert_eq!(macos_version_id("9.0"), None);
        assert_eq!(macos_version_id("26.0"), None);
    }

    #[test]
    fn test_macos_detect_base_is_macos() {
        let (id, like) = detect_os_ids();
        assert_eq!(id, "macos");
        assert!(
            like.iter()
                .all(|l| l.starts_with("macos-") || l.starts_with("osx-"))
        );
    }
}
