use std::time::Duration;

use crate::info::platform::shared::{UNKNOWN_GPU, commands::run_cmd_with_timeout};

/// `system_profiler` is notoriously slow, so it gets a much longer timeout
/// than the other probes.
const SYSTEM_PROFILER_TIMEOUT: Duration = Duration::from_secs(30);

const SYSTEM_PROFILER_CMD: &str = "system_profiler";
const CHIPSET_MODEL: &str = "Chipset Model:";

pub fn get_gpu_info() -> Vec<String> {
    let mut gpus = Vec::new();
    if let Some(output) = run_cmd_with_timeout(
        SYSTEM_PROFILER_CMD,
        &["SPDisplaysDataType"],
        SYSTEM_PROFILER_TIMEOUT,
    ) {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines() {
            if line.trim().starts_with(CHIPSET_MODEL) {
                gpus.push(line.trim().replace(CHIPSET_MODEL, "").trim().to_string());
            }
        }
    }
    if gpus.is_empty() {
        vec![UNKNOWN_GPU.to_string()]
    } else {
        gpus
    }
}
