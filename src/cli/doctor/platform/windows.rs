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
