use super::renders::color_sgr;
use super::x::{expand_path, get_default_ascii};
use crate::config::Config;
use crate::plugins::{AnimationFrame, run_logo_animation_plugin};
use crate::ui::frames::load_animation_frames;
use console::strip_ansi_codes;
use crossterm::execute;
use crossterm::terminal::size;
use std::io::{Write, stdout};
use viuer::{Config as ViuerConfig, print};

const IMAGE_EXTENSIONS: [&str; 4] = [".png", ".jpg", ".jpeg", ".svg"];

fn is_image_file(path: &str) -> bool {
    IMAGE_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

fn auto_logo_width() -> u32 {
    let term_width = size().map(|(w, _)| w as u32).unwrap_or(80);
    let w = if term_width < 60 {
        (term_width as f32 * 0.20) as u32
    } else {
        (term_width as f32 * 0.28) as u32
    };
    w.clamp(12, 42)
}

fn is_kitty() -> bool {
    std::env::var("KITTY_WINDOW_ID").is_ok()
}

pub fn get_logo_data(config: &Config) -> (Vec<String>, bool, usize, usize) {
    let mut ascii_lines: Vec<String> = Vec::new();
    let mut image_printed = false;
    let mut ascii_width = 0;
    let mut image_height = 0;
    let mut stdout = stdout();

    if let Some(path_str) = &config.logo_path {
        let path = expand_path(path_str);
        let treat_as_image = match config.logo_type.as_deref() {
            Some("ascii") => false,
            Some("image") => true,
            _ => is_image_file(path_str),
        };
        if treat_as_image {
            let img_width = config.logo_width.unwrap_or_else(auto_logo_width);
            let use_native = is_kitty() && config.logo_kitty.unwrap_or(true);

            let conf = ViuerConfig {
                width: Some(img_width),
                height: config.logo_height,
                absolute_offset: false,
                transparent: false,
                use_kitty: use_native,
                ..Default::default()
            };

            let _ = execute!(
                stdout,
                crossterm::cursor::SavePosition,
                crossterm::cursor::MoveToColumn(0)
            );

            if let Ok(img) = image::open(&path)
                && let Ok((width, height)) = print(&img, &conf)
            {
                image_printed = true;
                ascii_width = width as usize;
                image_height = height as usize;
                let _ = stdout.flush();
            }

            let _ = execute!(
                stdout,
                crossterm::cursor::RestorePosition,
                crossterm::cursor::MoveToColumn(0)
            );
            let _ = stdout.flush();
        } else if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                ascii_lines.push(line.to_string());
            }
        }
    } else if let Some(path_str) = &config.ascii {
        let path = expand_path(path_str);
        if let Ok(content) = std::fs::read_to_string(&path) {
            for line in content.lines() {
                ascii_lines.push(line.to_string());
            }
        }
    } else {
        let default_art = get_default_ascii();
        for line in default_art.lines() {
            ascii_lines.push(line.to_string());
        }
    }

    if !image_printed && !ascii_lines.is_empty() {
        ascii_lines = ascii_lines
            .into_iter()
            .map(|l| l.trim_end().to_string())
            .collect();
        if let Some(pad) = config.logo_padding {
            let pad_str = " ".repeat(pad);
            for line in &mut ascii_lines {
                *line = format!("{}{}", pad_str, line);
            }
        }
        if let Some(colors) = &config.logo_colors
            && !colors.is_empty()
        {
            for (row, line) in ascii_lines.iter_mut().enumerate() {
                let code = color_sgr(&colors[row % colors.len()]);
                *line = format!("\x1b[{}m{}\x1b[0m", code, line);
            }
        } else if let Some(color) = &config.logo_color {
            let code = color_sgr(color);
            for line in &mut ascii_lines {
                *line = format!("\x1b[{}m{}\x1b[0m", code, line);
            }
        }
        ascii_width = ascii_lines
            .iter()
            .map(|l| console::measure_text_width(&strip_ansi_codes(l)))
            .max()
            .unwrap_or(0);
    }
    (ascii_lines, image_printed, ascii_width, image_height)
}

/// Applies `logo_padding` and `logo_color`/`logo_colors` to animation frames
/// produced by a plugin. Lines that already contain ANSI styling are left
/// untouched.
pub fn apply_logo_style(frames: &mut [xfetch_plugin_api::AnimationFrame], config: &Config) {
    let pad = config.logo_padding.unwrap_or(0);
    let row_colors: Option<Vec<String>> = config
        .logo_colors
        .as_ref()
        .filter(|c| !c.is_empty())
        .map(|cs| cs.iter().map(|c| color_sgr(c)).collect());
    let single_color = config.logo_color.as_ref().map(|c| color_sgr(c));
    for frame in frames {
        for (row, line) in frame.lines.iter_mut().enumerate() {
            if pad > 0 {
                *line = format!("{}{}", " ".repeat(pad), line);
            }
            let code = row_colors
                .as_ref()
                .map(|cs| cs[row % cs.len()].clone())
                .or_else(|| single_color.clone());
            if let Some(code) = code
                && !line.contains('\u{1b}')
            {
                *line = format!("\x1b[{}m{}\x1b[0m", code, line);
            }
        }
    }
}

