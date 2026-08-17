//! macOS implementations of OS-specific probes.
//!
//! Contract: every file exposes the same function names as `linux/` and
//! `windows/`, and `crate::info::platform` re-exports the active one.

pub mod battery;
pub mod datetime;
pub mod gpu;
pub mod packages;

pub use battery::get_battery_info;
pub use datetime::get_datetime_info;
pub use gpu::get_gpu_info;
pub use packages::get_packages_breakdown;
