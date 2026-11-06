use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ModuleInfo {
    pub key: &'static str,
    pub description: &'static str,
    pub platforms: &'static str,
}

/// Canonical module keys accepted in `modules`. Keep this list in sync with
/// `ui::nodes::get_module_value` and the documentation tables.
pub const MODULES: &[ModuleInfo] = &[
    ModuleInfo {
        key: "os",
        description: "Operating system name and architecture",
        platforms: "all",
    },
    ModuleInfo {
        key: "kernel",
        description: "Kernel version",
        platforms: "all",
    },
    ModuleInfo {
        key: "hostname",
        description: "Machine hostname",
        platforms: "all",
    },
    ModuleInfo {
        key: "host",
        description: "Alias for hostname",
        platforms: "all",
    },
    ModuleInfo {
        key: "user",
        description: "Current user",
        platforms: "all",
    },
    ModuleInfo {
        key: "shell",
        description: "Login shell",
        platforms: "all",
    },
    ModuleInfo {
        key: "terminal",
        description: "Terminal emulator",
        platforms: "all",
    },
    ModuleInfo {
        key: "wm",
        description: "Desktop environment or window manager",
        platforms: "linux,macos,windows",
    },
    ModuleInfo {
        key: "cpu",
        description: "CPU model, cores and frequency",
        platforms: "all",
    },
    ModuleInfo {
        key: "gpu",
        description: "GPU model and vendor",
        platforms: "all",
    },
    ModuleInfo {
        key: "memory",
        description: "RAM usage",
        platforms: "all",
    },
    ModuleInfo {
        key: "swap",
        description: "Swap usage",
        platforms: "all",
    },
    ModuleInfo {
        key: "disk",
        description: "Disk usage",
        platforms: "all",
    },
    ModuleInfo {
        key: "battery",
        description: "Battery status",
        platforms: "linux,macos,windows",
    },
    ModuleInfo {
        key: "uptime",
        description: "System uptime",
        platforms: "all",
    },
    ModuleInfo {
        key: "packages",
        description: "Installed package count",
        platforms: "all",
    },
    ModuleInfo {
        key: "packages:<manager>",
        description: "Count for one package manager (pacman, dpkg, rpm, apk, nix, brew, flatpak, ...)",
        platforms: "all",
    },
    ModuleInfo {
        key: "datetime",
        description: "Local date and time",
        platforms: "all",
    },
    ModuleInfo {
        key: "local_ip",
        description: "Local IPv4 address",
        platforms: "all",
    },
    ModuleInfo {
        key: "local_ip:v6",
        description: "Local IPv6 address",
        platforms: "all",
    },
    ModuleInfo {
        key: "public_ip",
        description: "Public IP address (network)",
        platforms: "all",
    },
    ModuleInfo {
        key: "interfaces",
        description: "Network interface summary",
        platforms: "all",
    },
    ModuleInfo {
        key: "palette",
        description: "Terminal color palette",
        platforms: "all",
    },
    ModuleInfo {
        key: "header",
        description: "user@hostname header",
        platforms: "all",
    },
    ModuleInfo {
        key: "sep",
        description: "Separator line",
        platforms: "all",
    },
    ModuleInfo {
        key: "plugin:<name>",
        description: "Lines returned by an info plugin",
        platforms: "all",
    },
];

pub fn find(key: &str) -> Option<&'static ModuleInfo> {
    if let Some(info) = MODULES.iter().find(|info| info.key == key) {
        return Some(info);
    }
    if key.starts_with("packages:") && key.len() > "packages:".len() {
        return MODULES.iter().find(|info| info.key == "packages:<manager>");
    }
    if key.starts_with("plugin:") && key.len() > "plugin:".len() {
        return MODULES.iter().find(|info| info.key == "plugin:<name>");
    }
    None
}

pub fn is_known(key: &str) -> bool {
    find(key).is_some()
}

pub fn print_modules(json: bool) -> Result<(), crate::error::XFetchError> {
    if json {
        let output = serde_json::to_string_pretty(MODULES)
            .map_err(|err| crate::error::XFetchError::Fatal(err.to_string()))?;
        println!("{output}");
        return Ok(());
    }

    println!("{:<18} {:<20} DESCRIPTION", "KEY", "PLATFORMS");
    for module in MODULES {
        println!(
            "{:<18} {:<20} {}",
            module.key, module.platforms, module.description
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_keys_and_patterns() {
        assert!(is_known("os"));
        assert!(is_known("packages:pacman"));
        assert!(is_known("plugin:banner"));
        assert!(!is_known("packages:"));
        assert!(!is_known("plugin:"));
        assert!(!is_known("nope"));
    }

    #[test]
    fn registry_keys_are_unique() {
        let mut keys: Vec<&str> = MODULES.iter().map(|info| info.key).collect();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), MODULES.len());
    }
}
