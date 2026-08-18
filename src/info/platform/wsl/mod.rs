//! WSL-specific OS presentation.
//!
//! WSL runs on a Linux kernel, so this folder is compiled alongside `linux/`
//! and dispatched at runtime (there is no `cfg` that selects it): the kernel
//! announces itself in `/proc/version`, which works for every distro inside
//! WSL (Ubuntu, Debian, Arch, openSUSE, Alpine WSL, ...).

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WslStyle {
    #[default]
    Minimal,
    Off,
    Full,
}

impl WslStyle {
    pub fn from_config(value: Option<&str>) -> Self {
        match value {
            Some("off") => Self::Off,
            Some("full") => Self::Full,
            _ => Self::Minimal,
        }
    }
}

/// WSL kernels announce themselves in `/proc/version` ("microsoft"),
/// regardless of the distro running inside.
pub fn is_wsl() -> bool {
    std::fs::read_to_string("/proc/version")
        .map(|v| v.to_lowercase().contains("microsoft"))
        .unwrap_or(false)
}

/// "WSL 1" or "WSL 2" from the kernel string; `None` when undeterminable.
fn wsl_version() -> Option<&'static str> {
    let version = std::fs::read_to_string("/proc/version")
        .ok()?
        .to_lowercase();
    if version.contains("wsl2") || version.contains("microsoft-standard") {
        Some("WSL 2")
    } else if version.contains("microsoft") {
        Some("WSL 1")
    } else {
        None
    }
}

fn wslg_present() -> bool {
    Path::new("/mnt/wslg").exists()
        || std::env::var_os("WAYLAND_DISPLAY")
            .map(|d| d.to_string_lossy().contains("wslg"))
            .unwrap_or(false)
}

/// Applies the configured WSL presentation to an OS string:
/// - `off`     → "Ubuntu 24.04 x86_64"            (no WSL mention)
/// - `minimal` → "Ubuntu 24.04 x86_64 (WSL)"
/// - `full`    → "Ubuntu 24.04 x86_64 (WSL 2, WSLg)"
///
/// No-op on non-WSL systems and when the style is `off`.
pub fn decorate_os(base: String, style: Option<&str>) -> String {
    let style = WslStyle::from_config(style);
    if style == WslStyle::Off || !is_wsl() {
        return base;
    }
    let detail = match style {
        WslStyle::Minimal => "WSL".to_string(),
        WslStyle::Full => {
            let mut parts = vec![wsl_version().unwrap_or("WSL").to_string()];
            if wslg_present() {
                parts.push("WSLg".to_string());
            }
            parts.join(", ")
        }
        WslStyle::Off => return base,
    };
    format!("{} ({})", base, detail)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_wsl_consistent_with_proc_version() {
        let from_proc = std::fs::read_to_string("/proc/version")
            .map(|v| v.to_lowercase().contains("microsoft"))
            .unwrap_or(false);
        assert_eq!(is_wsl(), from_proc);
    }

    #[test]
    fn test_decorate_os_off_is_noop() {
        let base = "Ubuntu 24.04 x86_64".to_string();
        assert_eq!(decorate_os(base.clone(), Some("off")), base);
    }

    #[test]
    fn test_decorate_os_invalid_style_falls_back_to_minimal() {
        assert!(matches!(
            WslStyle::from_config(Some("bogus")),
            WslStyle::Minimal
        ));
        assert!(matches!(WslStyle::from_config(None), WslStyle::Minimal));
    }

    #[test]
    fn test_decorate_os_not_wsl_is_noop() {
        if !is_wsl() {
            assert_eq!(
                decorate_os("Arch Linux".to_string(), Some("minimal")),
                "Arch Linux"
            );
        }
    }
}
