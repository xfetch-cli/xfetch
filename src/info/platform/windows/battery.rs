use std::mem::zeroed;

use windows_sys::Win32::System::Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS};

use crate::info::platform::shared::NA;

const AC_OFFLINE: u8 = 0;
const AC_ONLINE: u8 = 1;
const BATTERY_FLAG_CHARGING: u8 = 8;
const BATTERY_FLAG_NO_BATTERY: u8 = 128;
const BATTERY_PERCENT_UNKNOWN: u8 = 255;

/// Battery status read straight from the Win32 API. `GetSystemPowerStatus`
/// returns the same percentage/state the previous `wmic`/PowerShell probes
/// reported, without a subprocess (PowerShell alone cost ~280 ms per fetch).
pub fn get_battery_info() -> String {
    let mut status: SYSTEM_POWER_STATUS = unsafe { zeroed() };
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return NA.to_string();
    }
    if status.BatteryFlag & BATTERY_FLAG_NO_BATTERY != 0
        || status.BatteryLifePercent == BATTERY_PERCENT_UNKNOWN
    {
        return NA.to_string();
    }
    let label = battery_label(
        status.ACLineStatus,
        status.BatteryFlag,
        status.BatteryLifePercent,
    );
    format!("{}% [{}]", status.BatteryLifePercent, label)
}

/// Maps `SYSTEM_POWER_STATUS` to the labels the `Win32_Battery` probe used:
/// `Discharging` (1), `Charged` (2/3) and `Charging` (6-9).
fn battery_label(ac_line: u8, flags: u8, percent: u8) -> &'static str {
    if flags & BATTERY_FLAG_CHARGING != 0 {
        return "Charging";
    }
    match ac_line {
        AC_OFFLINE => "Discharging",
        AC_ONLINE if percent == 100 => "Charged",
        AC_ONLINE => "Charging",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_battery_label_mapping() {
        assert_eq!(battery_label(AC_OFFLINE, 0, 42), "Discharging");
        assert_eq!(battery_label(AC_ONLINE, 0, 100), "Charged");
        assert_eq!(
            battery_label(AC_ONLINE, BATTERY_FLAG_CHARGING, 80),
            "Charging"
        );
        assert_eq!(battery_label(AC_ONLINE, 0, 80), "Charging");
        assert_eq!(battery_label(255, 0, 80), "Unknown");
    }

    #[test]
    fn test_get_battery_info_format() {
        let battery = get_battery_info();
        assert!(battery.contains('%') || battery == NA);
    }
}
