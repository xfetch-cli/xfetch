use crate::config::Config;
use crate::info::Info;
use crate::plugins::run_logo_animation_plugin;
use console::strip_ansi_codes;
use std::io::{IsTerminal, stdout};
pub mod custom_x;
mod frames;
mod layout;
mod logo;
mod nodes;
mod print;
mod renders;
mod x;
#[cfg(unix)]
mod daemon;
#[cfg(unix)]
pub use daemon::{draw_daemon, stop_daemon};

#[cfg(not(unix))]
pub fn draw_daemon(_info: &Info, _config: &Config) {}

#[cfg(not(unix))]
pub fn stop_daemon() -> bool {
    false
}
use frames::load_animation_frames;
use nodes::prepare_render_tree;

pub fn draw(info: &Info, config: &Config) {
    let _stdout = stdout();

    let nodes = prepare_render_tree(info, &config.modules, config);

    let (ascii_lines, image_printed, ascii_width, image_height) = logo::get_logo_data(config);

    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(print::LOGO_INFO_GAP) + gap_base;
    let mut available_width = term_width.saturating_sub(ascii_width + gap);
    if available_width < 10 && term_width > 40 {
        available_width = term_width.saturating_sub(ascii_width.max(12));
    }

    let content_lines = layout::get_content_lines(&nodes, config, Some(available_width));

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
