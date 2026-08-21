//! Distro logo resolution for `--gen-config`.
//!
//! Fetches the ASCII art of the detected OS/distro from the `xfetch-cli/logos`
//! catalog (index + raw files over HTTPS, via `curl` with a timeout).
//! Everything here is best-effort: on any failure (no network, bad catalog,
//! invalid art) the caller falls back to the default behavior.
//!
//! OS detection is delegated to `crate::info::platform` (one folder per OS),
//! so this module has no `#[cfg(target_os)]` blocks of its own.

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

fn fetch_text(url: &str) -> Option<String> {
    let output = run_cmd_with_timeout("curl", &["-fsS", "--max-time", "10", url], FETCH_TIMEOUT)?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8(output.stdout).ok()
}

/// Rejects control characters that could inject terminal escape sequences:
/// ESC (`\x1b`), C0/C1 controls (e.g. `\x0f`), etc. Only `\n` and `\t` are
/// allowed. The art is later printed raw to the terminal (and embedded in
/// `--gen-config`), so this is a hard gate: a compromised catalog falls back
/// to the built-in default logo instead of delivering payloads.
fn validate_art(art: &str) -> bool {
    art.len() <= MAX_ART_SIZE
        && art
            .chars()
            .all(|c| matches!(c, '\n' | '\t') || !c.is_control())
        && art.lines().all(|l| l.chars().count() <= MAX_LINE_WIDTH)
}

/// Fetches the ASCII art for the logo to embed: the `logo_override` (e.g. the
/// `--logo` flag) when given — resolved on its own, no ID_LIKE fallback — or
/// the detected OS/distro otherwise. Returns `(resolved entry id, art)`;
/// `None` on any failure (callers fall back to the default logo).
pub fn fetch_distro_logo(logo_override: Option<&str>) -> Option<(String, String)> {
    let (detected_id, id_like) = crate::info::platform::detect_os_ids();
    let (query_id, query_like) = match logo_override {
        Some(override_id) => (override_id, Vec::new()),
        None => (detected_id.as_str(), id_like),
    };
    let category = crate::info::platform::logo_category();
    let index: LogoIndex = fetch_text(&format!("{}/logos.json", logos_base_url()))
        .and_then(|s| serde_json::from_str(&s).ok())?;
    let entry = resolve_entry(&index, query_id, &query_like);
    let art_path = entry
        .map(|e| e.file.clone())
        .or_else(|| index.defaults.get(category).cloned())?;
    let art = fetch_text(&format!("{}/{art_path}", logos_base_url()))?;
    let resolved_id = match &entry {
        Some(e) => e.id.clone(),
        None if logo_override.is_some() => {
            eprintln!(
                "Warning: logo '{}' is not in the catalog; using the generic {} logo.",
                query_id, category
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
        assert!(validate_art("  tab\there\n"));
        assert!(!validate_art("a\0b"));
        assert!(!validate_art(&"x".repeat(MAX_ART_SIZE + 1)));
        assert!(!validate_art(&format!(
            "{}\n",
            "y".repeat(MAX_LINE_WIDTH + 1)
        )));
    }

    #[test]
    fn test_validate_art_rejects_escape_sequences() {
        assert!(!validate_art("\x1b[31mred\x1b[0m"));
        assert!(!validate_art("a\x07b"));
        assert!(!validate_art("\x1b]52;c;aGVsbG8=\x07"));
        assert!(!validate_art("\x0e shift-out\x0f"));
        assert!(!validate_art("del\x7f"));
    }
}
