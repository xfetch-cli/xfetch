//! Windows version mapping for the logo catalog (`--gen-config`).
//!
//! sysinfo's `os_version()` returns NT kernel strings like `"10.0.19045"`;
//! Windows 11 also reports `10.0` (build >= 22000), so the build number
//! decides. Substring matching on digits is unreliable (`"10.0.17763"`
//! contains a `7`), so this maps by version prefix + build.

/// Maps a Windows version string to a catalog id.
pub fn windows_version_id(version: &str) -> Option<&'static str> {
    let v = version.trim().to_lowercase();
    // Windows 11 reports itself as "10.0" (or "11.x" on newer kernels);
    // build >= 22000 decides.
    if v.starts_with("11") || (v.starts_with("10.0") && build_number(&v) >= 22000) {
        return Some("windows-11");
    }
    if v.starts_with("10.0") {
        return Some("windows-10");
    }
    match v.split('.').nth(1) {
        Some("3") => Some("windows-8-1"),
        Some("2") => Some("windows-8"),
        Some("1") => Some("windows-7"),
        _ => None,
    }
}

fn build_number(version: &str) -> u32 {
    version
        .split(['.', ' '])
        .find_map(|t| t.parse::<u32>().ok().filter(|n| *n >= 22000))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_version_id() {
        assert_eq!(windows_version_id("10.0.19045"), Some("windows-10"));
        assert_eq!(windows_version_id("10.0.26100"), Some("windows-11"));
        assert_eq!(windows_version_id("10.0.22000"), Some("windows-11"));
        assert_eq!(windows_version_id("11.0.22631"), Some("windows-11"));
        assert_eq!(windows_version_id("6.3.9600"), Some("windows-8-1"));
        assert_eq!(windows_version_id("6.2.9200"), Some("windows-8"));
        assert_eq!(windows_version_id("6.1.7601"), Some("windows-7"));
        assert_eq!(windows_version_id("6.0.6002"), None);
        // Build numbers must not be confused with major versions.
        assert_eq!(windows_version_id("10.0.17763"), Some("windows-10"));
        assert_eq!(windows_version_id("10.0.20348"), Some("windows-10"));
    }
}
