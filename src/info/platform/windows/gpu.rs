use std::collections::HashMap;
use std::time::Duration;

use crate::info::platform::shared::{
    UNKNOWN_GPU, commands::run_cmd_with_timeout, gpu::fields_from_name,
};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const WMIC_CMD: &str = "wmic";
const POWERSHELL_CMD: &str = "powershell";
/// `-NoProfile` keeps the user's profile script out of the probe output;
/// `-NonInteractive` forbids prompts. `[Console]::OutputEncoding=UTF8` fixes
/// the OEM codepage PowerShell 5.1 uses for redirected output.
const GPU_PS_SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";

pub fn get_gpu_info() -> Vec<String> {
    // `wmic` prints a `Name` header; the PowerShell fallback expands the
    // property and has none, so only the WMIC output skips its first line.
    let gpu_output = run_cmd_with_timeout(
        WMIC_CMD,
        &["path", "win32_videocontroller", "get", "name"],
        CMD_TIMEOUT,
    )
    .filter(|o| o.status.success())
    .map(|o| (o, true))
    .or_else(|| {
        run_cmd_with_timeout(
            POWERSHELL_CMD,
            &["-NoProfile", "-NonInteractive", "-Command", GPU_PS_SCRIPT],
            CMD_TIMEOUT,
        )
        .filter(|o| o.status.success())
        .map(|o| (o, false))
    });

    let mut gpus = Vec::new();
    if let Some((output, skip_header)) = gpu_output {
        let out = String::from_utf8_lossy(&output.stdout);
        gpus = parse_gpu_names(&out, skip_header);
    }
    if gpus.is_empty() {
        vec![UNKNOWN_GPU.to_string()]
    } else {
        gpus
    }
}

fn parse_gpu_names(output: &str, skip_header: bool) -> Vec<String> {
    output
        .lines()
        .skip(usize::from(skip_header))
        .map(|line| line.trim().trim_matches('\0'))
        .filter(|line| !line.is_empty())
        .map(str::to_string)
        .collect()
}

/// Structured fields for a stored GPU line: the `Name`/CIM values are plain
/// device names (`"NVIDIA GeForce GTX 1060 6GB"`).
pub fn gpu_fields(line: &str) -> HashMap<String, String> {
    fields_from_name(line)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_fields_wmic_name() {
        let f = gpu_fields("NVIDIA GeForce GTX 1060 6GB");
        assert_eq!(f.get("name").unwrap(), "NVIDIA GeForce GTX 1060 6GB");
        assert_eq!(f.get("vendor").unwrap(), "NVIDIA");
        assert_eq!(f.get("model").unwrap(), "GTX 1060");
        assert_eq!(f.get("vram").unwrap(), "6GB");
    }

    #[test]
    fn test_gpu_fields_amd() {
        let f = gpu_fields("AMD Radeon RX 6800 16GB");
        assert_eq!(f.get("vendor").unwrap(), "AMD");
        assert_eq!(f.get("model").unwrap(), "RX 6800");
    }

    #[test]
    fn test_parse_gpu_names_wmic_skips_header() {
        let out = "Name\r\nNVIDIA GeForce RTX 4070\r\nIntel UHD Graphics 770\r\n";
        let gpus = parse_gpu_names(out, true);
        assert_eq!(
            gpus,
            vec!["NVIDIA GeForce RTX 4070", "Intel UHD Graphics 770"]
        );
    }

    #[test]
    fn test_parse_gpu_names_powershell_keeps_first() {
        let out = "NVIDIA GeForce RTX 4070\r\nIntel UHD Graphics 770\r\n";
        let gpus = parse_gpu_names(out, false);
        assert_eq!(
            gpus,
            vec!["NVIDIA GeForce RTX 4070", "Intel UHD Graphics 770"]
        );
    }
}
