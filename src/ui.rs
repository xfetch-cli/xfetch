use crate::config::Config;
use crate::info::Info;
use crate::plugins::run_logo_animation_plugin;
use console::strip_ansi_codes;
use std::io::{IsTerminal, stdout};
use xfetch_effect_api::EffectFrame;
pub mod custom_x;
#[cfg(unix)]
mod daemon;
mod frames;
mod layout;
#[cfg(unix)]
mod live;
mod logo;
mod nodes;
mod print;
mod renders;
mod x;
#[cfg(unix)]
pub use daemon::{draw_daemon, stop_daemon};
pub use layout::is_known_layout;
#[cfg(unix)]
pub use live::{draw_live_daemon, stop_live_daemon};

#[cfg(not(unix))]
pub fn draw_daemon(_info: &Info, _config: &Config) {
    eprintln!("Daemon mode is not supported on Windows.");
}

#[cfg(not(unix))]
pub fn stop_daemon() -> bool {
    eprintln!("Daemon mode is not supported on Windows.");
    false
}

#[cfg(not(unix))]
pub fn draw_live_daemon(
    _info: &Info,
    _config: &Config,
    _config_path: Option<String>,
    _reload: bool,
) {
    eprintln!("Live daemon mode is not supported on Windows.");
}

#[cfg(not(unix))]
pub fn stop_live_daemon() -> bool {
    eprintln!("Live daemon mode is not supported on Windows.");
    false
}
use frames::load_animation_frames;
use nodes::prepare_render_tree;

pub fn draw(info: &Info, config: &Config) {
    let _stdout = stdout();

    let nodes = prepare_render_tree(info, &config.modules, config);

    let (ascii_lines, image_printed, ascii_width, image_height) = logo::get_logo_data(config);

    let term_width = crossterm::terminal::size()
        .map(|(w, _)| w as usize)
        .unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(print::LOGO_INFO_GAP) + gap_base;
    let mut available_width = term_width.saturating_sub(ascii_width + gap);
    if available_width < 10 && term_width > 40 {
        available_width = term_width.saturating_sub(ascii_width.max(12));
    }

    let content_lines = layout::get_content_lines(&nodes, config, Some(available_width));

    // Intro effects over the content lines (opt-in; skipped when an effect
    // binary is missing or fails, so the plain fetch is unaffected). Multiple
    // effects play in sequence.
    let effects = config.effects_list();
    if !image_printed && stdout().is_terminal() && !effects.is_empty() {
        let mut played: Vec<Vec<EffectFrame>> = Vec::new();
        for effect_cfg in &effects {
            match crate::effects::run_effect(effect_cfg, &content_lines) {
                Ok(frames) if !frames.is_empty() => played.push(frames),
                Ok(_) => eprintln!(
                    "Effect '{}' returned no frames; skipped.",
                    effect_cfg.plugin.as_deref().unwrap_or("?")
                ),
                Err(err) => eprintln!(
                    "Effect '{}' skipped: {}",
                    effect_cfg.plugin.as_deref().unwrap_or("?"),
                    err
                ),
            }
        }
        if !played.is_empty() {
            let (logo_frames, _) = logo::build_logo_frames(config, &ascii_lines, image_printed);
            let force_plain = logo_frames.len() > 1;
            print::print_effect_output(
                &logo_frames,
                ascii_width,
                &played,
                &content_lines,
                config,
                force_plain,
            );
            return;
        }
    }

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
            logo::apply_logo_style(&mut frames, config);
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
