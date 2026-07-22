pub mod hardware;
pub mod software;
pub mod system;

use crate::config::{Config, InfoPluginConfig};
use crate::plugins::run_info_plugin;
use std::collections::HashMap;
use sysinfo::{Components, Disks, Networks, System};

pub use hardware::get_gpu_info;
pub use software::{get_packages_breakdown, get_packages_info};
pub use system::{get_datetime_info, get_host_name, get_kernel_info, get_os_info, get_uptime_info};

const BYTES_PER_GIB: f64 = 1024.0 * 1024.0 * 1024.0;

fn unknown() -> String {
    "Unknown".to_string()
}

fn b_to_gib(bytes: u64) -> f64 {
    bytes as f64 / BYTES_PER_GIB
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
    pub plugin_info: HashMap<String, Vec<String>>,
}

impl Info {
    pub fn with_config(config: &Config) -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let disks = Disks::new_with_refreshed_list();
        let networks = Networks::new_with_refreshed_list();
        let components = Components::new_with_refreshed_list();

        let plugin_info = load_plugin_info(&config.info_plugins);

        Self {
            os: get_os_info(),
            kernel: get_kernel_info(),
            host_name: get_host_name(),
            shell: software::get_shell_info(),
            terminal: software::get_terminal_info(),
            cpu: hardware::get_cpu_info(&sys),
            gpu: get_gpu_info(),
            memory: hardware::get_memory_info(&sys),
            swap: hardware::get_swap_info(&sys),
            disks: hardware::get_disk_info(&disks),
            battery: hardware::get_battery_info(&components),
            uptime: get_uptime_info(),
            packages: get_packages_info(),
            packages_breakdown: get_packages_breakdown(),
            desktop: software::get_desktop_info(),
            user: software::get_user_info(),
            datetime: get_datetime_info(),
            local_ip: system::get_local_ip_info(&networks),
            plugin_info,
        }
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
        use sysinfo::Components;
        let battery = hardware::get_battery_info(&Components::new_with_refreshed_list());
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
