use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const CACHE_DIR_NAME: &str = "xfetch";
const CACHE_FILE: &str = "cache.json";

#[derive(Debug, Serialize, Deserialize)]
struct CacheEntry {
    data: String,
    cached_at: u64,
}

fn cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| {
            dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
        })
        .join(CACHE_DIR_NAME)
}

fn cache_path() -> PathBuf {
    cache_dir().join(CACHE_FILE)
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn load_entries() -> HashMap<String, CacheEntry> {
    let path = cache_path();
    if path.exists() {
        fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_entries(entries: &HashMap<String, CacheEntry>) {
    let path = cache_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string(entries) {
        let _ = fs::write(&path, json);
    }
}

pub fn get(key: &str, max_age: Duration) -> Option<String> {
    let entries = load_entries();
    let entry = entries.get(key)?;
    let age_secs = now_epoch().saturating_sub(entry.cached_at);
    if Duration::from_secs(age_secs) >= max_age {
        None
    } else {
        Some(entry.data.clone())
    }
}

pub fn set(key: &str, data: &str) {
    let mut entries = load_entries();
    entries.insert(
        key.to_string(),
        CacheEntry {
            data: data.to_string(),
            cached_at: now_epoch(),
        },
    );
    save_entries(&entries);
}

pub fn clean() -> std::io::Result<()> {
    let path = cache_path();
    if path.exists() {
        fs::remove_file(path)?;
    }
    let _ = fs::remove_dir(cache_dir());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static MTX: Mutex<()> = Mutex::new(());
    fn lock() -> std::sync::MutexGuard<'static, ()> {
        MTX.lock().unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn test_cache_set_get() {
        let _l = lock();
        let _ = clean();
        set("tsg_key", "tsg_value");
        let result = get("tsg_key", Duration::from_secs(60));
        assert_eq!(result, Some("tsg_value".to_string()));
        let _ = clean();
    }

    #[test]
    fn test_cache_expired() {
        let _l = lock();
        let _ = clean();
        set("te_key", "te_value");
        let result = get("te_key", Duration::from_secs(0));
        assert_eq!(result, None);
        let _ = clean();
    }

    #[test]
    fn test_cache_missing() {
        let _l = lock();
        let _ = clean();
        let result = get("missing_key", Duration::from_secs(60));
        assert_eq!(result, None);
    }

    #[test]
    fn test_clean() {
        let _l = lock();
        let _ = clean();
        set("tc_key", "tc_value");
        assert!(clean().is_ok());
        let result = get("tc_key", Duration::from_secs(60));
        assert_eq!(result, None);
    }
}
