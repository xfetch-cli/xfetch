use sysinfo::{Networks, System};
use std::process::Command;

const POWERSHELL_CMD: &str = "powershell";
const DATE_CMD: &str = "date";
const DATE_FMT_WIN: &str = "Get-Date -Format 'yyyy-MM-dd HH:mm:ss'";
const DATE_FMT_UNIX: &str = "+%Y-%m-%d %H:%M:%S";

pub fn get_os_info() -> String {
    let name = System::name().unwrap_or_else(super::unknown);
    let version = System::os_version().unwrap_or_default();
    let arch = std::env::consts::ARCH;
    if version.is_empty() {
        format!("{} {}", name, arch)
    } else {
        format!("{} {} {}", name, version, arch)
    }
}

pub fn get_kernel_info() -> String {
    System::kernel_version().unwrap_or_else(super::unknown)
}

pub fn get_host_name() -> String {
    System::host_name().unwrap_or_else(super::unknown)
}

pub fn get_uptime_info() -> String {
    let uptime = System::uptime();
    let hours = uptime / 3600;
    let mins = (uptime % 3600) / 60;
    let hour_label = if hours == 1 { "hour" } else { "hours" };
    let min_label = if mins == 1 { "min" } else { "mins" };
    format!("{} {}, {} {}", hours, hour_label, mins, min_label)
}

pub fn get_datetime_info() -> String {
    if cfg!(target_os = "windows") {
        if let Ok(output) = Command::new(POWERSHELL_CMD)
            .arg("-Command")
            .arg(DATE_FMT_WIN)
            .output()
        {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    } else {
        if let Ok(output) = Command::new(DATE_CMD).arg(DATE_FMT_UNIX).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
    }
    super::unknown()
}

pub fn get_local_ip_info(networks: &Networks) -> String {
    for (_name, data) in networks {
        for ip in data.ip_networks() {
            if let std::net::IpAddr::V4(ipv4) = ip.addr
                && !ipv4.is_loopback()
            {
                return ipv4.to_string();
            }
        }
    }
    "127.0.0.1".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_os_info() {
        let os = get_os_info();
        assert!(!os.is_empty(), "OS info should not be empty");
    }

    #[test]
    fn test_get_kernel_info() {
        let kernel = get_kernel_info();
        assert!(!kernel.is_empty(), "kernel info should not be empty");
    }

    #[test]
    fn test_get_uptime_info() {
        let uptime = get_uptime_info();
        assert!(
            uptime.contains("hour") || uptime.contains("min"),
            "uptime '{}' should contain hour or min",
            uptime
        );
    }

    #[test]
    fn test_get_datetime_info() {
        let dt = get_datetime_info();
        assert!(dt.len() >= 10, "datetime should be at least YYYY-MM-DD: got '{}'", dt);
    }
}
