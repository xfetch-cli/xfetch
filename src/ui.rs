use crate::config::{Config, FramePaths, LogoAnimationConfig};
use crate::info::Info;
use crate::plugins::run_logo_animation_plugin;
use console::strip_ansi_codes;
use std::io::{IsTerminal, stdout};
mod nodes;
use nodes::prepare_render_tree;
mod layout;
mod logo;
mod print;
mod renders;
mod x;

fn load_animation_frames(config: &LogoAnimationConfig) -> Option<Vec<Vec<String>>> {
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

const FRAME_SEPARATOR: &str = "\n===\n";

fn split_ascii_frames(content: &str) -> Vec<Vec<String>> {
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

pub fn draw(info: &Info, config: &Config) {
    let _stdout = stdout();

    let nodes = prepare_render_tree(info, &config.modules, config);

    let (ascii_lines, image_printed, ascii_width, image_height) = logo::get_logo_data(config);

    let content_lines = layout::get_content_lines(&nodes, config);

    if !image_printed
        && !ascii_lines.is_empty()
        && let Some(animation_config) = &config.logo_animation
        && let Some(plugin_name) = animation_config.plugin.as_deref()
        && stdout().is_terminal()
    {
        let frame_sets = load_animation_frames(animation_config);
        if let Ok(mut frames) =
            run_logo_animation_plugin(plugin_name, animation_config, &ascii_lines, frame_sets)
        {
            if !config.show_colors {
                for frame in &mut frames {
                    frame.lines = frame
                        .lines
                        .iter()
                        .map(|line| strip_ansi_codes(line).to_string())
                        .collect();
                }
            }

            print::print_animated_output(
                &frames,
                ascii_width,
                &content_lines,
                config,
                true,
                animation_config.duration_ms,
                animation_config.loop_enabled.unwrap_or(false),
            );
            return;
        }
    }

    if layout::is_minimal(config.layout.as_ref()) {
        for line in &content_lines {
            println!("{}", line);
        }
        return;
    }

    if layout::is_vertical(config.layout.as_ref()) {
        print::print_stacked_output(
            ascii_lines,
            image_printed,
            image_height,
            content_lines,
            config,
            !layout::is_bottom(config.layout.as_ref()),
        );
        return;
    }

    print::print_output(
        ascii_lines,
        image_printed,
        ascii_width,
        image_height,
        content_lines,
        config,
        false,
    );
}
