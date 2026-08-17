pub mod hardware;
pub mod platform;
pub mod software;
pub mod system;

use crate::cache;
use crate::config::{Config, InfoPluginConfig, ModuleConfig};
use crate::plugins::run_info_plugin;
use std::collections::{HashMap, HashSet};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::{Disks, Networks, System};

pub use platform::shared::packages::get_packages_info;
pub use platform::{get_battery_info, get_datetime_info, get_gpu_info, get_packages_breakdown};
pub use system::{get_host_name, get_kernel_info, get_os_info, get_uptime_info};

const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;

pub(crate) fn unknown() -> String {
    "Unknown".to_string()
}

fn b_to_gib(bytes: u64) -> f64 {
    bytes as f64 / BYTES_PER_GIB
}

fn collect_module_keys(modules: &[ModuleConfig]) -> HashSet<String> {
    let mut keys = HashSet::new();
    for module in modules {
        match module {
            ModuleConfig::Simple(key) => {
                keys.insert(key.clone());
            }
            ModuleConfig::Group {
                modules: children, ..
            } => {
                keys.extend(collect_module_keys(children));
            }
        }
    }
    keys
}

fn needs_sysinfo(keys: &HashSet<String>) -> bool {
    keys.contains("os")
        || keys.contains("kernel")
        || keys.contains("hostname")
        || keys.contains("host")
        || keys.contains("uptime")
        || keys.contains("cpu")
        || keys.contains("memory")
        || keys.contains("swap")
}

fn needs_network(keys: &HashSet<String>) -> bool {
    keys.contains("local_ip") || keys.contains("local_ip:v6") || keys.contains("interfaces")
}

fn needs_any_packages(keys: &HashSet<String>) -> bool {
    keys.contains("packages") || keys.iter().any(|k| k.starts_with("packages:"))
}

pub struct Info {
    pub os: String,
    pub kernel: String,
    pub host_name: String,
    pub shell: String,
    pub terminal: String,
    pub cpu: String,
    pub gpu: Vec<String>,
    pub memory: String,
    pub swap: String,
    pub disks: Vec<String>,
    pub battery: String,
    pub uptime: String,
    pub packages: String,
    pub packages_breakdown: Vec<(String, String)>,
    pub desktop: String,
    pub user: String,
    pub datetime: String,
    pub local_ip: String,
    pub local_ip_v6: String,
    pub public_ip: String,
    pub network_interfaces: String,
    pub plugin_info: HashMap<String, Vec<String>>,
}

