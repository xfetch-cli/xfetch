//! macOS implementations of OS-specific probes.
//!
//! Contract: every file exposes the same function names as `linux/` and
//! `windows/`, and `crate::info::platform` re-exports the active one.

pub mod battery;
pub mod datetime;
pub mod gpu;
#[cfg(unix)]
pub mod live;
pub mod network;
pub mod packages;
pub mod software;
pub mod version;

pub use battery::get_battery_info;
pub use datetime::get_datetime_info;
pub use gpu::{get_gpu_info, gpu_fields};
pub use network::get_local_ip_info;
pub use packages::get_packages_breakdown;
pub use software::{get_desktop_info, get_shell_info};
