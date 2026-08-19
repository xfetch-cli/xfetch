//! Network probing on Linux.
//!
//! `get_local_ip_info` in `crate::info::system` picks the first non-loopback
//! IPv4 address in HashMap order, which on Linux tends to surface virtual
//! interfaces (docker0/veth, br-, tun/tap, Tailscale) instead of the physical
//! NIC. This module prefers a physical interface and only falls back to the
//! generic behavior when every address is virtual.

use sysinfo::Networks;

/// Virtual-interface name markers; such interfaces are skipped when a physical
/// one is available.
const VIRTUAL_MARKERS: &[&str] = &[
    "docker",
    "veth",
    "br-",
    "virbr",
    "tun",
    "tap",
    "wg",
    "tailscale",
    "zerotier",
    "vmnet",
    "vmware",
    "virtualbox",
    "hamachi",
    "lxc",
];

fn is_virtual(name: &str) -> bool {
    let n = name.to_lowercase();
    VIRTUAL_MARKERS.iter().any(|m| n.contains(m))
}

/// First non-loopback IPv4 address of a physical interface, falling back to
/// virtual interfaces and finally `127.0.0.1` when nothing else exists.
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
        assert!(is_virtual("docker0"));
        assert!(is_virtual("veth8f3a2b"));
        assert!(is_virtual("br-7e9a1c2d"));
        assert!(is_virtual("virbr0"));
        assert!(is_virtual("tun0"));
        assert!(is_virtual("tailscale0"));
        assert!(!is_virtual("enp2s0"));
        assert!(!is_virtual("wlp3s0"));
        assert!(!is_virtual("eth0"));
    }
}
