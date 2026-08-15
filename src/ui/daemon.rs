use crate::config::{Config, config_dir};
use crate::info::Info;
use crate::plugins::{AnimationFrame, run_logo_animation_plugin};
use crate::ui::frames::load_animation_frames;
use crate::ui::layout;
use crate::ui::logo;
use crate::ui::nodes::prepare_render_tree;
use crate::ui::print::{
    build_daemon_frame_buffer, daemon_move_to_prompt, daemon_prepare, print_daemon_output,
    restore_terminal, DaemonState, LOGO_INFO_GAP,
};
use console::strip_ansi_codes;
use std::io::{IsTerminal, Write, stdout};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_signal(_: i32) {
    INTERRUPTED.store(true, Ordering::SeqCst);
}

fn install_signal_handlers() {
    use libc::sighandler_t;
    unsafe {
        let handler = handle_signal as *const () as sighandler_t;
        libc::signal(libc::SIGINT, handler);
        libc::signal(libc::SIGTERM, handler);
        libc::signal(libc::SIGHUP, handler);
    }
}

fn pid_file_path() -> PathBuf {
    config_dir().join("xfetch").join("daemon.pid")
}

fn rows_file_path() -> PathBuf {
    config_dir().join("xfetch").join("daemon.rows")
}

fn write_pid_file(pid: i32) {
    if let Some(parent) = pid_file_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(pid_file_path(), pid.to_string());
}

fn write_rows_file(rows: u16) {
    if let Some(parent) = rows_file_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(rows_file_path(), rows.to_string());
}

fn remove_pid_file() {
    let _ = std::fs::remove_file(pid_file_path());
    let _ = std::fs::remove_file(rows_file_path());
}

#[cfg(target_os = "linux")]
fn is_xfetch_process(pid: i32) -> bool {
    match std::fs::read_to_string(format!("/proc/{}/comm", pid)) {
        Ok(comm) => comm.trim() == "xfetch",
        Err(_) => false,
    }
}

#[cfg(not(target_os = "linux"))]
fn is_xfetch_process(_pid: i32) -> bool {
    true
}

pub fn stop_daemon() -> bool {
    let Ok(content) = std::fs::read_to_string(pid_file_path()) else {
        return false;
    };
    let Ok(pid) = content.trim().parse::<i32>() else {
        return false;
    };
    if !is_xfetch_process(pid) {
        remove_pid_file();
        return false;
    }
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    remove_pid_file();
    true
}

fn prepare_frames(
    info: &Info,
    config: &Config,
) -> Option<(Vec<AnimationFrame>, usize, Vec<String>, bool)> {
    if !stdout().is_terminal() {
        return None;
    }

    let nodes = prepare_render_tree(info, &config.modules, config);
    let (ascii_lines, image_printed, ascii_width, _image_height) = logo::get_logo_data(config);

    let term_width = crossterm::terminal::size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(LOGO_INFO_GAP) + gap_base;
    let mut available_width = term_width.saturating_sub(ascii_width + gap);
    if available_width < 10 && term_width > 40 {
        available_width = term_width.saturating_sub(ascii_width.max(12));
    }

    let content_lines = layout::get_content_lines(&nodes, config, Some(available_width));

    let Some(animation_config) = &config.logo_animation else {
        return None;
    };
    let Some(plugin_name) = animation_config.plugin.as_deref() else {
        return None;
    };

    let frame_sets = load_animation_frames(animation_config);
    let Ok(mut frames) =
        run_logo_animation_plugin(plugin_name, animation_config, &ascii_lines, frame_sets)
    else {
        return None;
    };

    if frames.is_empty() {
        return None;
    }

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

    let force_plain_logo = image_printed;
    Some((frames, ascii_width, content_lines, force_plain_logo))
}

/// Entry point for `--daemon` (or `daemon: true` in config).
///
/// Forks to the background: the parent writes its PID to
/// `~/.config/xfetch/daemon.pid` and exits immediately, so the shell prompt
/// returns instantly. The child keeps rendering the animation loop pinned at
/// the top of the terminal. Stop it with `--daemon-stop`.
pub fn draw_daemon(info: &Info, config: &Config) {
    let Some((frames, ascii_width, content_lines, force_plain_logo)) =
        prepare_frames(info, config)
    else {
        return;
    };

    stop_daemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let state: DaemonState = daemon_prepare(&frames, ascii_width, &content_lines, config);

    let mut out = stdout();

    let buffer = build_daemon_frame_buffer(&frames[0], &state, &content_lines, config, force_plain_logo);
    let _ = out.write_all(buffer.as_bytes());
    daemon_move_to_prompt(&mut out, &state);
    let _ = out.flush();

    let pid = unsafe { libc::fork() };
    if pid < 0 {
        restore_terminal();
        return;
    }

    if pid > 0 {
        write_pid_file(pid);
        write_rows_file(state.block_height);
        return;
    }

    unsafe {
        libc::setsid();
    }
    INTERRUPTED.store(false, Ordering::SeqCst);
    install_signal_handlers();

    print_daemon_output(&frames, ascii_width, &content_lines, config, force_plain_logo);

    remove_pid_file();
    restore_terminal();
    std::process::exit(0);
}
