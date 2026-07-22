use std::io::{BufReader, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::process::Command;
use std::time::Duration;
use sysinfo::{Networks, System};

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

pub fn get_ipv6_info(networks: &Networks) -> String {
    for (_name, data) in networks {
        for ip in data.ip_networks() {
            if let std::net::IpAddr::V6(ipv6) = ip.addr
                && !ipv6.is_loopback()
            {
                return ipv6.to_string();
            }
        }
    }
    "N/A".to_string()
}

fn fetch_public_ip_from(host: &str) -> Option<String> {
    let addr = (host, 80).to_socket_addrs().ok()?.next()?;
    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(3))).ok()?;

    let request = format!(
        "GET / HTTP/1.0\r\nHost: {}\r\nConnection: close\r\n\r\n",
        host
    );
    stream.write_all(request.as_bytes()).ok()?;

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_to_string(&mut response).ok()?;

    let body = response.split("\r\n\r\n").nth(1)?.trim().to_string();
    if body.is_empty() || body.contains('<') {
        None
    } else {
        Some(body)
    }
}

pub fn get_public_ip_info(enabled: bool) -> String {
    if !enabled {
        return "N/A".to_string();
    }
    for host in &["ifconfig.me", "api.ipify.org", "icanhazip.com"] {
        if let Some(ip) = fetch_public_ip_from(host) {
            return ip;
        }
    }
    "N/A".to_string()
}

fn is_link_local(ipv6: &std::net::Ipv6Addr) -> bool {
    ipv6.octets()[0] == 0xfe && ipv6.octets()[1] == 0x80
}

pub fn get_network_interfaces_info(networks: &Networks) -> String {
    let mut parts: Vec<String> = Vec::new();
    for (name, data) in networks {
        let raw_mac = data.mac_address();
        let mac = if raw_mac.0.iter().any(|&b| b != 0) {
            format!(
                "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                raw_mac.0[0], raw_mac.0[1], raw_mac.0[2], raw_mac.0[3], raw_mac.0[4], raw_mac.0[5]
            )
        } else {
            String::new()
        };
        let ips: Vec<String> = data
            .ip_networks()
            .iter()
            .filter(|ip| match ip.addr {
                std::net::IpAddr::V4(ipv4) => !ipv4.is_loopback(),
                std::net::IpAddr::V6(ipv6) => !ipv6.is_loopback() && !is_link_local(&ipv6),
            })
            .map(|ip| format!("{}/{}", ip.addr, ip.prefix))
            .collect();
        if !ips.is_empty() {
            let label = if mac.is_empty() {
                format!("{} {}", name, ips.join(", "))
            } else {
                format!("{} [{}] {}", name, mac, ips.join(", "))
            };
            parts.push(label);
        }
    }
    if parts.is_empty() {
        "N/A".to_string()
    } else {
        parts.join(" / ")
    }
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
        assert!(
            dt.len() >= 10,
            "datetime should be at least YYYY-MM-DD: got '{}'",
            dt
        );
    }
}
