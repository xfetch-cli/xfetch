use super::x::{expand_path, get_default_ascii};
use crate::config::Config;
use crossterm::execute;
use crossterm::terminal::size;
use std::io::{stdout, Write};
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
        if is_image_file(path_str) {
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
        } else {
            if let Ok(content) = std::fs::read_to_string(&path) {
                for line in content.lines() {
                    ascii_lines.push(line.to_string());
                }
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
        ascii_width = ascii_lines
            .iter()
            .map(|l| console::measure_text_width(l))
            .max()
            .unwrap_or(0);
    }
    (ascii_lines, image_printed, ascii_width, image_height)
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
}
