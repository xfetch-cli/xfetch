use crate::config::{FramePaths, LogoAnimationConfig};

pub const FRAME_SEPARATOR: &str = "\n===\n";

pub fn load_animation_frames(config: &LogoAnimationConfig) -> Option<Vec<Vec<String>>> {
    let paths = match config.frames_path.as_ref()? {
        FramePaths::Single(path) => vec![path.clone()],
        FramePaths::Multiple(paths) => paths.clone(),
    };
    let mut frames = Vec::new();
    for path_str in &paths {
        let expanded = crate::ui::x::expand_path(path_str);
        if let Ok(content) = std::fs::read_to_string(&expanded) {
            let sub_frames = split_ascii_frames(&content);
            if sub_frames.is_empty() {
                let lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
                if !lines.is_empty() {
                    frames.push(lines);
                }
            } else {
                frames.extend(sub_frames);
            }
        }
    }
    if frames.is_empty() {
        None
    } else {
        Some(frames)
    }
}

pub fn split_ascii_frames(content: &str) -> Vec<Vec<String>> {
    if !content.contains(FRAME_SEPARATOR) {
        return Vec::new();
    }
    content
        .split(FRAME_SEPARATOR)
        .map(|block| block.lines().map(|l| l.to_string()).collect())
        .filter(|frame: &Vec<String>| {
            !frame.is_empty() && !frame.iter().all(|l| l.trim().is_empty())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_ascii_frames_empty_without_separator() {
        let content = "line1\nline2\n";
        assert!(split_ascii_frames(content).is_empty());
    }

    #[test]
    fn test_split_ascii_frames_separates_blocks() {
        let content = "a\n===\nb\nc\n===\nd\n";
        let frames = split_ascii_frames(content);
        assert_eq!(frames.len(), 3);
        assert_eq!(frames[0], vec!["a"]);
        assert_eq!(frames[1], vec!["b", "c"]);
        assert_eq!(frames[2], vec!["d"]);
    }

    #[test]
    fn test_split_ascii_frames_drops_blank_blocks() {
        let content = "a\n===\n\n===\n  \nb\n";
        let frames = split_ascii_frames(content);
        assert_eq!(frames.len(), 2);
    }
}
