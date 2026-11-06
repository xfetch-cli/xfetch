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

pub(crate) fn is_image_file(path: &str) -> bool {
    IMAGE_EXTENSIONS.iter().any(|ext| path.ends_with(ext))
}

pub(crate) fn auto_logo_width() -> u32 {
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

pub(crate) fn is_image_logo(config: &Config) -> bool {
    config
        .logo_path
        .as_deref()
        .is_some_and(|path| match config.logo_type.as_deref() {
            Some("ascii") => false,
            Some("image") => true,
            _ => is_image_file(path),
        })
}

pub fn get_logo_data(config: &Config) -> (Vec<String>, bool, usize, usize) {
    load_logo(config, None)
}

/// Loads the logo for the right layout, placing an image flush against the
/// terminal's right edge.
pub(crate) fn get_logo_data_right(
    config: &Config,
    term_width: usize,
) -> (Vec<String>, bool, usize, usize) {
    load_logo(config, Some(term_width))
}

/// Width, in terminal cells, that `viuer` will actually print. Its resize logic
/// only downscales, so a source narrower than the requested width keeps its
/// original width; when a height is also configured the resize is exact.
fn printed_image_width(source_width: u32, requested: u32, logo_height: Option<u32>) -> u32 {
    if logo_height.is_some() {
        requested
    } else {
        source_width.min(requested)
    }
}

#[derive(Debug, Clone)]
pub(crate) struct LogoDiagnostics {
    pub is_image: bool,
    pub path: Option<String>,
    pub path_exists: bool,
    pub requested_width: Option<u32>,
    pub printed_width: Option<usize>,
    pub ascii_lines: Option<usize>,
    pub gap: usize,
    pub content_width: Option<usize>,
    pub image_x: Option<u16>,
}

pub(crate) fn diagnostics(config: &Config, term_width: usize) -> LogoDiagnostics {
    let is_image = is_image_logo(config);
    let gap = 2 + config.logo_gap.unwrap_or(12) as usize;
    let (path, path_exists) = if let Some(path) = &config.logo_path {
        (Some(path.clone()), expand_path(path).exists())
    } else if let Some(path) = &config.ascii {
        (Some(path.clone()), expand_path(path).exists())
    } else {
        (None, true)
    };
    let requested_width = config.logo_width.unwrap_or_else(auto_logo_width);

    let mut printed_width = None;
    let mut ascii_lines = None;
    if is_image {
        if let Some(path) = &config.logo_path
            && let Ok((width, _height)) = image::image_dimensions(expand_path(path))
        {
            printed_width =
                Some(printed_image_width(width, requested_width, config.logo_height) as usize);
        }
    } else {
        let content = if let Some(path) = &config.logo_path {
            std::fs::read_to_string(expand_path(path)).ok()
        } else if let Some(path) = &config.ascii {
            std::fs::read_to_string(expand_path(path)).ok()
        } else {
            Some(get_default_ascii())
        };
        if let Some(content) = content {
            let lines: Vec<&str> = content.lines().collect();
            let width = lines
                .iter()
                .map(|line| console::measure_text_width(&strip_ansi_codes(line)))
                .max()
                .unwrap_or(0);
            printed_width = Some(width);
            ascii_lines = Some(lines.len());
        }
    }

    let content_width = printed_width.map(|width| term_width.saturating_sub(width + gap));
    let image_x = if is_image {
        printed_width.map(|width| term_width.saturating_sub(width).min(u16::MAX as usize) as u16)
    } else {
        None
    };

    LogoDiagnostics {
        is_image,
        path,
        path_exists,
        requested_width: Some(requested_width),
        printed_width,
        ascii_lines,
        gap,
        content_width,
        image_x,
    }
}

fn load_logo(
    config: &Config,
    right_term_width: Option<usize>,
) -> (Vec<String>, bool, usize, usize) {
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

            let _ = execute!(
                stdout,
                crossterm::cursor::SavePosition,
                crossterm::cursor::MoveToColumn(0)
            );

            if let Ok(img) = image::open(&path) {
                let printed_width = printed_image_width(img.width(), img_width, config.logo_height);
                let image_x = right_term_width
                    .map(|term_width| {
                        term_width
                            .saturating_sub(printed_width as usize)
                            .min(u16::MAX as usize) as u16
                    })
                    .unwrap_or(0);

                let conf = ViuerConfig {
                    width: Some(img_width),
                    height: config.logo_height,
                    absolute_offset: false,
                    x: image_x,
                    transparent: false,
                    use_kitty: use_native,
                    ..Default::default()
                };

                if let Ok((width, height)) = print(&img, &conf) {
                    image_printed = true;
                    ascii_width = width as usize;
                    image_height = height as usize;
                    let _ = stdout.flush();
                }
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
            if !config.show_colors || config.no_color {
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

    #[test]
    fn printed_width_matches_viuer_downscale_only_behaviour() {
        assert_eq!(printed_image_width(10, 20, None), 10);
        assert_eq!(printed_image_width(120, 20, None), 20);
        assert_eq!(printed_image_width(10, 20, Some(5)), 20);
        assert_eq!(printed_image_width(120, 20, Some(5)), 20);
    }

    #[test]
    fn right_layout_uses_the_printed_width_for_the_offset() {
        let term_width = 80usize;
        let printed = printed_image_width(10, 20, None) as usize;
        let image_x = term_width.saturating_sub(printed);
        assert_eq!(image_x + printed, term_width);
        assert_eq!(image_x, 70);
    }
}