impl Info {
    pub fn with_config(config: &Config, benchmark: bool) -> (Self, Vec<String>) {
        let _total_start = Instant::now();
        let needed = collect_module_keys(&config.modules);

        let sys = if needs_sysinfo(&needed) {
            let mut s = System::new_all();
            s.refresh_all();
            Some(s)
        } else {
            None
        };

        let disks = if needed.contains("disk") {
            Some(Disks::new_with_refreshed_list())
        } else {
            None
        };

        let networks = if needs_network(&needed) {
            Some(Networks::new_with_refreshed_list())
        } else {
            None
        };

        let cache_enabled = !config.disable_cache.unwrap_or(false);

        let cached_packages = if cache_enabled {
            cache::get("packages", Duration::from_secs(300))
        } else {
            None
        };

        let ip_enabled =
            needed.contains("public_ip") && !config.disable_ip_fetching.unwrap_or(false);
        let cached_ip = if ip_enabled && cache_enabled {
            cache::get("public_ip", Duration::from_secs(300))
        } else {
            None
        };

        let mut _parallel_elapsed = None;

        let (gpu, pkg_opt, ip_opt) = thread::scope(|s| {
            let _ps = Instant::now();
            let gpu_h = needed.contains("gpu").then(|| s.spawn(get_gpu_info));
            let pkg_h = (needs_any_packages(&needed) && cached_packages.is_none())
                .then(|| s.spawn(|| (get_packages_info(), get_packages_breakdown())));
            let ip_h = (ip_enabled && cached_ip.is_none())
                .then(|| s.spawn(|| system::get_public_ip_info(true)));

            let gpu = gpu_h.and_then(|h| h.join().ok()).unwrap_or_default();
            let pkg = pkg_h.and_then(|h| h.join().ok());
            let ip = ip_h.and_then(|h| h.join().ok());
            if benchmark {
                _parallel_elapsed = Some(_ps.elapsed());
            }
            (gpu, pkg, ip)
        });

        let (packages, packages_breakdown) = match cached_packages {
            Some(json) => serde_json::from_str(&json).unwrap_or_else(|_| (unknown(), Vec::new())),
            None => match pkg_opt {
                Some((ref p, ref b)) => {
                    if cache_enabled && let Ok(json) = serde_json::to_string(&(p, b)) {
                        cache::set("packages", &json);
                    }
                    (p.clone(), b.clone())
                }
                None => (unknown(), Vec::new()),
            },
        };

        let public_ip = match cached_ip {
            Some(ip) => ip,
            None => match ip_opt {
                Some(ip) => {
                    if cache_enabled {
                        cache::set("public_ip", &ip);
                    }
                    ip
                }
                None => "N/A".to_string(),
            },
        };

        let plugin_info = load_plugin_info(&config.info_plugins);

        let bench_lines = if benchmark {
            let total = _total_start.elapsed();
            vec![
                format!(
                    "  Parallel (GPU+packages+IP): {:>6}.{:03}s",
                    _parallel_elapsed.unwrap().as_secs(),
                    _parallel_elapsed.unwrap().subsec_millis()
                ),
                format!(
                    "  Total:                     {:>6}.{:03}s",
                    total.as_secs(),
                    total.subsec_millis()
                ),
            ]
        } else {
            Vec::new()
        };

        (
            Self {
                os: get_os_info(),
                kernel: get_kernel_info(),
                host_name: get_host_name(),
                shell: software::get_shell_info(),
                terminal: software::get_terminal_info(),
                cpu: sys.as_ref().map(hardware::get_cpu_info).unwrap_or_default(),
                gpu,
                memory: sys
                    .as_ref()
                    .map(hardware::get_memory_info)
                    .unwrap_or_default(),
                swap: sys
                    .as_ref()
                    .map(hardware::get_swap_info)
                    .unwrap_or_default(),
                disks: disks
                    .as_ref()
                    .map(hardware::get_disk_info)
                    .unwrap_or_default(),
                battery: if needed.contains("battery") {
                    get_battery_info()
                } else {
                    "N/A".to_string()
                },
                uptime: get_uptime_info(),
                packages,
                packages_breakdown,
                desktop: software::get_desktop_info(),
                user: software::get_user_info(),
                datetime: get_datetime_info(),
                local_ip: networks
                    .as_ref()
                    .map(system::get_local_ip_info)
                    .unwrap_or_else(|| "127.0.0.1".to_string()),
                local_ip_v6: networks
                    .as_ref()
                    .map(system::get_ipv6_info)
                    .unwrap_or_else(|| "N/A".to_string()),
                public_ip,
                network_interfaces: networks
                    .as_ref()
                    .map(system::get_network_interfaces_info)
                    .unwrap_or_else(|| "N/A".to_string()),
                plugin_info,
            },
            bench_lines,
        )
    }
}

fn load_plugin_info(plugins: &[InfoPluginConfig]) -> HashMap<String, Vec<String>> {
    let mut result = HashMap::new();
    for plugin_cfg in plugins {
        match run_info_plugin(plugin_cfg) {
            Ok(lines) => {
                let key = format!("plugin:{}", plugin_cfg.plugin);
                result.insert(key, lines);
            }
            Err(err) => {
                eprintln!("Plugin '{}' error: {}", plugin_cfg.plugin, err);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_host_name() {
        let host = get_host_name();
        assert!(!host.is_empty(), "host name should not be empty");
    }

    #[test]
    fn test_get_battery_info_fallback() {
        let battery = get_battery_info();
        assert!(battery.contains('%') || battery == "N/A");
    }

    #[test]
    fn test_get_gpu_not_empty() {
        let gpus = get_gpu_info();
        assert!(!gpus.is_empty(), "GPU list should not be empty");
    }

    #[test]
    fn test_get_packages_not_empty() {
        let pkgs = get_packages_info();
        assert!(!pkgs.is_empty(), "packages should not be empty");
    }
}
