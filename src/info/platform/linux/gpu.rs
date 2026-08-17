use std::time::Duration;

use crate::info::platform::shared::{UNKNOWN_GPU, commands::run_cmd_with_timeout};

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
