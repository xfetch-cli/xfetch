use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Detail {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformInfo {
    pub os: String,
    pub arch: String,
    pub family: String,
    pub details: Vec<Detail>,
}

pub fn collect() -> PlatformInfo {
    PlatformInfo {
        os: std::env::consts::OS.to_string(),
        arch: std::env::consts::ARCH.to_string(),
        family: std::env::consts::FAMILY.to_string(),
        details: platform_details(),
    }
}

pub fn command_exists(command: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| {
            std::env::split_paths(&paths).any(|dir| {
                let candidate = dir.join(command);
                candidate.is_file() || cfg!(windows) && dir.join(format!("{command}.exe")).is_file()
            })
        })
        .unwrap_or(false)
}

pub fn env_detail(name: &str) -> Option<Detail> {
    std::env::var(name).ok().map(|value| Detail {
        name: name.to_string(),
        value,
    })
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
fn platform_details() -> Vec<Detail> {
    linux::collect()
}

#[cfg(target_os = "macos")]
fn platform_details() -> Vec<Detail> {
    macos::collect()
}

#[cfg(target_os = "windows")]
fn platform_details() -> Vec<Detail> {
    windows::collect()
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn platform_details() -> Vec<Detail> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_info_has_os_and_arch() {
        let info = collect();
        assert!(!info.os.is_empty());
        assert!(!info.arch.is_empty());
    }
}
