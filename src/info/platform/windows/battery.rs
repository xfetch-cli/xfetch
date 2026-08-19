use std::time::Duration;

use crate::info::platform::shared::{NA, commands::run_cmd_with_timeout};

const CMD_TIMEOUT: Duration = Duration::from_secs(10);

const WMIC_CMD: &str = "wmic";
const POWERSHELL_CMD: &str = "powershell";
/// `-NoProfile` keeps the user's profile script out of the probe output;
/// `-NonInteractive` forbids prompts. `[Console]::OutputEncoding=UTF8` fixes
/// the OEM codepage PowerShell 5.1 uses for redirected output.
const BATT_PS_SCRIPT: &str = "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-CimInstance Win32_Battery | Select-Object EstimatedChargeRemaining, BatteryStatus";

pub fn get_battery_info() -> String {
    let output = run_cmd_with_timeout(
        WMIC_CMD,
        &[
            "path",
            "Win32_Battery",
            "Get",
            "EstimatedChargeRemaining,BatteryStatus",
        ],
        CMD_TIMEOUT,
    )
    .filter(|o| o.status.success())
    .or_else(|| {
        run_cmd_with_timeout(
            POWERSHELL_CMD,
            &["-NoProfile", "-NonInteractive", "-Command", BATT_PS_SCRIPT],
            CMD_TIMEOUT,
        )
    });

    let Some(output) = output else {
        return NA.to_string();
    };
    let out = String::from_utf8_lossy(&output.stdout);
    for line in out.lines().skip(1) {
        let trimmed = line.trim().trim_matches('\0');
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.split_whitespace().collect();
        if cols.len() >= 2
            && let Ok(pct) = cols[0].parse::<u32>()
        {
            let status = match cols.get(1).and_then(|s| s.parse::<u32>().ok()) {
                Some(1) => "Discharging",
                Some(2) | Some(3) => "Charged",
                Some(6) | Some(7) | Some(8) | Some(9) => "Charging",
                _ => "Unknown",
            };
            return format!("{}% [{}]", pct, status);
        }
    }
    NA.to_string()
}
