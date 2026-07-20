use std::env;
use std::path::Path;
use std::process::Command;

const ENV_SHELL: &str = "SHELL";
const ENV_PS_MODULE_PATH: &str = "PSModulePath";
const ENV_TERM_PROGRAM: &str = "TERM_PROGRAM";
const ENV_WT_SESSION: &str = "WT_SESSION";
const ENV_TERM: &str = "TERM";
const ENV_XDG_DESKTOP: &str = "XDG_CURRENT_DESKTOP";
const ENV_DESKTOP_SESSION: &str = "DESKTOP_SESSION";
const ENV_USER: &str = "USER";
const ENV_USERNAME: &str = "USERNAME";

const SHELL_POWERSHELL: &str = "PowerShell";
const SHELL_CMD: &str = "cmd";
const TERMINAL_WT: &str = "Windows Terminal";
const DESKTOP_EXPLORER: &str = "Explorer";
const DESKTOP_AQUA: &str = "Aqua";

const PACMAN_CMD: &str = "pacman";
const DPKG_CMD: &str = "dpkg";
const SCOOP_CMD: &str = "scoop";
const BREW_CMD: &str = "brew";

pub fn get_shell_info() -> String {
    if let Ok(shell) = env::var(ENV_SHELL) {
        let path = Path::new(&shell);
        if let Some(name) = path.file_name() {
            return name.to_string_lossy().into_owned();
        }
    }
    if cfg!(target_os = "windows") {
        if env::var(ENV_PS_MODULE_PATH).is_ok() {
            return SHELL_POWERSHELL.to_string();
        }
        return SHELL_CMD.to_string();
    }
    super::unknown()
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

fn count_packages_linux() -> Option<String> {
    let checks: &[(&str, &[&str])] = &[
        (PACMAN_CMD, &["-Qq"]),
        (DPKG_CMD, &["--get-selections"]),
        ("rpm", &["-qa"]),
        ("flatpak", &["list", "--app"]),
        ("snap", &["list"]),
    ];
    for (cmd, args) in checks {
        if let Ok(output) = Command::new(cmd).args(*args).output()
            && output.status.success()
        {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            return Some(format!("{} ({})", count, cmd));
        }
    }
    None
}

fn count_packages_windows() -> Option<String> {
    let checks: &[(&str, &[&str])] = &[
        (SCOOP_CMD, &["list"]),
    ];
    for (cmd, args) in checks {
        if let Ok(output) = Command::new(cmd).args(*args).output()
            && output.status.success()
        {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            let count = count.saturating_sub(4);
            return Some(format!("{} ({})", count, cmd));
        }
    }
    None
}

fn count_packages_macos() -> Option<String> {
    let checks: &[(&str, &[&str])] = &[
        (BREW_CMD, &["list", "--formula"]),
    ];
    for (cmd, args) in checks {
        if let Ok(output) = Command::new(cmd).args(*args).output()
            && output.status.success()
        {
            let count = String::from_utf8_lossy(&output.stdout).lines().count();
            return Some(format!("{} ({})", count, cmd));
        }
    }
    None
}

pub fn get_packages_info() -> String {
    if cfg!(target_os = "linux")
        && let Some(info) = count_packages_linux()
    {
        return info;
    }
    if cfg!(target_os = "windows")
        && let Some(info) = count_packages_windows()
    {
        return info;
    }
    if cfg!(target_os = "macos")
        && let Some(info) = count_packages_macos()
    {
        return info;
    }
    super::unknown()
}

pub fn get_desktop_info() -> String {
    if let Ok(de) = env::var(ENV_XDG_DESKTOP) {
        return de;
    }
    if let Ok(de) = env::var(ENV_DESKTOP_SESSION) {
        return de;
    }
    if cfg!(target_os = "windows") {
        return DESKTOP_EXPLORER.to_string();
    }
    if cfg!(target_os = "macos") {
        return DESKTOP_AQUA.to_string();
    }
    super::unknown()
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
