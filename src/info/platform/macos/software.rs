//! macOS implementations of the "software" probes (`get_shell_info`,
//! `get_desktop_info`). The cross-platform module `crate::info::software`
//! dispatches here on macOS; everything macOS-specific lives in this folder.

use std::env;
use std::path::Path;

const ENV_SHELL: &str = "SHELL";

const DESKTOP_AQUA: &str = "Aqua";

/// Shell detection: `SHELL` env (e.g. `/bin/zsh` → `zsh`); falls back to the
/// shared unknown label when the variable is unset.
pub fn get_shell_info() -> String {
    if let Ok(shell) = env::var(ENV_SHELL) {
        let path = Path::new(&shell);
        if let Some(name) = path.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    crate::info::unknown()
}

pub fn get_desktop_info() -> String {
    DESKTOP_AQUA.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macos_desktop_is_aqua() {
        assert_eq!(get_desktop_info(), "Aqua");
    }

    #[test]
    fn test_macos_shell_not_empty() {
        assert!(!get_shell_info().is_empty());
    }
}
