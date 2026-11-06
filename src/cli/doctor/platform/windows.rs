use super::{Detail, env_detail};

pub fn collect() -> Vec<Detail> {
    let mut details = Vec::new();
    for name in [
        "PROCESSOR_ARCHITECTURE",
        "SESSIONNAME",
        "WT_SESSION",
        "HOMEDRIVE",
        "USERPROFILE",
    ] {
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
