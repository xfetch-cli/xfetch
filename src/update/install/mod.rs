//! In-place prebuilt updates.
//!
//! The release asset is downloaded, verified against the published
//! `SHA256SUMS`, extracted to a hidden temporary file next to the target and
//! moved over the current executable. The previous binary is kept once as
//! `xfetch.bak` / `xfetch.exe.bak` (overwritten by the next update, so no
//! version graveyard accumulates).
//!
//! Unix uses an atomic rename. Windows cannot delete a running executable, so
//! the replacement is delegated to `self-replace`, which renames the running
//! image aside and moves the new one into place.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use super::github::{self, Client, Release};
use super::platform;
use crate::hash::sha256_hex;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Upper bound for a release asset (the binaries are a few MiB).
pub(super) const MAX_ASSET: usize = 200 * 1024 * 1024;
/// Upper bound for the `SHA256SUMS` file.
const MAX_SUMS: usize = 1024 * 1024;

/// Downloads, verifies and installs the release for this platform.
pub fn update_prebuilt(
    client: &Client,
    release: &Release,
    target_dir: &Path,
) -> Result<PathBuf, String> {
    let target = platform::release_target()
        .ok_or_else(|| "Prebuilt updates are not available for this platform".to_string())?;

    let asset = github::choose_asset(release, target).ok_or_else(|| {
        format!(
            "Release {} does not publish a prebuilt binary for {}",
            release.tag, target
        )
    })?;
    let sums = github::checksums_asset(release)
        .ok_or_else(|| format!("Release {} does not publish SHA256SUMS", release.tag))?;

    let sums_text = client.get_text(&sums.url, MAX_SUMS, Duration::from_secs(30))?;
    let expected = github::parse_checksum(&sums_text, &asset.name).ok_or_else(|| {
        format!(
            "SHA256SUMS does not list {}; refusing to install an unverified binary",
            asset.name
        )
    })?;

    let bytes = client.get_bytes(&asset.url, MAX_ASSET)?;
    let actual = sha256_hex(&bytes);
    if actual != expected {
        return Err(format!(
            "Checksum mismatch for {}: expected {}, got {}",
            asset.name, expected, actual
        ));
    }

    fs::create_dir_all(target_dir)
        .map_err(|err| format!("Cannot create '{}': {}", target_dir.display(), err))?;

    let new_binary = extract_binary(&bytes, target_dir)?;
    match replace_binary(target_dir, &new_binary) {
        Ok(path) => Ok(path),
        Err(err) => {
            let _ = fs::remove_file(&new_binary);
            Err(err)
        }
    }
}

fn extract_binary(bytes: &[u8], dir: &Path) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        unix::extract_binary(bytes, dir)
    }
    #[cfg(windows)]
    {
        windows::extract_binary(bytes, dir)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (bytes, dir);
        Err("Prebuilt updates are not available for this platform".to_string())
    }
}

fn replace_binary(target_dir: &Path, new_binary: &Path) -> Result<PathBuf, String> {
    #[cfg(unix)]
    {
        unix::replace_binary(target_dir, new_binary)
    }
    #[cfg(windows)]
    {
        windows::replace_binary(target_dir, new_binary)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target_dir, new_binary);
        Err("Prebuilt updates are not available for this platform".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn release_target_is_known_on_supported_platforms() {
        #[cfg(any(target_os = "linux", target_os = "macos", target_os = "windows"))]
        assert!(platform::release_target().is_some());
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        assert!(platform::release_target().is_none());
    }

    #[test]
    fn reads_bounded_rejects_oversized_payloads() {
        use std::io::Read;

        fn read_bounded(reader: impl Read, cap: usize) -> Result<Vec<u8>, String> {
            let mut body = Vec::new();
            reader
                .take(cap as u64 + 1)
                .read_to_end(&mut body)
                .map_err(|err| format!("Read failed: {}", err))?;
            if body.len() > cap {
                return Err("Payload exceeds the size limit".to_string());
            }
            Ok(body)
        }

        let mut small = &b"abcd"[..];
        assert_eq!(read_bounded(&mut small, 8).expect("small"), b"abcd");
        let mut big = &b"abcdef"[..];
        assert!(read_bounded(&mut big, 3).is_err());
    }

    /// Full path against a real GitHub release: fetch, download, verify the
    /// checksum, extract and replace. Ignored by default so offline builds and
    /// CI do not depend on the network. Unix-only because running it on
    /// Windows would replace the test harness itself.
    #[cfg(unix)]
    #[test]
    #[ignore = "requires network access"]
    fn downloads_verifies_and_runs_the_latest_release() {
        let client = Client::new().expect("client");
        let release =
            github::fetch_latest(&client, &super::super::api_base()).expect("fetch latest release");
        let dir =
            std::env::temp_dir().join(format!("xfetch-update-test-network-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");

        let installed = update_prebuilt(&client, &release, &dir).expect("update");
        assert_eq!(
            installed.file_name().and_then(|n| n.to_str()),
            Some("xfetch")
        );

        let output = std::process::Command::new(&installed)
            .arg("--version")
            .output()
            .expect("run installed binary");
        assert!(output.status.success(), "installed binary failed to run");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.to_ascii_lowercase().contains("xfetch"),
            "unexpected version output: {}",
            stdout
        );

        let _ = fs::remove_dir_all(&dir);
    }
}
