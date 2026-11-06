use super::{Detail, env_detail};

pub fn collect() -> Vec<Detail> {
    let mut details = Vec::new();

    if let Ok(output) = std::process::Command::new("sw_vers")
        .arg("-productVersion")
        .output()
        && output.status.success()
    {
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !version.is_empty() {
            details.push(detail("macOS version", version));
        }
    }

    for name in ["TERM_PROGRAM", "SHELL", "SSH_CONNECTION"] {
        if let Some(detail) = env_detail(name) {
            details.push(detail);
        }
    }

    details
}

fn detail(name: &str, value: impl Into<String>) -> Detail {
    Detail {
        name: name.to_string(),
        value: value.into(),
    }
}
