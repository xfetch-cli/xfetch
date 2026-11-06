//! Top-level error handling of the CLI.
//!
//! `run()` returns a `Result` and every failure funnels through
//! `exit_on_error`, the single handler that prints and exits with the code
//! associated with the failure kind.
//!
//! Exit codes are stable for scripts:
//! - 0: success
//! - 1: generic failure
//! - 2: usage error (clap)
//! - 3: configuration error
//! - 4: network/offline error
//! - 5: addon error
//! - 6: update error
//! - 7: not found

use std::fmt;

pub const EXIT_GENERIC: i32 = 1;
pub const EXIT_USAGE: i32 = 2;
pub const EXIT_CONFIG: i32 = 3;
pub const EXIT_NETWORK: i32 = 4;
pub const EXIT_ADDON: i32 = 5;
pub const EXIT_UPDATE: i32 = 6;
pub const EXIT_NOT_FOUND: i32 = 7;

#[derive(Debug)]
pub enum XFetchError {
    Fatal(String),
    CleanCache(String),
    GenConfig(String),
    Usage(String),
    Config(String),
    Network(String),
    Addon(String),
    Update(String),
    NotFound(String),
    /// Exit with a specific code without printing an extra message.
    Exit(i32),
}

impl XFetchError {
    pub fn code(&self) -> i32 {
        match self {
            XFetchError::Fatal(_) => EXIT_GENERIC,
            XFetchError::CleanCache(_) => EXIT_GENERIC,
            XFetchError::GenConfig(_) => EXIT_CONFIG,
            XFetchError::Usage(_) => EXIT_USAGE,
            XFetchError::Config(_) => EXIT_CONFIG,
            XFetchError::Network(_) => EXIT_NETWORK,
            XFetchError::Addon(_) => EXIT_ADDON,
            XFetchError::Update(_) => EXIT_UPDATE,
            XFetchError::NotFound(_) => EXIT_NOT_FOUND,
            XFetchError::Exit(code) => *code,
        }
    }
}

impl fmt::Display for XFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XFetchError::Fatal(msg) => write!(f, "Error: {}", msg),
            XFetchError::CleanCache(msg) => write!(f, "Failed to clean cache: {}", msg),
            XFetchError::GenConfig(msg) => write!(f, "Failed to generate config: {}", msg),
            XFetchError::Usage(msg) => write!(f, "Error: {}", msg),
            XFetchError::Config(msg) => write!(f, "Config error: {}", msg),
            XFetchError::Network(msg) => write!(f, "Network error: {}", msg),
            XFetchError::Addon(msg) => write!(f, "Addon error: {}", msg),
            XFetchError::Update(msg) => write!(f, "Update error: {}", msg),
            XFetchError::NotFound(msg) => write!(f, "Not found: {}", msg),
            XFetchError::Exit(_) => Ok(()),
        }
    }
}

impl std::error::Error for XFetchError {}

/// Prints the error (with its prefix) and exits with the matching code.
pub fn exit_on_error(result: Result<(), XFetchError>) {
    if let Err(err) = result {
        let message = err.to_string();
        if !message.is_empty() {
            crate::runtime::error(message);
        }
        std::process::exit(err.code());
    }
}
