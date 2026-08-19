use std::time::Duration;

use crate::info::platform::shared::{UNKNOWN_GPU, commands::run_cmd_with_timeout};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const WMIC_CMD: &str = "wmic";
const POWERSHELL_CMD: &str = "powershell";
/// `-NoProfile` keeps the user's profile script out of the probe output;
/// `-NonInteractive` forbids prompts. `[Console]::OutputEncoding=UTF8` fixes
/// the OEM codepage PowerShell 5.1 uses for redirected output.
const GPU_PS_SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";

pub fn get_gpu_info() -> Vec<String> {
    let mut gpus = Vec::new();
    let gpu_output = run_cmd_with_timeout(
        WMIC_CMD,
        &["path", "win32_videocontroller", "get", "name"],
        CMD_TIMEOUT,
    )
    .filter(|o| o.status.success())
    .or_else(|| {
        run_cmd_with_timeout(
            POWERSHELL_CMD,
            &["-NoProfile", "-NonInteractive", "-Command", GPU_PS_SCRIPT],
            CMD_TIMEOUT,
        )
    });
    if let Some(output) = gpu_output.filter(|o| o.status.success()) {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines().skip(1) {
            let trimmed = line.trim().trim_matches('\0');
            if !trimmed.is_empty() {
                gpus.push(trimmed.to_string());
            }
        }
    }
    if gpus.is_empty() {
        vec![UNKNOWN_GPU.to_string()]
    } else {
        gpus
    }
}
