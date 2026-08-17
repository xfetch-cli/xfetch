//! Platform-agnostic machinery shared by all OS implementations.
//!
//! This module holds the *mechanism* (command runner, package-check runner,
//! timeouts, helpers) but never the *commands* themselves — each OS keeps its
//! own probe tables in its own folder.

pub mod commands;
pub mod packages;

pub const UNKNOWN_GPU: &str = "Unknown GPU";
pub const NA: &str = "N/A";
