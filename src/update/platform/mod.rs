//! Platform-specific self-update policy.
//!
//! Shared orchestration lives in `update::mod`; every OS-specific decision
//! (release asset target, executable name, cargo failure policy and winget
//! delegation) lives in its own module so Windows and Unix code never share a
//! runtime branch.

#[cfg(unix)]
mod unix;
#[cfg(not(any(unix, windows)))]
mod unsupported;
#[cfg(windows)]
mod windows;

#[cfg(unix)]
pub(super) use unix::*;
#[cfg(not(any(unix, windows)))]
pub(super) use unsupported::*;
#[cfg(windows)]
pub(super) use windows::*;
