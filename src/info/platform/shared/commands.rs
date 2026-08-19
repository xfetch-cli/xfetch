//! Backwards-compatible re-export of the subprocess machinery.
//!
//! The implementation moved to `crate::subprocess` (shared with the
//! plugin/extension runners without creating a module cycle); existing
//! callers of `run_cmd_with_timeout` keep working unchanged.

pub use crate::subprocess::*;
