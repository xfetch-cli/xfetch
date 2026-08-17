use std::time::Duration;

use crate::info::platform::shared::{NA, commands::run_cmd_with_timeout};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const PMSET_CMD: &str = "pmset";

pub fn get_battery_info() -> String {
    let Some(output) = run_cmd_with_timeout(PMSET_CMD, &["-g", "batt"], CMD_TIMEOUT) else {
        return NA.to_string();
    };
    let out = String::from_utf8_lossy(&output.stdout);
    let Some(line) = out.lines().nth(1) else {
        return NA.to_string();
    };
    let Some(pct) = line
        .split_whitespace()
        .find(|p| p.contains('%'))
        .map(|p| p.trim_end_matches(';').to_string())
    else {
        return NA.to_string();
    };
    let status = if line.contains("discharging") {
        "Discharging"
    } else if line.contains("charging") {
        "Charging"
    } else if line.contains("charged") {
        "Charged"
    } else {
        "Unknown"
    };
    format!("{} [{}]", pct, status)
}
