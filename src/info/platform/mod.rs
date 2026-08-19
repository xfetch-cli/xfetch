//! OS-specific probes, one folder per platform plus `shared/` machinery.
//!
//! Every per-OS folder exposes the same functions: `get_gpu_info()`,
//! `get_battery_info()`, `get_datetime_info()`, `get_packages_breakdown()`,
//! `get_local_ip_info()` (plus logo-catalog helpers per platform). Only the
//! active platform is compiled (`#[cfg(target_os)]`), so code that touches
//! another OS never enters the build.

pub mod shared;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;
/// Compiled on Linux too (WSL *is* Linux); dispatched at runtime.
#[cfg(target_os = "linux")]
pub mod wsl;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
/// Windows re-exports the four probes from `windows/mod.rs`; the network probe
/// lives in its own module, so it is re-exported here to keep the same
/// `platform::get_local_ip_info` surface as the other OSes.
#[cfg(target_os = "windows")]
pub use windows::network::get_local_ip_info;
#[cfg(target_os = "windows")]
pub use windows::*;

/// Logo catalog category for the running OS (`--gen-config`).
pub fn logo_category() -> &'static str {
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
}

/// `(ID, ID_LIKE)` for the running OS, used by the logo catalog resolution.
/// On Linux these come from `/etc/os-release` (lowercased); on macOS/Windows
/// the base id plus a version-specific candidate (e.g. `macos-ventura`,
/// `windows-11`) when the version can be mapped.
pub fn detect_os_ids() -> (String, Vec<String>) {
    #[cfg(target_os = "linux")]
    {
        linux::os_release::detect_os_ids()
    }
    #[cfg(target_os = "macos")]
    {
        macos::version::detect_os_ids()
    }
    #[cfg(target_os = "windows")]
    {
        let mut ids = Vec::new();
        if let Some(v) = sysinfo::System::os_version()
            && let Some(specific) = windows::version::windows_version_id(&v)
        {
            ids.push(specific.to_string());
        }
        ("windows".to_string(), ids)
    }
}
