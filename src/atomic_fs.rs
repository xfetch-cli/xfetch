//! Atomic file replacement helpers for the installers.
//!
//! Installing a binary or a manifest must never leave a half-written file
//! behind: a killed process, a full disk or a permission error mid-write
//! would corrupt the previous installation. Every destination is therefore
//! written through a hidden temporary file in the same directory and moved
//! over the target with a single rename:
//!
//! - The temporary lives next to the destination, so the rename stays inside
//!   one filesystem and is atomic even when the config directory sits on a
//!   different mount than `/tmp`.
//! - On error the temporary is removed and the previous file stays intact.
//! - On Windows, renaming over an in-use binary fails cleanly instead of
//!   truncating it (which is what the old direct copy did).

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Monotonic suffix so consecutive calls in one process cannot collide.
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Copies `source` over `dest` atomically, preserving its permission bits.
pub fn copy_atomic(source: &Path, dest: &Path) -> io::Result<()> {
    let temp = temp_path(dest);
    let result = (|| {
        fs::copy(source, &temp)?;
        // fs::copy does not flush; sync before renaming so a power loss never
        // exposes a partially written file at the final path.
        sync_temp(&temp)?;
        fs::rename(&temp, dest)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Flushes the staged temporary to disk. On Windows `FlushFileBuffers`
/// requires a handle with write access, so the read-only handle used on Unix
/// (`fs::copy` can leave the temporary read-only) fails with `Access Denied`
/// and every install was aborted. Unix keeps its read-only handle.
fn sync_temp(path: &Path) -> io::Result<()> {
    #[cfg(windows)]
    {
        fs::OpenOptions::new().write(true).open(path)?.sync_all()
    }
    #[cfg(not(windows))]
    {
        fs::File::open(path)?.sync_all()
    }
}

/// Writes `contents` to `dest` atomically.
pub fn write_atomic(dest: &Path, contents: &[u8]) -> io::Result<()> {
    let temp = temp_path(dest);
    let result = (|| {
        let mut file = fs::File::create(&temp)?;
        file.write_all(contents)?;
        file.sync_all()?;
        fs::rename(&temp, dest)
    })();

    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

/// Builds the hidden temporary path used for `dest`.
fn temp_path(dest: &Path) -> PathBuf {
    let name = dest
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("xfetch");
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    dest.with_file_name(format!(
        ".{}.xfetch-tmp-{}-{}",
        name,
        std::process::id(),
        counter
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch-atomic-fs-{}-{}", tag, std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("create scratch");
        dir
    }

    fn names(dir: &Path) -> Vec<String> {
        let mut entries: Vec<String> = fs::read_dir(dir)
            .expect("read dir")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        entries
    }

    #[test]
    fn write_atomic_replaces_an_existing_file() {
        let dir = scratch("write");
        let dest = dir.join("config.json");
        fs::write(&dest, b"old").expect("seed");

        write_atomic(&dest, b"new").expect("write");

        assert_eq!(fs::read(&dest).expect("read"), b"new");
        assert_eq!(names(&dir), vec!["config.json"], "no temporary must remain");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_atomic_replaces_and_preserves_permissions() {
        let dir = scratch("copy");
        let source = dir.join("source");
        let dest = dir.join("xfetch");
        fs::write(&source, b"binary").expect("source");
        fs::write(&dest, b"old").expect("seed");

        copy_atomic(&source, &dest).expect("copy");

        assert_eq!(fs::read(&dest).expect("read"), b"binary");
        assert_eq!(names(&dir), vec!["source", "xfetch"]);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).expect("mode");
            copy_atomic(&source, &dest).expect("copy again");
            let mode = fs::metadata(&dest).expect("metadata").permissions().mode();
            assert_eq!(mode & 0o777, 0o755);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn copy_atomic_failure_keeps_the_destination() {
        let dir = scratch("copy-fail");
        let dest = dir.join("xfetch");
        fs::write(&dest, b"old").expect("seed");

        // A directory as source makes fs::copy fail before touching dest.
        let bad_source = dir.join("source-dir");
        fs::create_dir_all(&bad_source).expect("source dir");

        assert!(copy_atomic(&bad_source, &dest).is_err());
        assert_eq!(fs::read(&dest).expect("read"), b"old");
        assert_eq!(
            names(&dir),
            vec!["source-dir", "xfetch"],
            "the temporary must be cleaned up"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn write_atomic_failure_in_a_missing_directory_is_clean() {
        let dir = scratch("write-fail");
        let dest = dir.join("missing").join("file.json");

        assert!(write_atomic(&dest, b"data").is_err());
        assert_eq!(names(&dir), Vec::<String>::new());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn temp_path_is_hidden_and_next_to_the_destination() {
        let dest = Path::new("/some/dir/xfetch-plugin-foo");
        let temp = temp_path(dest);
        assert_eq!(temp.parent(), dest.parent());
        let name = temp.file_name().expect("name").to_string_lossy();
        assert!(
            name.starts_with(".xfetch-plugin-foo.xfetch-tmp-"),
            "{}",
            name
        );
    }
}
