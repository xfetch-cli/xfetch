use crate::info::platform::shared::NA;

const BATT_DIR: &str = "/sys/class/power_supply";
const BATT_CAPACITY: &str = "capacity";
const BATT_STATUS: &str = "status";
const BATT_PREFIXES: [&str; 6] = [
    "BAT",
    "bat",
    "hidpp_battery",
    "ucsi_battery",
    "C22C",
    "ps-battery",
];

fn is_battery_dir(name: &str) -> bool {
    BATT_PREFIXES.iter().any(|p| name.starts_with(p))
}

pub fn get_battery_info() -> String {
    let batt_dir = std::path::Path::new(BATT_DIR);
    let Ok(entries) = std::fs::read_dir(batt_dir) else {
        return NA.to_string();
    };
    let mut total_pct = 0u32;
    let mut batt_count = 0u32;
    let mut statuses: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !is_battery_dir(&name) {
            continue;
        }
        let base = entry.path();
        if let Ok(cap) = std::fs::read_to_string(base.join(BATT_CAPACITY))
            && let Ok(pct) = cap.trim().parse::<u32>()
        {
            total_pct += pct;
            batt_count += 1;
        }
        if let Ok(s) = std::fs::read_to_string(base.join(BATT_STATUS)) {
            let s = s.trim().to_string();
            if !statuses.contains(&s) {
                statuses.push(s);
            }
        }
    }
    if let Some(avg) = total_pct.checked_div(batt_count) {
        let status = if statuses.is_empty() {
            crate::info::unknown()
        } else {
            statuses.join("+")
        };
        format!("{}% [{}]", avg, status)
    } else {
        NA.to_string()
    }
}
