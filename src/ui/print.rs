use crate::config::Config;
use crate::plugins::AnimationFrame;
use console::strip_ansi_codes;
use crossterm::cursor::{Hide, MoveToColumn, MoveUp, Show};
use crossterm::execute;
use crossterm::style::{Color, Print, ResetColor, SetForegroundColor};
use crossterm::terminal::size;
use crossterm::terminal::{Clear, ClearType};
use std::io::{Stdout, stdout};
use std::time::{Duration, Instant};

const LOGO_INFO_GAP: &str = "  ";

fn truncate_line(line: &str, max_visible: usize) -> String {
    let stripped_len = console::measure_text_width(&strip_ansi_codes(line));
    if stripped_len <= max_visible {
        return line.to_string();
    }
    let mut result = String::new();
    let mut visible = 0;
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            result.push(ch);
            if chars.next() == Some('[') {
                result.push('[');
                loop {
                    match chars.next() {
                        Some(c) if c.is_ascii_alphabetic() && c != '[' => {
                            result.push(c);
                            break;
                        }
                        Some(c) => result.push(c),
                        None => break,
                    }
                }
            }
        } else if visible < max_visible {
            result.push(ch);
            visible += 1;
        } else {
            result.push_str("...");
            break;
        }
    }
    result
}
const DEFAULT_LOGO_COLOR: Color = Color::Rgb {
    r: 128,
    g: 128,
    b: 128,
};
const MIN_FRAME_DELAY_MS: u64 = 1;

pub fn print_stacked_output(
    ascii_lines: Vec<String>,
    image_printed: bool,
    image_height: usize,
    content_lines: Vec<String>,
    _config: &Config,
    logo_first: bool,
) {
    let mut out = stdout();

    if image_printed && !ascii_lines.is_empty() {
        let (first, second) = if logo_first {
            (&ascii_lines, &content_lines)
        } else {
            (&content_lines, &ascii_lines)
        };
        for line in first {
            let _ = execute!(out, Print(line), Print("\n"));
        }
        if !second.is_empty() {
            let _ = execute!(out, Print("\n"));
            for line in second {
                let _ = execute!(out, Print(line), Print("\n"));
            }
        }
        return;
    }

    if image_printed {
        for _ in 0..image_height {
            let _ = execute!(out, Print("\n"));
        }
        if !content_lines.is_empty() {
            let _ = execute!(out, Print("\n"));
            for line in &content_lines {
                let _ = execute!(out, Print(line), Print("\n"));
            }
        }
        return;
    }

    if logo_first {
        for line in &ascii_lines {
            let _ = execute!(out, Print(line), Print("\n"));
        }
        if !content_lines.is_empty() {
            let _ = execute!(out, Print("\n"));
            for line in &content_lines {
                let _ = execute!(out, Print(line), Print("\n"));
            }
        }
    } else {
        for line in &content_lines {
            let _ = execute!(out, Print(line), Print("\n"));
        }
        if !ascii_lines.is_empty() {
            let _ = execute!(out, Print("\n"));
            for line in &ascii_lines {
                let _ = execute!(out, Print(line), Print("\n"));
            }
        }
    }
}

pub fn print_output(
    ascii_lines: Vec<String>,
    image_printed: bool,
    ascii_width: usize,
    image_height: usize,
    content_lines: Vec<String>,
    config: &Config,
    force_plain_logo: bool,
) {
    let mut out = stdout();

    let term_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(LOGO_INFO_GAP) + gap_base;

    let text_column = ascii_width + gap;
    let mut max_content_width = term_width.saturating_sub(text_column);

    if max_content_width < 10 && term_width > 40 {
        max_content_width = term_width.saturating_sub(ascii_width.max(12));
    }

    let text_height = content_lines.len();
    let logo_height = if image_printed { image_height } else { ascii_lines.len() };
    let max_lines = std::cmp::max(logo_height, text_height);

    for i in 0..max_lines {
        if image_printed {
            let _ = execute!(out, MoveToColumn(text_column as u16));
        } else {
            let ascii_line = if i < ascii_lines.len() {
                &ascii_lines[i]
            } else {
                ""
            };
            print_logo_line(&mut out, ascii_line, ascii_width, config, force_plain_logo);
            execute!(out, Print(LOGO_INFO_GAP)).unwrap();
        }

        if i < content_lines.len() {
            let line = truncate_line(&content_lines[i], max_content_width);
            execute!(out, Print(&line)).unwrap();
        }
        execute!(out, Print("\n")).unwrap();
    }
}

