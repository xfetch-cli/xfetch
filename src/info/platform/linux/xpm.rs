//! Package counts for the X package manager (`xpm`) and its AUR helper
//! (`xpm-aur`).
//!
//! `xpm` keeps its own local database at `/var/lib/xpm/local` and mirrors
//! transactions into pacman's database. Each `<package>/version` file marks a
//! valid installed package and `<package>/origin` records the repository it
//! came from. The native X repository is `x`, so packages with that origin are
//! reported as `xp` (native `.xp` packages). Packages pulled from Arch
//! repositories through xpm's pacman compatibility layer still count towards
//! the `xpm` total but not towards `xp`.
//!
//! `xpm-aur` tracks the AUR packages it installed in
//! `$XDG_STATE_HOME/xpm-aur/state.json`; the count is the number of installed
//! split outputs across all tracked package bases. The sidecar is intersected
//! with the xpm local database so only packages that are actually installed
//! are counted.

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use serde::Deserialize;

pub const XPM_LABEL: &str = "xpm";
pub const XP_LABEL: &str = "xp";
pub const XPM_AUR_LABEL: &str = "xpm-aur";

const XPM_DB_DIR: &str = "/var/lib/xpm/local";
const XPM_AUR_FALLBACK_DIR: &str = "/var/lib/xpm-aur";

/// Repository names that publish native X packages. `x` is the native endpoint
/// used by `xpm`'s example configuration; extend this list if more native
/// repositories are added.
const X_NATIVE_REPOS: &[&str] = &["x"];

const VERSION_FILE: &str = "version";
const ORIGIN_FILE: &str = "origin";

#[derive(Debug, Default, Deserialize)]
struct AurState {
    #[serde(default)]
    packages: BTreeMap<String, AurRecord>,
}

#[derive(Debug, Default, Deserialize)]
struct AurRecord {
    #[serde(default)]
    packages: Vec<String>,
}

/// Returns the `xpm`, `xp` and `xpm-aur` counts that apply to this machine.
///
/// The list is empty when `xpm` has no local database. `xp` is always present
/// alongside `xpm` (it may be zero) so the native package count is explicit.
/// `xpm-aur` is only present when its state file can be read. Every count is
/// derived from locally installed packages; remote catalogs are never queried.
pub fn package_counts() -> Vec<(&'static str, usize)> {
    let db = Path::new(XPM_DB_DIR);
    let Some(installed) = installed_package_names(db) else {
        return Vec::new();
    };

    let mut counts = vec![(XPM_LABEL, installed.len())];

    let native = count_xp_packages(db, &installed, X_NATIVE_REPOS);
    counts.push((XP_LABEL, native));

    let state = xpm_aur_state_path();
    if let Some(count) = state
        .as_deref()
        .and_then(|path| count_xpm_aur_packages(path, &installed))
    {
        counts.push((XPM_AUR_LABEL, count));
    }

    counts
}

/// Returns the names of packages installed in the xpm local database.
///
/// A directory counts only when it contains a `version` file, matching what
/// `xpm query` reports, and dot-prefixed entries are ignored.
fn installed_package_names(db: &Path) -> Option<HashSet<String>> {
    let mut names = HashSet::new();

    for entry in std::fs::read_dir(db).ok()?.filter_map(Result::ok) {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        if is_dot_entry(&entry.file_name()) {
            continue;
        }
        if !entry.path().join(VERSION_FILE).is_file() {
            continue;
        }
        names.insert(entry.file_name().to_string_lossy().into_owned());
    }

    Some(names)
}

/// Counts installed packages whose recorded origin is a native X repository.
fn count_xp_packages(db: &Path, installed: &HashSet<String>, native_repos: &[&str]) -> usize {
    installed
        .iter()
        .filter(|name| {
            std::fs::read_to_string(db.join(name).join(ORIGIN_FILE))
                .ok()
                .is_some_and(|origin| native_repos.contains(&origin.trim()))
        })
        .count()
}

