use std::collections::HashMap;
use std::mem::zeroed;
use std::time::Duration;

use windows_sys::Win32::Graphics::Gdi::{DISPLAY_DEVICEW, EnumDisplayDevicesW};

use crate::info::platform::shared::{
    UNKNOWN_GPU, commands::run_cmd_with_timeout, gpu::fields_from_name,
};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const POWERSHELL_CMD: &str = "powershell";
/// `-NoProfile` keeps the user's profile script out of the probe output;
/// `-NonInteractive` forbids prompts. `[Console]::OutputEncoding=UTF8` fixes
/// the OEM codepage PowerShell 5.1 uses for redirected output.
const GPU_PS_SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";

/// GPU names via `EnumDisplayDevicesW` (~1 ms). The enumeration repeats the
/// adapter name for every monitor entry, hence the dedupe; `wmic` and the
/// PowerShell/CIM fallback cost hundreds of milliseconds and `wmic` is absent
/// from Windows 11 24H2+.
pub fn get_gpu_info() -> Vec<String> {
    let mut gpus = display_adapters();
    if gpus.is_empty() {
        gpus = powershell_gpus();
    }
    if gpus.is_empty() {
        vec![UNKNOWN_GPU.to_string()]
    } else {
        gpus
    }
}

fn display_adapters() -> Vec<String> {
    let mut gpus = Vec::new();
    let mut index = 0u32;
    loop {
        let mut device: DISPLAY_DEVICEW = unsafe { zeroed() };
        device.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
        let found = unsafe { EnumDisplayDevicesW(std::ptr::null(), index, &mut device, 0) };
        if found == 0 {
            break;
        }
        index += 1;
        let name = wide_to_string(&device.DeviceString);
        if !name.is_empty() && !gpus.contains(&name) {
            gpus.push(name);
        }
    }
    gpus
}

fn wide_to_string(buf: &[u16]) -> String {
    let len = buf.iter().position(|c| *c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..len]).trim().to_string()
}

fn powershell_gpus() -> Vec<String> {
    run_cmd_with_timeout(
        POWERSHELL_CMD,
        &["-NoProfile", "-NonInteractive", "-Command", GPU_PS_SCRIPT],
        CMD_TIMEOUT,
    )
    .filter(|o| o.status.success())
    .map(|o| parse_gpu_names(&String::from_utf8_lossy(&o.stdout)))
    .unwrap_or_default()
}

fn parse_gpu_names(output: &str) -> Vec<String> {
    let mut gpus = Vec::new();
    for line in output.lines() {
        let name = line.trim().trim_matches('\0');
        if !name.is_empty() && !gpus.contains(&name.to_string()) {
            gpus.push(name.to_string());
        }
    }
    gpus
}

/// Structured fields for a stored GPU line: the CIM/`DISPLAY_DEVICE` values
/// are plain device names (`"NVIDIA GeForce GTX 1060 6GB"`).
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
    fn test_parse_gpu_names_keeps_first_and_dedupes() {
        let out =
            "NVIDIA GeForce RTX 4070\r\nNVIDIA GeForce RTX 4070\r\nIntel UHD Graphics 770\r\n\r\n";
        let gpus = parse_gpu_names(out);
        assert_eq!(
            gpus,
            vec!["NVIDIA GeForce RTX 4070", "Intel UHD Graphics 770"]
        );
    }

    #[test]
    fn test_display_adapters_are_deduped() {
        let gpus = display_adapters();
        let mut unique = gpus.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), gpus.len());
    }
}
