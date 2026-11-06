use super::{Detail, command_exists, env_detail};

pub fn collect() -> Vec<Detail> {
    let mut details = Vec::new();

    if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
        for line in content.lines() {
            if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                details.push(detail("Distribution", trim_quotes(value)));
            } else if let Some(value) = line.strip_prefix("VERSION_ID=") {
                details.push(detail("Distribution version", trim_quotes(value)));
            } else if let Some(value) = line.strip_prefix("ID=") {
                details.push(detail("Distribution id", trim_quotes(value)));
            }
        }
    }

    if let Some(detail) = env_detail("WSL_DISTRO_NAME") {
        details.push(detail);
    } else if let Ok(version) = std::fs::read_to_string("/proc/version")
        && version.to_ascii_lowercase().contains("microsoft")
    {
        details.push(detail("WSL", "yes"));
    }

    for name in [
        "XDG_SESSION_TYPE",
        "XDG_CURRENT_DESKTOP",
        "WAYLAND_DISPLAY",
        "DISPLAY",
    ] {
        if let Some(detail) = env_detail(name) {
            details.push(detail);
        }
    }

    let managers: Vec<&str> = ["pacman", "dpkg", "rpm", "apk", "nix", "flatpak", "brew"]
        .into_iter()
        .filter(|command| command_exists(command))
        .collect();
    details.push(detail(
        "Package managers",
        if managers.is_empty() {
            "none detected".to_string()
        } else {
            managers.join(", ")
        },
    ));

    details
}

fn detail(name: &str, value: impl Into<String>) -> Detail {
    Detail {
        name: name.to_string(),
        value: value.into(),
    }
}

fn trim_quotes(value: &str) -> String {
    value.trim().trim_matches('"').to_string()
}
