//! In-place prebuilt updates (Unix).
//!
//! The release tarball is downloaded, verified against the published
//! `SHA256SUMS`, extracted to a hidden temporary file next to the target and
//! moved over the current executable with a single atomic rename. The
//! previous binary is kept once as `xfetch.bak` (overwritten by the next
//! update, so no version graveyard accumulates).

use super::github::{self, Client, Release};
use crate::hash::sha256_hex;
use std::fs;
use std::io::{Cursor, Read};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Upper bound for a release asset (the binaries are a few MiB).
const MAX_ASSET: usize = 200 * 1024 * 1024;
/// Upper bound for the `SHA256SUMS` file.
const MAX_SUMS: usize = 1024 * 1024;

/// Downloads, verifies and installs the release for this platform.
pub fn update_prebuilt(
    client: &Client,
    release: &Release,
    target_dir: &Path,
) -> Result<PathBuf, String> {
    let target = release_target()
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

/// The release workflow target triple for the running platform.
pub fn release_target() -> Option<&'static str> {
    if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "gnu"
    )) {
        Some("x86_64-unknown-linux-gnu")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "x86_64",
        target_env = "musl"
    )) {
        Some("x86_64-unknown-linux-musl")
    } else if cfg!(all(
        target_os = "linux",
        target_arch = "aarch64",
        target_env = "gnu"
    )) {
        Some("aarch64-unknown-linux-gnu")
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        Some("x86_64-apple-darwin")
    } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        Some("aarch64-apple-darwin")
    } else {
        None
    }
}

/// Extracts the `xfetch` entry from the release tarball into a temporary file.
fn extract_binary(bytes: &[u8], dir: &Path) -> Result<PathBuf, String> {
    let decoder = flate2::read::GzDecoder::new(Cursor::new(bytes));
    let mut archive = tar::Archive::new(decoder);
    let tmp = dir.join(format!(".xfetch-update-{}", std::process::id()));

    let entries = archive
        .entries()
        .map_err(|err| format!("Invalid release archive: {}", err))?;
    for entry in entries {
        let mut entry = entry.map_err(|err| format!("Invalid archive entry: {}", err))?;
        let path = entry
            .path()
            .map_err(|err| format!("Invalid archive path: {}", err))?;
        if path.file_name().and_then(|name| name.to_str()) != Some("xfetch") {
            continue;
        }

        let mut file = fs::File::create(&tmp)
            .map_err(|err| format!("Cannot create '{}': {}", tmp.display(), err))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|err| format!("Failed to extract the binary: {}", err))?;
        let _ = file.sync_all();
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))
            .map_err(|err| format!("Cannot mark '{}' executable: {}", tmp.display(), err))?;
        return Ok(tmp);
    }

    let _ = fs::remove_file(&tmp);
    Err("The release archive does not contain the xfetch binary".to_string())
}

/// Keeps one backup and moves the new binary into place atomically.
fn replace_binary(target_dir: &Path, new_binary: &Path) -> Result<PathBuf, String> {
    let target = target_dir.join("xfetch");
    let backup = target_dir.join("xfetch.bak");

    if target.exists() {
        fs::copy(&target, &backup)
            .map_err(|err| format!("Cannot back up '{}': {}", target.display(), err))?;
    }

    fs::rename(new_binary, &target)
        .map_err(|err| format!("Cannot replace '{}': {}", target.display(), err))?;

    Ok(target)
}

/// Reads a bounded stream into memory.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
    use super::*;
    use flate2::Compression;
    use flate2::write::GzEncoder;
    use std::io::Write;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch-update-test-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn tarball(name: &str, payload: &[u8]) -> Vec<u8> {
        let encoder = GzEncoder::new(Vec::new(), Compression::fast());
        let mut builder = tar::Builder::new(encoder);

        let mut header = tar::Header::new_gnu();
        header.set_size(payload.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        builder
            .append_data(&mut header, format!("./{}", name), payload)
            .expect("append");

        builder
            .into_inner()
            .expect("finish tar")
            .finish()
            .expect("finish gzip")
    }

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn extracts_xfetch_from_the_archive() {
        let dir = scratch("extract");
        let archive = tarball("xfetch", b"binary-bytes");
        let extracted = extract_binary(&archive, &dir).expect("extract");
        assert_eq!(fs::read(&extracted).expect("read"), b"binary-bytes");
        let mode = fs::metadata(&extracted)
            .expect("metadata")
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_archives_without_the_binary() {
        let dir = scratch("missing");
        let archive = tarball("README.md", b"docs");
        assert!(extract_binary(&archive, &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_the_binary_and_keeps_one_backup() {
        let dir = scratch("replace");
        let target = dir.join("xfetch");
        fs::write(&target, b"old").expect("write old");
        let new_binary = dir.join(".xfetch-update-test-new");
        fs::write(&new_binary, b"new").expect("write new");

        let replaced = replace_binary(&dir, &new_binary).expect("replace");
        assert_eq!(replaced, target);
        assert_eq!(fs::read(&target).expect("read"), b"new");
        assert_eq!(fs::read(dir.join("xfetch.bak")).expect("backup"), b"old");
        assert!(!new_binary.exists());

        // A second update overwrites the same backup instead of accumulating.
        let newer = dir.join(".xfetch-update-test-newer");
        fs::write(&newer, b"newest").expect("write newest");
        replace_binary(&dir, &newer).expect("replace again");
        assert_eq!(fs::read(dir.join("xfetch.bak")).expect("backup"), b"new");
        let backups: Vec<_> = fs::read_dir(&dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".bak"))
            .collect();
        assert_eq!(backups.len(), 1, "only one backup file must exist");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn reads_bounded_rejects_oversized_payloads() {
        let mut small = &b"abcd"[..];
        assert_eq!(read_bounded(&mut small, 8).expect("small"), b"abcd");
        let mut big = &b"abcdef"[..];
        assert!(read_bounded(&mut big, 3).is_err());
    }

    #[test]
    fn release_target_is_known_on_supported_platforms() {
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        assert!(release_target().is_some());
        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        assert!(release_target().is_none());
    }

    #[test]
    fn tarball_helper_roundtrips() {
        let dir = scratch("roundtrip");
        let archive = tarball("xfetch", b"payload");
        let extracted = extract_binary(&archive, &dir).expect("extract");
        assert_eq!(fs::read(extracted).expect("read"), b"payload");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn encoder_writes_are_flushed() {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(b"x").expect("write");
        assert!(!encoder.finish().expect("finish").is_empty());
    }

    /// Full path against a real GitHub release: fetch, download, verify the
    /// checksum, extract and replace. Ignored by default so offline builds and
    /// CI do not depend on the network.
    #[test]
    #[ignore = "requires network access"]
    fn downloads_verifies_and_runs_the_latest_release() {
        let client = Client::new().expect("client");
        let release =
            github::fetch_latest(&client, &super::super::api_base()).expect("fetch latest release");
        let dir = scratch("network");

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
