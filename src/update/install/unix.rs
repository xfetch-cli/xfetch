//! Unix prebuilt archive extraction and in-place replacement.

use super::super::platform::binary_name;
use std::fs;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

/// Extracts the `xfetch` entry from the release tarball into a temporary file.
pub(super) fn extract_binary(bytes: &[u8], dir: &Path) -> Result<PathBuf, String> {
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
        if path.file_name().and_then(|name| name.to_str()) != Some(binary_name()) {
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
pub(super) fn replace_binary(target_dir: &Path, new_binary: &Path) -> Result<PathBuf, String> {
    let target = target_dir.join(binary_name());
    let backup = target_dir.join("xfetch.bak");

    if target.exists() {
        fs::copy(&target, &backup)
            .map_err(|err| format!("Cannot back up '{}': {}", target.display(), err))?;
    }

    fs::rename(new_binary, &target)
        .map_err(|err| format!("Cannot replace '{}': {}", target.display(), err))?;

    Ok(target)
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
}
