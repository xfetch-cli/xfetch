//! Top-level error handling of the CLI.
//!
//! `run()` in `main.rs` returns a `Result` and every failure funnels through
//! `exit_on_error`, the single handler that prints and exits with code 1.
//! Each variant carries the full message; `Display` restores the exact
//! prefixes the user-facing output expects.

use std::fmt;

#[derive(Debug)]
pub enum XFetchError {
    Fatal(String),
    CleanCache(String),
    GenConfig(String),
}

impl fmt::Display for XFetchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            XFetchError::Fatal(msg) => write!(f, "Error: {}", msg),
            XFetchError::CleanCache(msg) => write!(f, "Failed to clean cache: {}", msg),
            XFetchError::GenConfig(msg) => write!(f, "Failed to generate config: {}", msg),
        }
    }
}

impl std::error::Error for XFetchError {}

/// Prints the error (with its prefix) and exits 1. The only error handler in
/// the binary: every `run()` failure funnels through here.
pub fn exit_on_error(result: Result<(), XFetchError>) {
    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
