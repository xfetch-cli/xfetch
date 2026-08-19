//! Network probing on Windows.
//!
//! `get_local_ip_info` in `crate::info::system` picks the first non-loopback
//! IPv4 address in HashMap order, which on Windows tends to surface virtual
//! adapters (vEthernet/WSL, Hyper-V, Docker, VPN) instead of the real
//! physical interface. This module prefers a physical interface and only
//! falls back to the generic behavior when every address is virtual.

use sysinfo::Networks;

/// Virtual-adapter name markers; such interfaces are skipped when a physical
/// one is available.
const VIRTUAL_MARKERS: &[&str] = &[
    "vethernet",
    "wsl",
    "hyper-v",
    "docker",
    "virtualbox",
    "vmware",
    "loopback",
    "npcap",
    "tap-",
    "tailscale",
    "zerotier",
    "bluetooth",
    "hamachi",
    "proxmox",
];

fn is_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    VIRTUAL_MARKERS.iter().any(|m| n.contains(m))
}

/// First non-loopback IPv4 address of a physical interface, falling back to
/// virtual adapters and finally `127.0.0.1` when nothing else exists.
pub fn get_local_ip_info(networks: &Networks) -> String {
    let mut virtual_fallback: Option<String> = None;
    for (name, data) in networks {
        for ip in data.ip_networks() {
            if let std::net::IpAddr::V4(ipv4) = ip.addr
                && !ipv4.is_loopback()
            {
                if is_virtual(name) {
                    virtual_fallback.get_or_insert_with(|| ipv4.to_string());
                } else {
                    return ipv4.to_string();
                }
            }
        }
    }
    virtual_fallback.unwrap_or_else(|| "127.0.0.1".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_virtual() {
        assert!(is_virtual("vEthernet (WSL (Hyper-V firewall))"));
        assert!(is_virtual("vEthernet (Default Switch)"));
        assert!(is_virtual("Loopback Pseudo-Interface"));
        assert!(is_virtual("Tailscale"));
        assert!(!is_virtual("Ethernet"));
        assert!(!is_virtual("Wi-Fi"));
    }
}
