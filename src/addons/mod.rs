//! Shared addon metadata and user-facing guidance.
//!
//! Plugins, effects and extensions use different directories and binary
//! prefixes, but they share the same wasm manifest format and the same
//! install/list/info workflow. This module keeps the cross-cutting pieces in
//! one place so the three command families stay consistent.

pub mod catalog;
pub mod install;
pub mod model;
pub mod receipt;
pub mod report;
pub mod update;