pub fn print_animated_output(
    frames: &[AnimationFrame],
    ascii_width: usize,
    content_lines: &[String],
    config: &Config,
    force_plain_logo: bool,
    duration_ms: Option<u64>,
    loop_enabled: bool,
) {
    if frames.is_empty() {
        return;
    }

    let mut out = stdout();
    let max_logo_width = max_frame_width(frames, ascii_width);
    let max_logo_lines = frames
        .iter()
        .map(|frame| frame.lines.len())
        .max()
        .unwrap_or(0);
    let max_lines = std::cmp::max(max_logo_lines, content_lines.len());
    let start = Instant::now();
    let duration_limit = duration_ms.map(Duration::from_millis);
    let loop_enabled = loop_enabled && duration_limit.is_some();
    let mut frame_index = 0;
    let mut first_frame = true;

    let term_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap_len = console::measure_text_width(LOGO_INFO_GAP) + gap_base;
    let max_content_width = content_lines
        .iter()
        .map(|l| visible_width(l))
        .max()
        .unwrap_or(0);
    let available_content_width = term_width.saturating_sub(max_logo_width + gap_len);
    let line_physical_width = max_logo_width + gap_len + max_content_width;
    let wraps = line_physical_width.div_ceil(term_width);
    let physical_lines = max_lines * std::cmp::max(1, wraps);
    let scroll_margin = physical_lines + 4;

    let _ = execute!(out, Hide);

    for _ in 0..scroll_margin {
        let _ = execute!(out, Print("\n"));
    }
    let _ = execute!(out, MoveUp(scroll_margin as u16));

    loop {
        let frame = &frames[frame_index];

        if !first_frame {
            let _ = execute!(out, MoveUp(scroll_margin as u16));
            let _ = execute!(out, Clear(ClearType::FromCursorDown));
        }

        for i in 0..max_lines {
            let ascii_line = frame.lines.get(i).map(|line| line.as_str()).unwrap_or("");
            print_logo_line(
                &mut out,
                ascii_line,
                max_logo_width,
                config,
                force_plain_logo,
            );
            let _ = execute!(out, Print(LOGO_INFO_GAP));
            if i < content_lines.len() {
                let line = truncate_line(&content_lines[i], available_content_width);
                let _ = execute!(out, Print(&line));
            }
            let _ = execute!(out, Print("\n"));
        }

        let delay = std::cmp::max(MIN_FRAME_DELAY_MS, frame.delay_ms);
        std::thread::sleep(Duration::from_millis(delay));

        if !loop_enabled {
            if frame_index + 1 >= frames.len() {
                break;
            }
        } else if let Some(limit) = duration_limit
            && start.elapsed() >= limit
        {
            break;
        }

        frame_index = (frame_index + 1) % frames.len();
        first_frame = false;
    }

    let _ = execute!(out, Show);
}

fn print_logo_line(
    out: &mut Stdout,
    ascii_line: &str,
    ascii_width: usize,
    config: &Config,
    force_plain_logo: bool,
) {
    let is_custom_ascii = force_plain_logo || config.ascii.is_some() || config.logo_path.is_some();
    let visible_len = visible_width(ascii_line);
    let padding = ascii_width.saturating_sub(visible_len);

    if is_custom_ascii {
        execute!(out, Print(format!("{}{}", ascii_line, " ".repeat(padding)))).unwrap();
    } else {
        execute!(
            out,
            SetForegroundColor(DEFAULT_LOGO_COLOR),
            Print(format!("{}{}", ascii_line, " ".repeat(padding))),
            ResetColor
        )
        .unwrap();
    }
}

fn max_frame_width(frames: &[AnimationFrame], fallback: usize) -> usize {
    let mut max_width = fallback;
    for frame in frames {
        for line in &frame.lines {
            let width = visible_width(line);
            if width > max_width {
                max_width = width;
            }
        }
    }
    max_width
}

fn visible_width(value: &str) -> usize {
    let stripped = console::strip_ansi_codes(value);
    console::measure_text_width(&stripped)
}
