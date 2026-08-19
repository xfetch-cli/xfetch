//! `/etc/os-release` parsing used by the logo catalog resolution
//! (`crate::logos`), Linux-only.

/// Parses `ID` and `ID_LIKE` out of an os-release file.
pub fn parse_os_release(content: &str) -> (String, Vec<String>) {
    let mut id = "linux".to_string();
    let mut id_like = Vec::new();
    for line in content.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key {
            "ID" if !value.is_empty() => id = value.to_lowercase(),
            "ID_LIKE" => id_like = value.split_whitespace().map(|s| s.to_lowercase()).collect(),
            _ => {}
        }
    }
    (id, id_like)
}

/// `(ID, ID_LIKE)` for the running Linux distro, read from `/etc/os-release`
/// (lowercased). Falls back to `("linux", [])` when the file is missing.
pub fn detect_os_ids() -> (String, Vec<String>) {
    parse_os_release(&std::fs::read_to_string("/etc/os-release").unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_os_release() {
        let (id, like) =
            parse_os_release("NAME=\"Ubuntu\"\nID=ubuntu\nID_LIKE=debian\nVERSION_ID=\"24.04\"\n");
        assert_eq!(id, "ubuntu");
        assert_eq!(like, vec!["debian"]);

        let (id, like) = parse_os_release("ID=\"Linux Mint\"\nID_LIKE=\"ubuntu debian\"\n");
        assert_eq!(id, "linux mint");
        assert_eq!(like, vec!["ubuntu", "debian"]);

        let (id, _) = parse_os_release("");
        assert_eq!(id, "linux");
    }

    #[test]
    fn test_detect_on_current_system() {
        let (id, like) = detect_os_ids();
        assert!(!id.is_empty());
        assert!(like.iter().all(|l| !l.is_empty()));
    }
}
