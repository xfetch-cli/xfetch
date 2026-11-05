//! Windows prebuilt archive extraction and in-place replacement.

use super::super::platform::binary_name;
use super::MAX_ASSET;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use zip::ZipArchive;

/// Extracts `xfetch.exe` from the release zip into a temporary file.
pub(super) fn extract_binary(bytes: &[u8], dir: &Path) -> Result<PathBuf, String> {
    let mut archive = ZipArchive::new(Cursor::new(bytes))
        .map_err(|err| format!("Invalid release archive: {}", err))?;
    let tmp = dir.join(format!(".xfetch-update-{}.exe", std::process::id()));

    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|err| format!("Invalid archive entry: {}", err))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map_err(|err| format!("Invalid archive entry name: {}", err))?;
        if Path::new(name.as_ref())
            .file_name()
            .and_then(|name| name.to_str())
            != Some(binary_name())
        {
            continue;
        }
        if entry.size() > MAX_ASSET as u64 {
            return Err("The release binary exceeds the size limit".to_string());
        }

        let mut file = fs::File::create(&tmp)
            .map_err(|err| format!("Cannot create '{}': {}", tmp.display(), err))?;
        std::io::copy(&mut entry, &mut file)
            .map_err(|err| format!("Failed to extract the binary: {}", err))?;
        let _ = file.sync_all();
        return Ok(tmp);
    }

    let _ = fs::remove_file(&tmp);
    Err("The release archive does not contain the xfetch binary".to_string())
}

/// Keeps one backup and replaces the running executable.
///
/// `self-replace` renames the running image aside and moves the new binary into
/// place. Windows permits renaming a running executable, but not deleting it.
pub(super) fn replace_binary(target_dir: &Path, new_binary: &Path) -> Result<PathBuf, String> {
    let target = target_dir.join(binary_name());
    let backup = target_dir.join("xfetch.exe.bak");

    if target.exists() {
        fs::copy(&target, &backup)
            .map_err(|err| format!("Cannot back up '{}': {}", target.display(), err))?;
    }

    if is_current_executable(&target) {
        // The running image can be renamed but not deleted; let self-replace
        // perform the Windows-specific swap.
        self_replace::self_replace(new_binary).map_err(|err| {
            format!(
                "Cannot replace '{}': {}. Close other xfetch processes and try again",
                target.display(),
                err
            )
        })?;
        let _ = fs::remove_file(new_binary);
    } else {
        // `--bin-dir` can point at an installation that is not the running
        // process. In that case a normal rename is enough (and works when the
        // target file is not in use).
        fs::rename(new_binary, &target).map_err(|err| {
            format!(
                "Cannot replace '{}': {}. Close any xfetch process using it and try again",
                target.display(),
                err
            )
        })?;
    }

    Ok(target)
}

fn is_current_executable(path: &Path) -> bool {
    if !path.exists() {
        return false;
    }
    let Ok(current) = std::env::current_exe() else {
        return false;
    };
    let current = fs::canonicalize(&current).unwrap_or(current);
    let path = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    normalized(&current) == normalized(&path)
}

fn normalized(path: &Path) -> String {
    let mut text = path.to_string_lossy().replace('/', "\\");
    if let Some(rest) = text.strip_prefix("\\\\?\\") {
        text = rest.to_string();
    }
    text.make_ascii_lowercase();
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch-update-win-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn zip_with(name: &str, payload: &[u8]) -> Vec<u8> {
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        writer
            .start_file(name, zip::write::SimpleFileOptions::default())
            .expect("start zip entry");
        writer.write_all(payload).expect("write zip entry");
        writer.finish().expect("finish zip").into_inner()
    }

    #[test]
    fn extracts_xfetch_exe_from_the_archive() {
        let dir = scratch("extract");
        let archive = zip_with("xfetch.exe", b"binary-bytes");
        let extracted = extract_binary(&archive, &dir).expect("extract");
        assert_eq!(fs::read(&extracted).expect("read"), b"binary-bytes");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn finds_xfetch_exe_in_a_nested_directory() {
        let dir = scratch("nested");
        let archive = zip_with("stage/xfetch.exe", b"nested-bytes");
        let extracted = extract_binary(&archive, &dir).expect("extract");
        assert_eq!(fs::read(extracted).expect("read"), b"nested-bytes");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_archives_without_the_binary() {
        let dir = scratch("missing");
        let archive = zip_with("README.md", b"docs");
        assert!(extract_binary(&archive, &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_invalid_archives() {
        let dir = scratch("invalid");
        assert!(extract_binary(b"not a zip", &dir).is_err());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn replaces_a_non_running_target_and_keeps_one_backup() {
        let dir = scratch("replace");
        let target = dir.join("xfetch.exe");
        fs::write(&target, b"old").expect("write old");
        let new_binary = dir.join(".xfetch-update-test-new");
        fs::write(&new_binary, b"new").expect("write new");

        let replaced = replace_binary(&dir, &new_binary).expect("replace");
        assert_eq!(replaced, target);
        assert_eq!(fs::read(&target).expect("read"), b"new");
        assert_eq!(
            fs::read(dir.join("xfetch.exe.bak")).expect("backup"),
            b"old"
        );
        assert!(!new_binary.exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
