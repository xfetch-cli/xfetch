//! Windows implementations of the "software" probes (`get_shell_info`,
//! `get_desktop_info`). The cross-platform module `crate::info::software`
//! dispatches here on Windows; everything Windows-specific lives in this
//! folder.

use std::env;
use std::path::Path;

use crate::info::platform::windows::shell::detect_shell_name;

const ENV_SHELL: &str = "SHELL";
const ENV_PS_MODULE_PATH: &str = "PSModulePath";

const SHELL_POWERSHELL: &str = "PowerShell";
const SHELL_CMD: &str = "cmd";
const DESKTOP_EXPLORER: &str = "Explorer";

/// Full Windows shell detection: `SHELL` env (Git Bash/MSYS), then the real
/// parent-process walk (`shell::detect_shell_name`), then a `PSModulePath`
/// fallback and finally `cmd`. `PSModulePath` alone cannot distinguish cmd
/// from PowerShell (it is a persistent system variable), hence the parent
/// walk first.
pub fn get_shell_info() -> String {
    if let Ok(shell) = env::var(ENV_SHELL) {
        let path = Path::new(&shell);
        if let Some(name) = path.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    if let Some(shell) = detect_shell_name() {
        return shell;
    }
    if env::var(ENV_PS_MODULE_PATH).is_ok() {
        return SHELL_POWERSHELL.to_string();
    }
    SHELL_CMD.to_string()
}

pub fn get_desktop_info() -> String {
    DESKTOP_EXPLORER.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_desktop_is_explorer() {
        assert_eq!(get_desktop_info(), "Explorer");
    }

    #[test]
    fn test_windows_shell_not_empty() {
        assert!(!get_shell_info().is_empty());
    }
}
