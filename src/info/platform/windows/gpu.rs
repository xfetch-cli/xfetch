use std::time::Duration;

use crate::info::platform::shared::{UNKNOWN_GPU, commands::run_cmd_with_timeout};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const WMIC_CMD: &str = "wmic";
const POWERSHELL_CMD: &str = "powershell";
const GPU_PS_SCRIPT: &str =
    "Get-CimInstance Win32_VideoController | Select-Object -ExpandProperty Name";

pub fn get_gpu_info() -> Vec<String> {
    let mut gpus = Vec::new();
    let gpu_output = run_cmd_with_timeout(
        WMIC_CMD,
        &["path", "win32_videocontroller", "get", "name"],
        CMD_TIMEOUT,
    )
    .or_else(|| run_cmd_with_timeout(POWERSHELL_CMD, &["-Command", GPU_PS_SCRIPT], CMD_TIMEOUT));
    if let Some(output) = gpu_output {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines().skip(1) {
            let trimmed = line.trim();
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
