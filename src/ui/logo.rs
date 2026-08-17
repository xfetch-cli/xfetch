use super::renders::color_sgr;
use super::x::{expand_path, get_default_ascii};
use crate::config::Config;
use console::strip_ansi_codes;
use crossterm::execute;
use crossterm::terminal::size;
use std::io::{Write, stdout};
use viuer::{Config as ViuerConfig, print_from_file};

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

            if let Ok((width, height)) = print_from_file(&path, &conf) {
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
        if let Some(color) = &config.logo_color {
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

/// Applies `logo_padding` and `logo_color` to animation frames produced by a
/// plugin. Lines that already contain ANSI styling are left untouched.
pub fn apply_logo_style(frames: &mut [xfetch_plugin_api::AnimationFrame], config: &Config) {
    let pad = config.logo_padding.unwrap_or(0);
    let color = config.logo_color.as_deref().map(color_sgr);
    for frame in frames {
        for line in &mut frame.lines {
            if pad > 0 {
                *line = format!("{}{}", " ".repeat(pad), line);
            }
            if let Some(code) = &color
                && !line.contains('\u{1b}')
            {
                *line = format!("\x1b[{}m{}\x1b[0m", code, line);
            }
        }
    }
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
}
