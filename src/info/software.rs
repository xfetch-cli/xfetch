use std::env;
#[cfg(not(target_os = "windows"))]
use std::path::Path;

#[cfg(not(target_os = "windows"))]
const ENV_SHELL: &str = "SHELL";
const ENV_TERM_PROGRAM: &str = "TERM_PROGRAM";
const ENV_WT_SESSION: &str = "WT_SESSION";
const ENV_TERM: &str = "TERM";
const ENV_XDG_DESKTOP: &str = "XDG_CURRENT_DESKTOP";
const ENV_DESKTOP_SESSION: &str = "DESKTOP_SESSION";
const ENV_USER: &str = "USER";
const ENV_USERNAME: &str = "USERNAME";

const TERMINAL_WT: &str = "Windows Terminal";
#[cfg(target_os = "macos")]
const DESKTOP_AQUA: &str = "Aqua";

/// Shell detection. Platform-specific logic lives in `platform/`:
/// `platform/windows/software.rs` implements the Windows probes (the parent
/// process walk lives in `platform/windows/shell.rs`); other platforms fall
/// back to `SHELL`.
pub fn get_shell_info() -> String {
    #[cfg(target_os = "windows")]
    {
        crate::info::platform::windows::software::get_shell_info()
    }
    #[cfg(not(target_os = "windows"))]
    {
        if let Ok(shell) = env::var(ENV_SHELL) {
            let path = Path::new(&shell);
            if let Some(name) = path.file_name() {
                return name.to_string_lossy().into_owned();
            }
        }
        super::unknown()
    }
}

pub fn get_terminal_info() -> String {
    if let Ok(term) = env::var(ENV_TERM_PROGRAM) {
        return term;
    }
    if env::var(ENV_WT_SESSION).is_ok() {
        return TERMINAL_WT.to_string();
    }
    if let Ok(term) = env::var(ENV_TERM) {
        return term;
    }
    super::unknown()
}

/// Desktop/window-manager detection. The XDG checks are generic; the
/// platform defaults (Explorer on Windows, Aqua on macOS) live per platform
/// (`platform/windows/software.rs`).
pub fn get_desktop_info() -> String {
    if let Ok(de) = env::var(ENV_XDG_DESKTOP) {
        return de;
    }
    if let Ok(de) = env::var(ENV_DESKTOP_SESSION) {
        return de;
    }
    #[cfg(target_os = "windows")]
    {
        crate::info::platform::windows::software::get_desktop_info()
    }
    #[cfg(target_os = "macos")]
    {
        DESKTOP_AQUA.to_string()
    }
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        super::unknown()
    }
}

pub fn get_user_info() -> String {
    env::var(ENV_USER)
        .or_else(|_| env::var(ENV_USERNAME))
        .unwrap_or_else(|_| super::unknown())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_user_info() {
        let user = get_user_info();
        assert!(!user.is_empty(), "user info should not be empty");
        let expected = env::var("USER")
            .or_else(|_| env::var("USERNAME"))
            .unwrap_or_else(|_| "Unknown".to_string());
        assert_eq!(user, expected);
    }

    #[test]
    fn test_get_desktop_not_empty() {
        let de = get_desktop_info();
        assert!(!de.is_empty(), "desktop info should not be empty");
    }
}
