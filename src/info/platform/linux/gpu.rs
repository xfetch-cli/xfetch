use std::collections::HashMap;
use std::time::Duration;

use crate::info::platform::shared::{
    UNKNOWN_GPU,
    commands::run_cmd_with_timeout,
    gpu::{bracket_content, fields_from_name},
};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const LSPCI_CMD: &str = "lspci";

const GPU_CLASS_VGA: &str = "VGA";
const GPU_CLASS_3D: &str = "3D";
const GPU_CLASS_DISPLAY: &str = "Display";

pub fn get_gpu_info() -> Vec<String> {
    let mut gpus = Vec::new();
    if let Some(output) = run_cmd_with_timeout(LSPCI_CMD, &["-mm"], CMD_TIMEOUT) {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines() {
            if line.contains(GPU_CLASS_VGA)
                || line.contains(GPU_CLASS_3D)
                || line.contains(GPU_CLASS_DISPLAY)
            {
                let parts: Vec<&str> = line.split('"').collect();
                if parts.len() > 5 {
                    gpus.push(parts[5].to_string());
                }
            }
        }
    }
    if gpus.is_empty() {
        vec![UNKNOWN_GPU.to_string()]
    } else {
        gpus
    }
}

/// Structured fields for a stored GPU line: `lspci` device descriptions
/// wrap the readable name in brackets (`"GP106 [GeForce GTX 1060 6GB]"`),
/// so `{name}` uses the bracketed part and the rest of the fields are
/// derived from it.
pub fn gpu_fields(line: &str) -> HashMap<String, String> {
    let name = bracket_content(line)
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| line.trim().to_string());
    fields_from_name(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_fields_lspci_bracket() {
        let f = gpu_fields("GP106 [GeForce GTX 1060 6GB]");
        assert_eq!(f.get("name").unwrap(), "GeForce GTX 1060 6GB");
        assert_eq!(f.get("vendor").unwrap(), "NVIDIA");
        assert_eq!(f.get("model").unwrap(), "GTX 1060");
        assert_eq!(f.get("vram").unwrap(), "6GB");
    }

    #[test]
    fn test_gpu_fields_plain_line() {
        let f = gpu_fields("NVIDIA GeForce GTX 1060 6GB");
        assert_eq!(f.get("vendor").unwrap(), "NVIDIA");
        assert_eq!(f.get("model").unwrap(), "GTX 1060");
    }

    #[test]
    fn test_gpu_fields_unknown() {
        let f = gpu_fields("mystery hardware");
        assert_eq!(f.get("name").unwrap(), "mystery hardware");
        assert!(!f.contains_key("vendor"));
    }
}
