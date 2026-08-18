//! Distro logo resolution for `--gen-config`.
//!
//! Fetches the ASCII art of the detected OS/distro from the `xfetch-cli/logos`
//! catalog (index + raw files over HTTPS, via `curl` with a timeout).
//! Everything here is best-effort: on any failure (no network, bad catalog,
//! invalid art) the caller falls back to the default behavior.

use std::collections::HashMap;
use std::time::Duration;

use serde::Deserialize;

use crate::info::platform::shared::commands::run_cmd_with_timeout;

const LOGOS_BASE_URL: &str = "https://raw.githubusercontent.com/xfetch-cli/logos/main";
/// Env override for testing against local mirrors/fork branches
/// (e.g. `XFETCH_LOGOS_URL=http://localhost:8000`).
const LOGOS_URL_ENV: &str = "XFETCH_LOGOS_URL";
const FETCH_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_ART_SIZE: usize = 64 * 1024;
const MAX_LINE_WIDTH: usize = 200;

fn logos_base_url() -> String {
    std::env::var(LOGOS_URL_ENV).unwrap_or_else(|_| LOGOS_BASE_URL.to_string())
}

#[derive(Debug, Deserialize)]
pub struct LogoIndex {
    pub defaults: HashMap<String, String>,
    pub logos: Vec<LogoEntry>,
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct LogoEntry {
    pub id: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub file: String,
}

/// `(ID, ID_LIKE)` for the running OS. On Linux these come from
/// `/etc/os-release` (lowercased); on macOS/Windows the base id plus a
/// version-specific candidate (e.g. `macos-ventura`, `windows-11`) when the
/// version can be mapped.
pub fn detect_os_ids() -> (String, Vec<String>) {
    #[cfg(target_os = "linux")]
    {
        parse_os_release(&std::fs::read_to_string("/etc/os-release").unwrap_or_default())
    }
    #[cfg(target_os = "macos")]
    {
        let mut ids = Vec::new();
        if let Some(v) = sysinfo::System::os_version()
            && let Some(specific) = macos_version_id(&v)
        {
            ids.push(specific.to_string());
        }
        ("macos".to_string(), ids)
    }
    #[cfg(target_os = "windows")]
    {
        let mut ids = Vec::new();
        if let Some(v) = sysinfo::System::os_version()
            && let Some(specific) = windows_version_id(&v)
        {
            ids.push(specific.to_string());
        }
        ("windows".to_string(), ids)
    }
}

/// Parses `ID` and `ID_LIKE` out of an os-release file.
fn parse_os_release(content: &str) -> (String, Vec<String>) {
    let mut id = "linux".to_string();
    let mut id_like = Vec::new();
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key {
            "ID" if !value.is_empty() => id = value.to_lowercase(),
            "ID_LIKE" => id_like = value.split_whitespace().map(|s| s.to_lowercase()).collect(),
            _ => {}
        }
    }
    (id, id_like)
}

/// Maps a macOS version string to a catalog id (macOS 15 → sequoia, ...).
#[cfg(target_os = "macos")]
fn macos_version_id(version: &str) -> Option<&'static str> {
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

/// Maps a Windows version string to a catalog id. Windows 11 reports itself
/// as "10.0" (build >= 22000), so the build number decides when present.
#[cfg(target_os = "windows")]
fn windows_version_id(version: &str) -> Option<&'static str> {
    let v = version.to_lowercase();
    if v.contains("11") || v.contains("10.0") && build_number(&v) >= 22000 {
        Some("windows-11")
    } else if v.contains("8.1") {
        Some("windows-8-1")
    } else if v.contains("8") {
        Some("windows-8")
    } else if v.contains("7") {
        Some("windows-7")
    } else if v.contains("10") {
        Some("windows-10")
    } else {
        None
    }
}

#[cfg(target_os = "windows")]
fn build_number(version: &str) -> u32 {
    version
        .split(['.', ' '])
        .find_map(|t| t.parse::<u32>().ok().filter(|n| *n >= 22000))
        .unwrap_or(0)
}

fn entry_matches(entry: &LogoEntry, key: &str) -> bool {
    entry.id.eq_ignore_ascii_case(key) || entry.aliases.iter().any(|a| a.eq_ignore_ascii_case(key))
}

/// Resolves the catalog entry: exact `ID` first, then each `ID_LIKE` token in
/// order. `None` when nothing matches.
pub fn resolve_entry<'a>(
    index: &'a LogoIndex,
    id: &str,
    id_like: &[String],
) -> Option<&'a LogoEntry> {
    index
        .logos
        .iter()
        .find(|e| entry_matches(e, id))
        .or_else(|| {
            id_like
                .iter()
                .find_map(|like| index.logos.iter().find(|e| entry_matches(e, like)))
        })
}

