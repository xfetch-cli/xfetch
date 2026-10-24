//! Shared safety helpers for remote plugin/effect/extension installs.
//!
//! User-controlled names feed into temporary clone paths and user-controlled
//! repositories feed into `git clone`. Both are validated here so the three
//! installers cannot diverge: names may not escape the temp directory and
//! repositories may not be parsed as git options.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

const GIT_CMD: &str = "git";

/// Validates a guest name before it is used in a filesystem path.
///
/// Only `[A-Za-z0-9._-]` is accepted; empty names, `.`/`..` and path
/// separators are rejected.
pub fn validate_guest_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Guest name must not be empty".to_string());
    }
    if name == "." || name == ".." {
        return Err(format!("Invalid guest name '{}'", name));
    }
    if name.contains('/') || name.contains('\\') {
        return Err(format!(
            "Invalid guest name '{}': path separators are not allowed",
            name
        ));
    }
    if !name
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err(format!(
            "Invalid guest name '{}': only ASCII letters, digits, '.', '_' and '-' are allowed",
            name
        ));
    }
    Ok(())
}

/// Creates the work directory with owner-only permissions on Unix.
#[cfg(unix)]
fn create_work_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;

    fs::DirBuilder::new().mode(0o700).create(path)
}

/// Creates the work directory with the platform default ACL on Windows.
#[cfg(not(unix))]
fn create_work_dir(path: &Path) -> std::io::Result<()> {
    fs::create_dir(path)
}

/// Creates a fresh, uniquely named working directory under the system temp
/// directory and returns its path.
///
/// The directory is created atomically with `create_dir`/`DirBuilder`, so an
/// existing file or symlink at the chosen path is never followed or removed.
/// On Unix the mode is `0700`, so cloned sources are not world-readable.
pub fn temp_work_dir(prefix: &str, name: &str) -> Result<PathBuf, String> {
    validate_guest_name(name)?;

    let base = std::env::temp_dir();
    let pid = std::process::id();
    for attempt in 0..32u32 {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        let dir = base.join(format!("{}{}-{}-{}-{}", prefix, name, pid, nanos, attempt));
        match create_work_dir(&dir) {
            Ok(()) => return Ok(dir),
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(err) => {
                return Err(format!(
                    "Failed to create temporary directory '{}': {}",
                    dir.display(),
                    err
                ));
            }
        }
    }

    Err("Failed to create a unique temporary install directory".to_string())
}

/// Rejects repository values that `git clone` could parse as options.
pub fn validate_repo(repo: &str) -> Result<(), String> {
    if repo.is_empty() {
        return Err("Repository URL must not be empty".to_string());
    }
    if repo.starts_with('-') {
        return Err(format!(
            "Invalid repository '{}': values starting with '-' are not allowed",
            repo
        ));
    }
    if repo.contains('\0') {
        return Err("Invalid repository: NUL bytes are not allowed".to_string());
    }
    Ok(())
}

/// Runs `git clone --depth 1 -- <repo> <dest>` with the option terminator so a
/// repository value can never be interpreted as a git flag.
pub fn clone_repository(repo: &str, dest: &Path, label: &str) -> Result<(), String> {
    validate_repo(repo)?;

    let status = Command::new(GIT_CMD)
        .args(["clone", "--depth", "1", "--"])
        .arg(repo)
        .arg(dest)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|err| format!("Failed to run git: {}. Is git installed?", err))?;

    if !status.success() {
        return Err(format!("Failed to clone {} repository", label));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_guest_names() {
        for name in ["weather", "wasm-matrix", "night_mode", "v1.2.3", "-leading"] {
            assert!(validate_guest_name(name).is_ok(), "rejected {}", name);
        }
    }

    #[test]
    fn rejects_path_traversal_and_separators() {
        for name in [
            "", ".", "..", "../evil", "a/b", "a\\b", "a b", "a\0b", "café",
        ] {
            assert!(validate_guest_name(name).is_err(), "accepted {:?}", name);
        }
    }

    #[test]
    fn temp_work_dir_is_created_under_temp() {
        let dir = temp_work_dir("xfetch-test-", "guest").expect("temp dir");
        assert!(dir.is_dir());
        assert!(dir.starts_with(std::env::temp_dir()));
        assert!(
            dir.file_name()
                .expect("name")
                .to_string_lossy()
                .contains("guest")
        );
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[cfg(unix)]
    #[test]
    fn temp_work_dir_is_owner_only_on_unix() {
        use std::os::unix::fs::PermissionsExt;

        let dir = temp_work_dir("xfetch-test-", "guest-mode").expect("temp dir");
        let mode = fs::metadata(&dir).expect("metadata").permissions().mode();
        assert_eq!(
            mode & 0o077,
            0,
            "work dir must not be group/world-accessible"
        );
        assert_eq!(mode & 0o700, 0o700, "work dir must be owner-accessible");
        fs::remove_dir_all(&dir).expect("cleanup");
    }

    #[test]
    fn temp_work_dir_rejects_unsafe_names() {
        assert!(temp_work_dir("xfetch-test-", "../escape").is_err());
    }

    #[test]
    fn repo_validation_rejects_options_and_empty_values() {
        assert!(validate_repo("https://github.com/xfetch-cli/plugins.git").is_ok());
        assert!(validate_repo("").is_err());
        assert!(validate_repo("--upload-pack=evil").is_err());
        assert!(validate_repo("-oProxyCommand=evil").is_err());
        assert!(validate_repo("https://example.com/a\0b").is_err());
    }
}
