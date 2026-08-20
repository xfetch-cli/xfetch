//! Shared GPU field extraction: the *rules* used by every platform's
//! `gpu_fields` to split a raw GPU line into structured fields. Each OS
//! folder keeps its own extractor (it knows the shape of its probe output);
//! the shared pieces avoid duplicating vendor/VRAM heuristics.

use std::collections::HashMap;

/// Content of the last `[...]` group in a line (lspci device descriptions
/// like `"GP106 [GeForce GTX 1060 6GB]"`), when present.
pub fn bracket_content(line: &str) -> Option<&str> {
    let start = line.find('[')?;
    let end = line[start + 1..].find(']')? + start + 1;
    Some(&line[start + 1..end])
}

/// Trailing VRAM token of a GPU name (`"6GB"`, `"12 GB"`, `"4GiB"`), when
/// present.
pub fn trailing_vram(name: &str) -> Option<String> {
    let last = name.split_whitespace().last()?;
    let upper = last.to_ascii_uppercase();
    let digits = upper.trim_end_matches(|c: char| c.is_ascii_alphabetic());
    if (upper.ends_with("GB") || upper.ends_with("GIB") || upper.ends_with("MB"))
        && !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit() || c == '.')
    {
        Some(last.to_string())
    } else {
        None
    }
}

/// Vendor detected from a GPU name: `"NVIDIA"`, `"AMD"`, `"Intel"`,
/// `"Apple"`, `"Qualcomm"`, `"Microsoft"`, `"VMware"` or `"Mesa"`.
/// Matches either the vendor word itself or its well-known product lines
/// (e.g. `GeForce` → NVIDIA, `Radeon` → AMD).
pub fn detect_vendor(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    for (needles, vendor) in [
        (&["nvidia", "geforce", "gtx", "rtx"][..], "NVIDIA"),
        (&["radeon", "amd", "ati", "rx "][..], "AMD"),
        (
            &["intel", "iris", "uhd graphics", "hd graphics"][..],
            "Intel",
        ),
        (&["apple", "m1 ", "m2 ", "m3 ", "m4 "][..], "Apple"),
        (&["qualcomm", "adreno"][..], "Qualcomm"),
        (&["microsoft", "hyper-v", "basic display"][..], "Microsoft"),
        (&["vmware"][..], "VMware"),
        (&["llvmpipe", "mesa"][..], "Mesa"),
    ] {
        if needles.iter().any(|n| lower.contains(n)) {
            return Some(vendor);
        }
    }
    None
}

/// Best-effort model name: the device name without its vendor word, common
/// product-line prefixes (`GeForce`, `Radeon`, ...) and the trailing VRAM.
/// May equal the name when nothing matches — never empty when `name` is not.
pub fn model_from_name(name: &str, vendor: Option<&str>) -> String {
    let mut m = name.trim().to_string();
    if let Some(v) = vendor {
        for word in v.split_whitespace() {
            let lower = m.to_lowercase();
            if (lower == word.to_lowercase()
                || lower.starts_with(&format!("{} ", word.to_lowercase())))
                && let Some(stripped) = m.split_once(' ').map(|(_, rest)| rest)
            {
                m = stripped.to_string();
            }
        }
    }
    for prefix in [
        "GeForce ",
        "Geforce ",
        "Radeon ",
        "Iris ",
        "UHD Graphics ",
        "HD Graphics ",
    ] {
        if let Some(stripped) = m.strip_prefix(prefix) {
            m = stripped.to_string();
            break;
        }
    }
    if let Some(vram) = trailing_vram(&m)
        && let Some(stripped) = m.trim_end().strip_suffix(&vram)
    {
        m = stripped.trim().to_string();
    }
    m
}

/// Builds the standard GPU field set from a device name: `{name}`, plus
/// `{vendor}`, `{model}` and `{vram}` when detectable.
pub fn fields_from_name(name: &str) -> HashMap<String, String> {
    let mut fields = HashMap::new();
    fields.insert("name".to_string(), name.trim().to_string());
    if let Some(vram) = trailing_vram(name) {
        fields.insert("vram".to_string(), vram);
    }
    let vendor = detect_vendor(name);
    if let Some(vendor) = vendor {
        fields.insert("vendor".to_string(), vendor.to_string());
        let model = model_from_name(name, Some(vendor));
        if !model.is_empty() && model != name.trim() {
            fields.insert("model".to_string(), model);
        }
    }
    fields
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bracket_content() {
        assert_eq!(
            bracket_content("GP106 [GeForce GTX 1060 6GB]"),
            Some("GeForce GTX 1060 6GB")
        );
        assert_eq!(bracket_content("No brackets"), None);
    }

    #[test]
    fn test_trailing_vram() {
        assert_eq!(
            trailing_vram("GeForce GTX 1060 6GB"),
            Some("6GB".to_string())
        );
        assert_eq!(
            trailing_vram("Radeon RX 6800 16GB"),
            Some("16GB".to_string())
        );
        assert_eq!(trailing_vram("RTX 4090"), None);
        assert_eq!(trailing_vram("Apple M2 Pro"), None);
    }

    #[test]
    fn test_detect_vendor() {
        assert_eq!(detect_vendor("GeForce GTX 1060 6GB"), Some("NVIDIA"));
        assert_eq!(detect_vendor("NVIDIA GeForce GTX 1060"), Some("NVIDIA"));
        assert_eq!(detect_vendor("Radeon RX 6800"), Some("AMD"));
        assert_eq!(detect_vendor("Intel UHD Graphics 630"), Some("Intel"));
        assert_eq!(detect_vendor("Apple M2 Pro"), Some("Apple"));
        assert_eq!(detect_vendor("Qualcomm Adreno 660"), Some("Qualcomm"));
        assert_eq!(detect_vendor("mystery gpu"), None);
    }

    #[test]
    fn test_model_from_name() {
        assert_eq!(
            model_from_name("GeForce GTX 1060 6GB", Some("NVIDIA")),
            "GTX 1060"
        );
        assert_eq!(
            model_from_name("NVIDIA GeForce GTX 1060", Some("NVIDIA")),
            "GTX 1060"
        );
        assert_eq!(model_from_name("Apple M2 Pro", Some("Apple")), "M2 Pro");
        assert_eq!(
            model_from_name("Radeon RX 6800 16GB", Some("AMD")),
            "RX 6800"
        );
    }

    #[test]
    fn test_fields_from_name() {
        let f = fields_from_name("NVIDIA GeForce GTX 1060 6GB");
        assert_eq!(f.get("name").unwrap(), "NVIDIA GeForce GTX 1060 6GB");
        assert_eq!(f.get("vendor").unwrap(), "NVIDIA");
        assert_eq!(f.get("model").unwrap(), "GTX 1060");
        assert_eq!(f.get("vram").unwrap(), "6GB");

        let f = fields_from_name("GP106 [GeForce GTX 1060 6GB]");
        assert_eq!(f.get("vendor").unwrap(), "NVIDIA");

        let f = fields_from_name("unknown device");
        assert_eq!(f.get("name").unwrap(), "unknown device");
        assert!(!f.contains_key("vendor"));
    }
}