fn category() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
}

fn fetch_text(url: &str) -> Option<String> {
    let output = run_cmd_with_timeout("curl", &["-fsS", "--max-time", "10", url], FETCH_TIMEOUT)?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

fn validate_art(art: &str) -> bool {
    art.len() <= MAX_ART_SIZE
        && !art.contains('\0')
        && art.lines().all(|l| l.chars().count() <= MAX_LINE_WIDTH)
}

/// Fetches the ASCII art for the logo to embed: the `logo_override` (e.g. the
/// `--logo` flag) when given — resolved on its own, no ID_LIKE fallback — or
/// the detected OS/distro otherwise. Returns `(resolved entry id, art)`;
/// `None` on any failure (callers fall back to the default logo).
pub fn fetch_distro_logo(logo_override: Option<&str>) -> Option<(String, String)> {
    let (detected_id, id_like) = detect_os_ids();
    let (query_id, query_like) = match logo_override {
        Some(override_id) => (override_id, Vec::new()),
        None => (detected_id.as_str(), id_like),
    };
    let index: LogoIndex = fetch_text(&format!("{}/logos.json", logos_base_url()))
        .and_then(|s| serde_json::from_str(&s).ok())?;
    let entry = resolve_entry(&index, query_id, &query_like);
    let art_path = entry
        .map(|e| e.file.clone())
        .or_else(|| index.defaults.get(category()).cloned())?;
    let art = fetch_text(&format!("{}/{art_path}", logos_base_url()))?;
    let resolved_id = match &entry {
        Some(e) => e.id.clone(),
        None if logo_override.is_some() => {
            eprintln!(
                "Warning: logo '{}' is not in the catalog; using the generic {} logo.",
                query_id,
                category()
            );
            "default".to_string()
        }
        None => query_id.to_string(),
    };
    validate_art(&art).then_some((resolved_id, art))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_index() -> LogoIndex {
        serde_json::from_str(
            r#"{
                "defaults": { "linux": "defaults/linux/default.txt" },
                "logos": [
                    { "id": "ubuntu", "aliases": ["ubuntu linux", "noble"], "file": "defaults/linux/ubuntu.txt" },
                    { "id": "linux-mint", "aliases": ["linux mint", "linuxmint"], "file": "defaults/linux/linux-mint.txt" },
                    { "id": "debian", "file": "defaults/linux/debian.txt" }
                ]
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn test_parse_os_release() {
        let (id, like) =
            parse_os_release("NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"24.04\"\n");
        assert_eq!(id, "ubuntu");
        assert_eq!(like, vec!["debian"]);

        let (id, like) = parse_os_release("ID=\"Linux Mint\"\nID_LIKE=\"ubuntu debian\"\n");
        assert_eq!(id, "linux mint");
        assert_eq!(like, vec!["ubuntu", "debian"]);

        let (id, _) = parse_os_release("");
        assert_eq!(id, "linux");
    }

    #[test]
    fn test_resolve_entry_by_id() {
        let index = fixture_index();
        assert_eq!(
            resolve_entry(&index, "ubuntu", &[]).map(|e| e.file.as_str()),
            Some("defaults/linux/ubuntu.txt")
        );
        assert_eq!(
            resolve_entry(&index, "linuxmint", &[]).map(|e| e.id.as_str()),
            Some("linux-mint")
        );
    }

    #[test]
    fn test_resolve_entry_by_id_like() {
        let index = fixture_index();
        assert_eq!(
            resolve_entry(&index, "linux", &["debian".to_string()]).map(|e| e.id.as_str()),
            Some("debian")
        );
        assert_eq!(resolve_entry(&index, "void", &[]), None);
    }

    #[test]
    fn test_resolve_entry_case_insensitive() {
        let index = fixture_index();
        assert_eq!(
            resolve_entry(&index, "UBUNTU", &[]).map(|e| e.id.as_str()),
            Some("ubuntu")
        );
    }

    #[test]
    fn test_validate_art() {
        assert!(validate_art("███\n█ █\n"));
        assert!(!validate_art("a\0b"));
        assert!(!validate_art(&"x".repeat(MAX_ART_SIZE + 1)));
        assert!(!validate_art(&format!(
            "{}\n",
            "y".repeat(MAX_LINE_WIDTH + 1)
        )));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_detect_on_current_system() {
        let (id, like) = detect_os_ids();
        assert!(!id.is_empty());
        assert!(like.iter().all(|l| !l.is_empty()));
    }
}
