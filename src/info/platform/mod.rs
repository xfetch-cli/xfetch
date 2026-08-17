//! OS-specific probes, one folder per platform plus `shared/` machinery.
//!
//! Every per-OS folder exposes the same functions: `get_gpu_info()`,
//! `get_battery_info()`, `get_datetime_info()`, `get_packages_breakdown()`.
//! Only the active platform is compiled (`#[cfg(target_os)]`), so code that
//! touches another OS never enters the build.

pub mod shared;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(target_os = "linux")]
pub use linux::*;
#[cfg(target_os = "macos")]
pub use macos::*;
#[cfg(target_os = "windows")]
pub use windows::*;
