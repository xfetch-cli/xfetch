//! Install receipts written by the platform installers.
//!
//! A path alone cannot always tell how a binary was installed: the default
//! prebuilt directory on Windows can also receive a manually copied build, and
//! a `cargo install --path` binary lives next to a crates.io install. The
//! installers therefore leave a small JSON receipt that the updater reads
//! first. Receipts are advisory: a missing or malformed file falls back to the
//! path-based detection in `provenance`.

use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Receipt schema accepted by this version of xfetch.
pub const SCHEMA_VERSION: u32 = 1;

/// How the binary was installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Method {
    /// Installed by `install-prebuilt.ps1` or `install-prebuilt.sh`.
    Prebuilt,
    /// Built from a source checkout, usually by `install.ps1`.
    Source,
    /// Installed with `cargo install xfetch-cli`.
    Cargo,
    /// Installed with winget.
    Winget,
    /// Copied manually by the user.
    Manual,
}

/// Contents of `install.json`.
#[allow(dead_code)]
#[derive(Debug, Clone, Deserialize)]
pub struct InstallReceipt {
    /// Must match [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Installation method recorded by the installer.
    pub method: Method,
    /// Path of the installed executable.
    pub path: PathBuf,
    /// Source checkout when `method` is `source`.
    #[serde(default)]
    pub source_path: Option<PathBuf>,
    /// Installed version, when the installer knows it.
    #[serde(default)]
    pub version: Option<String>,
    /// Human-readable installer name, for diagnostics.
    #[serde(default)]
    pub source: Option<String>,
    /// Repository URL for source installs.
    #[serde(default)]
    pub repo: Option<String>,
    /// Git commit for source installs.
    #[serde(default)]
    pub commit: Option<String>,
    /// RFC 3339 timestamp written by the installer.
    #[serde(default)]
    pub installed_at: Option<String>,
}

impl InstallReceipt {
    /// Reads the receipt next to the user's data directory, if present.
    pub fn load() -> Option<Self> {
        Self::load_from(&receipt_path()?)
    }

    /// Reads and validates a receipt from an explicit path.
    pub fn load_from(path: &Path) -> Option<Self> {
        let text = fs::read_to_string(path).ok()?;
        let receipt: Self = serde_json::from_str(&text).ok()?;
        (receipt.schema_version == SCHEMA_VERSION).then_some(receipt)
    }

    /// Whether this receipt describes `executable`.
    ///
    /// Paths are canonicalized when possible; Windows comparisons are
    /// case-insensitive and tolerate the `\\?\` verbatim prefix.
    pub fn matches_executable(&self, executable: &Path) -> bool {
        same_path(&self.path, executable)
    }
}

/// Location of the receipt for the current user.
pub fn receipt_path() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        dirs::data_local_dir().map(|dir| dir.join("xfetch").join("install.json"))
    }
    #[cfg(not(windows))]
    {
        dirs::config_dir().map(|dir| dir.join("xfetch").join("install.json"))
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left = fs::canonicalize(left).unwrap_or_else(|_| left.to_path_buf());
    let right = fs::canonicalize(right).unwrap_or_else(|_| right.to_path_buf());
    path_key(&left) == path_key(&right)
}

#[cfg(windows)]
fn path_key(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = text.strip_prefix("\\\\?\\UNC\\") {
        text = format!("\\\\{}", rest);
    } else if let Some(rest) = text.strip_prefix("\\\\?\\") {
        text = rest.to_string();
    }
    text.make_ascii_lowercase();
    text.trim_end_matches('\\').to_string()
}

#[cfg(not(windows))]
fn path_key(path: &Path) -> String {
    path.to_string_lossy().trim_end_matches('/').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch-receipt-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    #[test]
    fn parses_a_prebuilt_receipt() {
        let text = r#"{
            "schema_version": 1,
            "method": "prebuilt",
            "path": "C:\\Users\\u\\AppData\\Local\\Programs\\xfetch\\bin\\xfetch.exe",
            "version": "1.0.3",
            "source": "install-prebuilt.ps1",
            "installed_at": "2026-11-05T10:00:00Z"
        }"#;
        let receipt: InstallReceipt = serde_json::from_str(text).expect("parse");
        assert_eq!(receipt.schema_version, 1);
        assert_eq!(receipt.method, Method::Prebuilt);
        assert_eq!(receipt.version.as_deref(), Some("1.0.3"));
        assert!(receipt.source_path.is_none());
    }

    #[test]
    fn parses_a_source_receipt() {
        let text = r#"{
            "schema_version": 1,
            "method": "source",
            "path": "/home/u/.cargo/bin/xfetch",
            "source_path": "/home/u/repos/xfetch",
            "repo": "https://github.com/xfetch-cli/xfetch.git",
            "commit": "abc123"
        }"#;
        let receipt: InstallReceipt = serde_json::from_str(text).expect("parse");
        assert_eq!(receipt.method, Method::Source);
        assert_eq!(
            receipt.source_path.as_deref(),
            Some(Path::new("/home/u/repos/xfetch"))
        );
    }

    #[test]
    fn rejects_unsupported_schema_versions() {
        let dir = scratch("schema");
        let path = dir.join("install.json");
        fs::write(
            &path,
            r#"{"schema_version": 2, "method": "prebuilt", "path": "xfetch"}"#,
        )
        .expect("write");
        assert!(InstallReceipt::load_from(&path).is_none());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn matches_the_recorded_executable() {
        let dir = scratch("match");
        let binary = dir.join("xfetch");
        fs::write(&binary, b"binary").expect("write");
        let text = format!(
            r#"{{"schema_version":1,"method":"prebuilt","path":"{}"}}"#,
            binary.to_string_lossy().replace('\\', "\\\\")
        );
        let receipt: InstallReceipt = serde_json::from_str(&text).expect("parse");
        assert!(receipt.matches_executable(&binary));
        assert!(!receipt.matches_executable(&dir.join("other")));
        let _ = fs::remove_dir_all(&dir);
    }
}