/// Counts the installed package outputs recorded by `xpm-aur`.
///
/// Records and split outputs that are no longer present in the xpm local
/// database are ignored, so stale sidecar entries do not inflate the count.
fn count_xpm_aur_packages(state_file: &Path, installed: &HashSet<String>) -> Option<usize> {
    let text = std::fs::read_to_string(state_file).ok()?;
    let state: AurState = serde_json::from_str(&text).ok()?;

    let mut count = 0;
    for (base, record) in &state.packages {
        if record.packages.is_empty() {
            if installed.contains(base) {
                count += 1;
            }
        } else {
            count += record
                .packages
                .iter()
                .filter(|name| installed.contains(*name))
                .count();
        }
    }

    Some(count)
}

fn xpm_aur_state_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("XPM_AUR_STATE") {
        return Some(PathBuf::from(path));
    }
    let base = dirs::state_dir()
        .or_else(|| dirs::home_dir().map(|home| home.join(".local/state")))
        .unwrap_or_else(|| PathBuf::from(XPM_AUR_FALLBACK_DIR));
    Some(base.join("xpm-aur/state.json"))
}

fn is_dot_entry(name: &std::ffi::OsStr) -> bool {
    name.to_str().is_some_and(|name| name.starts_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("xfetch_test_xpm_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn package(db: &Path, name: &str, origin: Option<&str>) {
        let dir = db.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(VERSION_FILE), "1.0-1").unwrap();
        if let Some(origin) = origin {
            std::fs::write(dir.join(ORIGIN_FILE), origin).unwrap();
        }
    }

    #[test]
    fn test_installed_package_names_skips_incomplete_entries() {
        let db = temp_dir("xpm_total");
        package(&db, "foo", Some("x"));
        package(&db, "bar", Some("core"));
        std::fs::create_dir_all(db.join("incomplete")).unwrap();
        std::fs::create_dir_all(db.join(".partial")).unwrap();
        std::fs::write(db.join(".partial").join(VERSION_FILE), "1.0").unwrap();
        std::fs::write(db.join("db.lck"), "").unwrap();

        let installed = installed_package_names(&db).unwrap();
        assert_eq!(installed.len(), 2);
        assert!(installed.contains("foo"));
        assert!(installed.contains("bar"));
        let _ = std::fs::remove_dir_all(&db);
    }

    #[test]
    fn test_count_xp_packages_filters_by_native_origin() {
        let db = temp_dir("xp_total");
        package(&db, "native-a", Some("x"));
        package(&db, "native-b", Some("x\n"));
        package(&db, "arch", Some("core"));
        package(&db, "local", Some("local"));
        package(&db, "no-origin", None);

        let installed = installed_package_names(&db).unwrap();
        assert_eq!(count_xp_packages(&db, &installed, &["x"]), 2);
        assert_eq!(count_xp_packages(&db, &installed, &["x", "x-extra"]), 2);
        let _ = std::fs::remove_dir_all(&db);
    }

    #[test]
    fn test_count_xpm_aur_packages_sums_outputs() {
        let dir = temp_dir("aur_state");
        let state = dir.join("state.json");
        std::fs::write(
            &state,
            r#"{
                "schema": 1,
                "packages": {
                    "yay": {
                        "base": "yay",
                        "version": "1.0",
                        "commit": "abc",
                        "installed_at": 1,
                        "explicit": true,
                        "packages": ["yay", "yay-debug"]
                    },
                    "old": {
                        "base": "old",
                        "version": "2.0",
                        "commit": "def",
                        "installed_at": 2,
                        "explicit": false,
                        "packages": []
                    },
                    "stale": {
                        "base": "stale",
                        "version": "3.0",
                        "commit": "ghi",
                        "installed_at": 3,
                        "explicit": false,
                        "packages": ["stale-pkg"]
                    }
                }
            }"#,
        )
        .unwrap();

        let installed: HashSet<String> = ["yay", "yay-debug", "old"]
            .into_iter()
            .map(String::from)
            .collect();
        assert_eq!(count_xpm_aur_packages(&state, &installed), Some(3));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_count_xpm_aur_packages_rejects_invalid_state() {
        let dir = temp_dir("aur_invalid");
        let state = dir.join("state.json");
        std::fs::write(&state, "not json").unwrap();
        let installed = HashSet::new();
        assert_eq!(count_xpm_aur_packages(&state, &installed), None);
        assert_eq!(
            count_xpm_aur_packages(&dir.join("missing.json"), &installed),
            None
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
