//! Shell detection on Windows.
//!
//! Environment variables are unreliable here: `PSModulePath` is a persistent
//! system/user variable that also exists in plain `cmd.exe` sessions, so the
//! generic env-based logic in `crate::info::software` cannot tell cmd from
//! PowerShell. The real shell is found by walking the parent process chain
//! and mapping the first known shell executable (terminal hosts such as
//! conhost/WindowsTerminal are skipped).

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Shell executable names (without the `.exe` suffix) we can identify.
const SHELL_NAMES: &[&str] = &[
    "cmd",
    "powershell",
    "pwsh",
    "bash",
    "sh",
    "zsh",
    "fish",
    "nu",
    "elvish",
    "xonsh",
    "ksh",
    "dash",
    "tcsh",
    "wsl",
];

/// Maximum number of ancestors to walk (guards against pathological chains).
const MAX_DEPTH: usize = 16;

/// Human-readable label for a shell process name.
fn shell_label(name: &str) -> String {
    match name {
        "powershell" => "PowerShell".to_string(),
        "pwsh" => "pwsh".to_string(),
        other => other.to_string(),
    }
}

/// Detects the shell that launched the current process by walking the parent
/// process chain. `None` when no known shell is found (e.g. launched by
/// Explorer); the caller falls back to env detection.
pub fn detect_shell_name() -> Option<String> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing(),
    );
    let current = sysinfo::get_current_pid().ok()?;
    let mut proc = sys.process(current)?;
    for _ in 0..MAX_DEPTH {
        let parent_pid = proc.parent()?;
        let parent = sys.process(parent_pid)?;
        let bare = parent
            .name()
            .to_string_lossy()
            .to_lowercase()
            .trim_end_matches(".exe")
            .to_string();
        if let Some(name) = SHELL_NAMES.iter().find(|s| **s == bare) {
            return Some(shell_label(name));
        }
        if bare == "explorer" {
            return None;
        }
        proc = parent;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shell_label() {
        assert_eq!(shell_label("powershell"), "PowerShell");
        assert_eq!(shell_label("pwsh"), "pwsh");
        assert_eq!(shell_label("cmd"), "cmd");
    }

    #[test]
    fn test_detect_shell_returns_known_name() {
        // `cargo test` runs from a shell, so the parent chain must resolve
        // to a known shell name (or None when launched by a non-shell host);
        // it must never panic or hang.
        if let Some(shell) = detect_shell_name() {
            assert!(SHELL_NAMES.contains(&shell.to_lowercase().as_str()));
        }
    }
}
