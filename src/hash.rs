//! SHA-256 helpers shared by the self-updater and the wasm installers.
//!
//! Both update verification and wasm artifact provenance use the same
//! lowercase-hex encoding, so the encoding and validation live here instead of
//! being duplicated in two feature-gated modules.

use sha2::{Digest, Sha256};

/// Lowercase hex SHA-256 of a byte slice.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{:02x}", byte));
    }
    out
}

/// Validates a user- or manifest-supplied digest and returns it lowercase.
///
/// Surrounding whitespace is ignored and upper/lowercase hex is accepted; a
/// value that is not exactly 64 hex characters is rejected so a truncated
/// checksum can never silently compare against a prefix.
pub fn normalize_sha256(value: &str) -> Result<String, String> {
    let digest = value.trim();
    if digest.len() != 64 {
        return Err(format!(
            "expected a 64-character SHA-256 hex digest, got {} character(s)",
            digest.len()
        ));
    }
    if !digest.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("SHA-256 digest contains non-hexadecimal characters".to_string());
    }
    Ok(digest.to_ascii_lowercase())
}

/// Verifies `bytes` against `expected`, returning a descriptive error on a
/// mismatch or malformed digest.
pub fn verify_sha256(bytes: &[u8], expected: &str) -> Result<(), String> {
    let expected =
        normalize_sha256(expected).map_err(|err| format!("Invalid SHA-256 digest: {}", err))?;
    let actual = sha256_hex(bytes);
    if actual != expected {
        return Err(format!(
            "SHA-256 mismatch: expected {}, got {}",
            expected, actual
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const ABC: &str = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";

    #[test]
    fn sha256_matches_known_vector() {
        assert_eq!(sha256_hex(b"abc"), ABC);
    }

    #[test]
    fn normalize_accepts_uppercase_and_whitespace() {
        assert_eq!(
            normalize_sha256(&format!("  {}  ", ABC.to_ascii_uppercase())).expect("normalize"),
            ABC
        );
    }

    #[test]
    fn normalize_rejects_wrong_length_and_non_hex() {
        assert!(normalize_sha256("abc").is_err());
        assert!(normalize_sha256(&"z".repeat(64)).is_err());
        // Odd lengths must not be accepted even when the characters are hex.
        assert!(normalize_sha256(&"a".repeat(63)).is_err());
    }

    #[test]
    fn verify_matches_expected_digest() {
        assert!(verify_sha256(b"abc", ABC).is_ok());
        assert!(verify_sha256(b"abc", &ABC.to_ascii_uppercase()).is_ok());
    }

    #[test]
    fn verify_reports_mismatch_with_both_digests() {
        let other = "0".repeat(64);
        let err = verify_sha256(b"abc", &other).expect_err("mismatch");
        assert!(err.contains(&other));
        assert!(err.contains(ABC));
    }
}
