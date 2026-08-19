use std::path::PathBuf;

pub fn expand_path(path: &str) -> PathBuf {
    if path == "~" {
        return dirs::home_dir().unwrap_or_else(|| PathBuf::from(path));
    }
    if path.starts_with("~")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(&path[2..]);
    }
    PathBuf::from(path)
}

pub fn get_default_ascii() -> String {
    r#"
__  __
  \ \/ /
   \  /
   /  \
  /_/\_\
 /____/linux
---------BEGIN PUBLIC KEY----------
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMII
BCgKCAQEAwtU/XOS/xOf/FakeKeyDataFor
ArtPutHere+Of/XOSLINUXDISTRO/gD4t4+
N4ToR3aL/K3yG3NkR/v3RyL0ngD4t4+XoSy
LiNuX/R0cK/M0r3+L1nUx/D1sTr0/R4cK3a
XYS/aBcDeFgHiJkLmNoPqRsTuVwXyZ01234
56789+/woXosLinuxR0xOsD1sTr0d1stR0f
0/c00L/k3Y/wER3aL63nD/v3RyL0ngD4t4a
ABcD/EfGhIjK/xOsLiNuX/D1sTr0/R0cK3a
LmNoPqRsTuVwXyZ0/1nUx/wER3aL63nD/v3
RyL0ngD4t4+XoSy/M0r3+L1nUx/D1sTr0/R
4cK3a/XYS/aBcDeFgHiJkLmNoPqRsTuVwXy
Z0123456789+/woXos/Linux/Rocks/==
----------END PUBLIC KEY-----------
"#
    .trim()
    .to_string()
}

//tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    //verify that the expand_path function correctly expands the tilde to the home directory
    fn test_expand_path_home() {
        let path = expand_path("~/tests");
        let path_str = path.to_string_lossy();
        // Verify that the tilde was expanded correctly to an absolute path
        assert!(!path_str.starts_with('~'));
        assert!(path.is_absolute());
    }

    #[test]
    //verify that the expand_path function returns the same path if it does not start with a tilde
    fn test_security_path_traversal_and_null_byte() {
        // Cybersec: Injection test for nulled bytes and path traversal
        // Rust is memory safe, but we validate that the function returns the path
        // without panicking or corrupting it when receiving malicious input.
        let malicius_input = "../../../etc/passwd\0malicious_file.png";
        let path = expand_path(malicius_input);
        let path_str = path.to_string_lossy();

        assert!(path_str.contains("../../../etc/passwd"));
        assert!(path_str.contains('\0'));
    }

    #[test]
    //verify that the default ASCII logo is generated correctly
    fn test_get_default_ascii_not_empty() {
        let ascii = get_default_ascii();
        assert!(!ascii.is_empty());
        assert!(ascii.contains("linux"));
    }
    //verify that a normal path is returned unchanged
    #[test]
    fn test_expand_path_normal() {
        let path = expand_path("/usr/bin/custom_logo.png");
        assert_eq!(path.to_string_lossy(), "/usr/bin/custom_logo.png");
    }
}