/// Logo frames for a render pass: the configured `logo_animation` frames when
/// present, otherwise a single static frame. Returns `(frames,
/// force_plain_logo)` where `force_plain_logo` mirrors `image_printed`.
pub fn build_logo_frames(
    config: &Config,
    ascii_lines: &[String],
    image_printed: bool,
) -> (Vec<AnimationFrame>, bool) {
    if !image_printed
        && !ascii_lines.is_empty()
        && let Some(animation_config) = &config.logo_animation
        && let Some(plugin_name) = animation_config.plugin.as_deref()
    {
        let frame_sets = load_animation_frames(animation_config);
        if let Ok(mut frames) =
            run_logo_animation_plugin(plugin_name, animation_config, ascii_lines, frame_sets)
            && !frames.is_empty()
        {
            apply_logo_style(&mut frames, config);
            if !config.show_colors {
                for frame in &mut frames {
                    frame.lines = frame
                        .lines
                        .iter()
                        .map(|line| strip_ansi_codes(line).to_string())
                        .collect();
                }
            }
            return (frames, false);
        }
    }

    (
        vec![AnimationFrame::new(0, ascii_lines.to_vec())],
        image_printed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_get_logo_data_default() {
        let config = Config::default();
        let (ascii_lines, is_image, _width, _height) = get_logo_data(&config);

        assert!(!is_image);
        assert!(!ascii_lines.is_empty());
    }

    #[test]
    fn test_get_logo_data_color() {
        let config = Config {
            logo_color: Some("Cyan".to_string()),
            ..Config::default()
        };
        let (ascii_lines, _, _, _) = get_logo_data(&config);
        assert!(ascii_lines.iter().all(|l| l.starts_with("\x1b[36m")));
    }

    #[test]
    fn test_get_logo_data_row_colors_cycle() {
        let config = Config {
            logo_colors: Some(vec!["Cyan".to_string(), "Red".to_string()]),
            ..Config::default()
        };
        let (ascii_lines, _, _, _) = get_logo_data(&config);
        assert!(ascii_lines.len() >= 3);
        assert!(ascii_lines[0].starts_with("\x1b[36m"), "row 0 cyan");
        assert!(ascii_lines[1].starts_with("\x1b[31m"), "row 1 red");
        assert!(
            ascii_lines[2].starts_with("\x1b[36m"),
            "row 2 cycles back to cyan"
        );
    }

    #[test]
    fn test_get_logo_data_empty_row_colors_falls_back() {
        let config = Config {
            logo_color: Some("Cyan".to_string()),
            logo_colors: Some(Vec::new()),
            ..Config::default()
        };
        let (ascii_lines, _, _, _) = get_logo_data(&config);
        assert!(ascii_lines.iter().all(|l| l.starts_with("\x1b[36m")));
    }

    #[test]
    fn test_get_logo_data_padding() {
        let config = Config {
            logo_padding: Some(3),
            ..Config::default()
        };
        let (ascii_lines, _, width, _) = get_logo_data(&config);
        assert!(ascii_lines.iter().all(|l| l.starts_with("   ")));
        assert!(width >= 3);
    }

    #[test]
    fn test_apply_logo_style_colors_frames() {
        use xfetch_plugin_api::AnimationFrame;
        let config = Config {
            logo_color: Some("196".to_string()),
            ..Config::default()
        };
        let mut frames = vec![AnimationFrame {
            delay_ms: 10,
            lines: vec!["  hello".to_string(), "\x1b[31mstyled\x1b[0m".to_string()],
        }];
        apply_logo_style(&mut frames, &config);
        assert!(frames[0].lines[0].starts_with("\x1b[38;5;196m"));
        assert!(frames[0].lines[0].ends_with("\x1b[0m"));
        assert_eq!(frames[0].lines[1], "\x1b[31mstyled\x1b[0m");
    }

    #[test]
    fn test_apply_logo_style_row_colors_frames() {
        use xfetch_plugin_api::AnimationFrame;
        let config = Config {
            logo_colors: Some(vec!["Cyan".to_string(), "Magenta".to_string()]),
            ..Config::default()
        };
        let mut frames = vec![AnimationFrame {
            delay_ms: 10,
            lines: vec!["a".to_string(), "b".to_string(), "c".to_string()],
        }];
        apply_logo_style(&mut frames, &config);
        assert!(frames[0].lines[0].starts_with("\x1b[36m"));
        assert!(frames[0].lines[1].starts_with("\x1b[35m"));
        assert!(frames[0].lines[2].starts_with("\x1b[36m"), "cycles");
    }

    #[test]
    fn test_build_logo_frames_static_fallback() {
        let config = Config::default();
        let lines = vec!["█".to_string(), "█".to_string()];
        let (frames, force_plain) = build_logo_frames(&config, &lines, false);
        assert_eq!(frames.len(), 1);
        assert_eq!(frames[0].lines, lines);
        assert!(!force_plain);
    }

    #[test]
    fn test_build_logo_frames_image_forces_plain() {
        let config = Config::default();
        let (frames, force_plain) = build_logo_frames(&config, &[], true);
        assert_eq!(frames.len(), 1);
        assert!(force_plain);
    }
}
