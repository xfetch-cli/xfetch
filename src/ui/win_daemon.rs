//! Windows daemon implementations (`--daemon` and `daemon_live`).
//!
//! Windows has no `fork`/`setsid` or POSIX signals, so the Unix engine is
//! mirrored with native primitives:
//!
//! - The parent renders the first frame and spawns a worker copy of itself
//!   (`XFETCH_WIN_DAEMON_WORKER`) that inherits the console and keeps
//!   drawing pinned at the top while the shell prompt returns.
//! - `--daemon-stop` opens a named manual-reset event created by the worker
//!   and sets it; if the worker does not exit within a second it is
//!   terminated and the terminal is restored either way.
//! - Frames use ANSI escapes (DECSTBM scroll region, DECSC cursor save)
//!   after enabling `ENABLE_VIRTUAL_TERMINAL_PROCESSING`, and the worker
//!   exits when the shell leaves the console (`GetConsoleProcessList`),
//!   mirroring the Unix pty-hangup check.
//!
//! The Unix implementation (`ui/daemon.rs`, `ui/live.rs`) is untouched.

use crate::config::{Config, ModuleConfig, config_dir, default_themes_dir, load_config};
use crate::info::Info;
use crate::plugins::{AnimationFrame, run_logo_animation_plugin};
use crate::ui::frames::load_animation_frames;
use crate::ui::layout;
use crate::ui::logo;
use crate::ui::nodes::prepare_render_tree;
use crate::ui::print::{FrameGeometry, LOGO_INFO_GAP, compute_frame_geometry};
use console::strip_ansi_codes;
use crossterm::terminal::size;
use std::io::{IsTerminal, Write, stdout};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, Instant, SystemTime};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE, WAIT_OBJECT_0};
use windows_sys::Win32::System::Console::{
    CTRL_CLOSE_EVENT, ENABLE_VIRTUAL_TERMINAL_PROCESSING, GetConsoleMode, GetConsoleProcessList,
    GetStdHandle, STD_OUTPUT_HANDLE, SetConsoleCtrlHandler, SetConsoleMode,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, EVENT_MODIFY_STATE, OpenEventW, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, QueryFullProcessImageNameW, SetEvent, TerminateProcess,
    WaitForSingleObject,
};

/// Environment variable that marks a spawned daemon worker and which loop it
/// must run. The worker inherits the parent's argv, so the normal CLI/config
/// routing happens again and then stops here.
const ENV_WORKER: &str = "XFETCH_WIN_DAEMON_WORKER";
const WORKER_ANIMATED: &str = "animated";
const WORKER_LIVE: &str = "live";

/// Poll cadence (ms) when the logo is static (live daemon).
const POLL_MS: u64 = 100;
/// How long `--daemon-stop` waits for a signalled worker before killing it.
const STOP_WAIT_MS: u32 = 1000;
const MIN_FRAME_DELAY_MS: u64 = 1;
const DEFAULT_LOGO_SGR: &str = "\x1b[38;2;128;128;128m";
const RESET_SGR: &str = "\x1b[0m";

/// Pid/rows files of the running worker, removed on a console close.
static CLOSE_FILES: OnceLock<(PathBuf, PathBuf)> = OnceLock::new();

struct DaemonFiles {
    pid: PathBuf,
    rows: PathBuf,
}

fn daemon_files(live: bool) -> DaemonFiles {
    let dir = config_dir().join("xfetch");
    let name = if live { "daemon_live" } else { "daemon" };
    DaemonFiles {
        pid: dir.join(format!("{name}.pid")),
        rows: dir.join(format!("{name}.rows")),
    }
}

fn write_pid_file(path: &Path, pid: u32) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, pid.to_string());
}

fn write_rows_file(path: &Path, rows: u16) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, rows.to_string());
}

fn remove_files(files: &DaemonFiles) {
    let _ = std::fs::remove_file(&files.pid);
    let _ = std::fs::remove_file(&files.rows);
}

fn worker_mode() -> Option<&'static str> {
    match std::env::var(ENV_WORKER).ok()?.as_str() {
        WORKER_ANIMATED => Some(WORKER_ANIMATED),
        WORKER_LIVE => Some(WORKER_LIVE),
        _ => None,
    }
}

/// Turns on VT sequence processing for the console so the raw escape
/// sequences used by the pinned frames are interpreted. Returns false when
/// stdout is not a console (older Windows without VT support included).
fn enable_vt() -> bool {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_null() {
            return false;
        }
        let mut mode = 0u32;
        if GetConsoleMode(handle, &mut mode) == 0 {
            return false;
        }
        SetConsoleMode(handle, mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING) != 0
    }
}

fn ignore_ctrl_c() {
    unsafe {
        let _ = SetConsoleCtrlHandler(None, 1);
    }
}

unsafe extern "system" fn close_handler(ctrl_type: u32) -> i32 {
    if ctrl_type == CTRL_CLOSE_EVENT
        && let Some((pid, rows)) = CLOSE_FILES.get()
    {
        let _ = std::fs::remove_file(pid);
        let _ = std::fs::remove_file(rows);
    }
    0
}

/// Shared worker console setup: ignore Ctrl+C (the shell keeps sending it and
/// the daemon must survive) and remove the pid files if the console is
/// closed, mirroring `remove_files` from the Unix shutdown path.
fn setup_worker_console(files: &DaemonFiles) {
    let _ = CLOSE_FILES.set((files.pid.clone(), files.rows.clone()));
    let _ = enable_vt();
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(close_handler), 1);
    }
    ignore_ctrl_c();
}

fn event_name(pid: u32) -> Vec<u16> {
    format!("Local\\xfetch-daemon-{pid}")
        .encode_utf16()
        .chain(std::iter::once(0))
        .collect()
}

fn create_stop_event() -> HANDLE {
    let name = event_name(std::process::id());
    unsafe { CreateEventW(std::ptr::null(), 1, 0, name.as_ptr()) }
}

fn stop_requested(event: HANDLE) -> bool {
    !event.is_null() && unsafe { WaitForSingleObject(event, 0) == WAIT_OBJECT_0 }
}

/// True when only this process is still attached to the console: the shell
/// (and everything else) left, so the pinned block has no surface left.
fn console_hung_up() -> bool {
    let mut pids = [0u32; 32];
    let count = unsafe { GetConsoleProcessList(pids.as_mut_ptr(), pids.len() as u32) };
    count == 1
}

fn is_xfetch_process(pid: u32) -> bool {
    let handle = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if handle.is_null() {
        return false;
    }
    let mut buf = [0u16; 260];
    let mut len = buf.len() as u32;
    let ok = unsafe { QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut len) };
    unsafe { CloseHandle(handle) };
    if ok == 0 {
        return false;
    }
    let path = String::from_utf16_lossy(&buf[..len as usize]);
    Path::new(&path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.eq_ignore_ascii_case("xfetch"))
}

/// Restores the terminal after a daemon exit: cursor visible, scroll region
/// reset to the full screen.
pub fn restore_terminal() {
    let mut out = stdout();
    let _ = out.write_all(b"\x1b[?25h\x1b[r");
    let _ = out.flush();
}

fn stop_impl(live: bool) -> bool {
    let files = daemon_files(live);
    let Ok(content) = std::fs::read_to_string(&files.pid) else {
        return false;
    };
    let Ok(pid) = content.trim().parse::<u32>() else {
        return false;
    };
    if !is_xfetch_process(pid) {
        remove_files(&files);
        return false;
    }

    let signal = unsafe { OpenEventW(EVENT_MODIFY_STATE, 0, event_name(pid).as_ptr()) };
    if !signal.is_null() {
        unsafe {
            SetEvent(signal);
            CloseHandle(signal);
        }
        let process = unsafe { OpenProcess(PROCESS_SYNCHRONIZE, 0, pid) };
        if !process.is_null() {
            unsafe {
                WaitForSingleObject(process, STOP_WAIT_MS);
                CloseHandle(process);
            }
        }
    }
    if is_xfetch_process(pid) {
        let process = unsafe { OpenProcess(PROCESS_TERMINATE, 0, pid) };
        if !process.is_null() {
            unsafe {
                TerminateProcess(process, 0);
                CloseHandle(process);
            }
            // The worker could not clean up after itself: restore the console
            // from here (normally the worker does it on the stop event).
            restore_terminal();
        }
    }

    remove_files(&files);
    true
}

pub fn stop_daemon() -> bool {
    stop_impl(false)
}

pub fn stop_live_daemon() -> bool {
    stop_impl(true)
}

/// Spawns the detached worker with the parent's own arguments (so the CLI and
/// config routing resolve identically) plus the worker marker.
fn spawn_worker(mode: &str) -> Option<u32> {
    let exe = std::env::current_exe().ok()?;
    let mut command = std::process::Command::new(exe);
    for arg in std::env::args_os().skip(1) {
        command.arg(arg);
    }
    command
        .env(ENV_WORKER, mode)
        .stdin(std::process::Stdio::null());
    command.spawn().ok().map(|child| child.id())
}

struct DaemonState {
    geometry: FrameGeometry,
    block_height: u16,
    term_height: u16,
    scale: f64,
}

fn daemon_state(
    frames: &[AnimationFrame],
    ascii_width: usize,
    content_lines: &[String],
    config: &Config,
) -> DaemonState {
    let geometry = compute_frame_geometry(frames, ascii_width, content_lines, config);
    let term_height = size().map(|(_, h)| h).unwrap_or(24);
    let min_free_rows = config.daemon_min_rows.unwrap_or(6) as u16;

    let natural_height = geometry.scroll_margin.saturating_sub(4) as u16;
    let max_height = term_height.saturating_sub(min_free_rows).max(1);

    let (block_height, scale) = if natural_height > max_height {
        (max_height, natural_height as f64 / max_height as f64)
    } else {
        (natural_height, 1.0)
    };

    DaemonState {
        geometry,
        block_height: block_height.max(1),
        term_height,
        scale,
    }
}

fn visible_width(value: &str) -> usize {
    let stripped = console::strip_ansi_codes(value);
    console::measure_text_width(&stripped)
}

fn scale_index(row: u16, source_len: usize, state: &DaemonState) -> usize {
    if source_len == 0 {
        return usize::MAX;
    }
    if state.scale <= 1.0 {
        return row as usize;
    }
    let pos = (row as f64) * state.scale;
    (pos as usize).min(source_len - 1)
}

/// Copy of the Unix truncation in `ui/print.rs` (private there), so both
/// platforms cut ANSI-colored lines the same way.
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

fn append_logo_line(
    buf: &mut String,
    ascii_line: &str,
    ascii_width: usize,
    config: &Config,
    force_plain_logo: bool,
) {
    let is_custom_ascii = force_plain_logo || config.ascii.is_some() || config.logo_path.is_some();
    let padding = ascii_width.saturating_sub(visible_width(ascii_line));
    if is_custom_ascii {
        buf.push_str(ascii_line);
        buf.push_str(&" ".repeat(padding));
    } else {
        buf.push_str(DEFAULT_LOGO_SGR);
        buf.push_str(ascii_line);
        buf.push_str(&" ".repeat(padding));
        buf.push_str(RESET_SGR);
    }
}

fn append_daemon_row(
    buf: &mut String,
    ascii_line: &str,
    content_line: Option<&str>,
    geometry: &FrameGeometry,
    config: &Config,
    force_plain_logo: bool,
) {
    append_logo_line(
        buf,
        ascii_line,
        geometry.max_logo_width,
        config,
        force_plain_logo,
    );
    buf.push_str(LOGO_INFO_GAP);
    if let Some(line) = content_line {
        buf.push_str(&truncate_line(line, geometry.available_content_width));
    }
    buf.push_str("\x1b[K");
}

/// Builds the pinned frame as one buffer: hide cursor, save position, assert
/// the scroll region, draw each row with absolute positioning and restore the
/// user's cursor. Emitted with a single `write_all` so it cannot interleave
/// with the shell's output.
fn build_frame_buffer(
    frame: &AnimationFrame,
    state: &DaemonState,
    content_lines: &[String],
    config: &Config,
    force_plain_logo: bool,
) -> String {
    let mut buf = String::new();
    buf.push_str("\x1b[?25l");
    buf.push_str("\x1b7");
    buf.push_str(&format!(
        "\x1b[{};{}r",
        state.block_height + 1,
        state.term_height
    ));

    for row in 0..state.block_height {
        let ascii_line = frame
            .lines
            .get(scale_index(row, frame.lines.len(), state))
            .map(|line| line.as_str())
            .unwrap_or("");
        let content_line = content_lines.get(scale_index(row, content_lines.len(), state));
        buf.push_str(&format!("\x1b[{};1H", row + 1));
        append_daemon_row(
            &mut buf,
            ascii_line,
            content_line.map(|line| line.as_str()),
            &state.geometry,
            config,
            force_plain_logo,
        );
    }

    buf.push_str("\x1b8");
    buf
}

fn move_to_prompt(out: &mut impl Write, state: &DaemonState) {
    let _ = write!(out, "\x1b[{};1H", state.block_height + 1);
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

    let term_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(LOGO_INFO_GAP) + gap_base;
    let mut available_width = term_width.saturating_sub(ascii_width + gap);
    if available_width < 10 && term_width > 40 {
        available_width = term_width.saturating_sub(ascii_width.max(12));
    }

    let content_lines = layout::get_content_lines(&nodes, config, Some(available_width));

    let animation_config = config.logo_animation.as_ref()?;
    let plugin_name = animation_config.plugin.as_deref()?;

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

    Some((frames, ascii_width, content_lines, image_printed))
}

fn run_animated_worker(info: &Info, config: &Config) {
    let Some((frames, ascii_width, content_lines, force_plain_logo)) = prepare_frames(info, config)
    else {
        return;
    };
    let files = daemon_files(false);
    setup_worker_console(&files);
    let event = create_stop_event();

    let mut out = stdout();
    let mut state = daemon_state(&frames, ascii_width, &content_lines, config);
    let mut last_size = size().ok();
    let mut frame_index = 0usize;

    loop {
        if stop_requested(event) || console_hung_up() {
            break;
        }

        let cur_size = match size() {
            Ok(size) => size,
            Err(_) => break,
        };
        if Some(cur_size) != last_size {
            last_size = Some(cur_size);
            state = daemon_state(&frames, ascii_width, &content_lines, config);
        }

        let frame = &frames[frame_index];
        let buffer = build_frame_buffer(frame, &state, &content_lines, config, force_plain_logo);
        if out.write_all(buffer.as_bytes()).is_err() || out.flush().is_err() {
            break;
        }

        std::thread::sleep(Duration::from_millis(
            frame.delay_ms.max(MIN_FRAME_DELAY_MS),
        ));
        frame_index = (frame_index + 1) % frames.len();
    }

    remove_files(&files);
    restore_terminal();
    std::process::exit(0);
}

/// Entry point for `--daemon` (or `daemon: true` in config).
pub fn draw_daemon(info: &Info, config: &Config) {
    if worker_mode() == Some(WORKER_ANIMATED) {
        run_animated_worker(info, config);
        return;
    }
    if !stdout().is_terminal() {
        return;
    }
    let Some((frames, ascii_width, content_lines, force_plain_logo)) = prepare_frames(info, config)
    else {
        return;
    };

    stop_daemon();
    std::thread::sleep(Duration::from_millis(50));

    if !enable_vt() {
        eprintln!("xfetch: daemon mode requires a terminal with ANSI support.");
        return;
    }

    let state = daemon_state(&frames, ascii_width, &content_lines, config);
    let mut out = stdout();
    let buffer = build_frame_buffer(&frames[0], &state, &content_lines, config, force_plain_logo);
    let _ = out.write_all(buffer.as_bytes());
    move_to_prompt(&mut out, &state);
    let _ = out.flush();

    let Some(pid) = spawn_worker(WORKER_ANIMATED) else {
        restore_terminal();
        return;
    };
    let files = daemon_files(false);
    write_pid_file(&files.pid, pid);
    write_rows_file(&files.rows, state.block_height);
}

struct LiveBlock {
    frames: Vec<AnimationFrame>,
    ascii_width: usize,
    force_plain_logo: bool,
    config: Config,
    content_lines: Vec<String>,
    reload: bool,
    watch: ReloadWatch,
}

impl LiveBlock {
    fn build(config: &Config, config_path: Option<&str>, reload: bool) -> Self {
        let modules = live_modules(config);
        let live_cfg = live_config(config, &modules);
        let (ascii_lines, image_printed, ascii_width, _) = logo::get_logo_data(&live_cfg);
        let (frames, force_plain_logo) =
            logo::build_logo_frames(&live_cfg, &ascii_lines, image_printed);
        let content_lines = build_content_lines(
            &Info::with_config(&live_cfg, false).0,
            &live_cfg,
            ascii_width,
        );
        let mut watch = ReloadWatch::default();
        if reload {
            watch.snapshot(&live_cfg, config_path);
        }
        Self {
            frames,
            ascii_width,
            force_plain_logo,
            config: live_cfg,
            content_lines,
            reload,
            watch,
        }
    }

    fn refresh_content(&mut self) {
        let fresh = Info::with_config(&self.config, false).0;
        self.content_lines = build_content_lines(&fresh, &self.config, self.ascii_width);
    }

    fn maybe_reload(&mut self, config_path: Option<&str>) -> bool {
        if !self.reload {
            return false;
        }
        let Some(cp) = config_path else {
            return false;
        };
        if !self.watch.changed(&self.config, Some(cp)) {
            return false;
        }

        let fresh = load_config(Some(cp.to_string()));
        self.watch.snapshot(&fresh, Some(cp));
        let modules = live_modules(&fresh);
        let live_cfg = live_config(&fresh, &modules);
        let (ascii_lines, image_printed, ascii_width, _) = logo::get_logo_data(&live_cfg);
        let (frames, force_plain_logo) =
            logo::build_logo_frames(&live_cfg, &ascii_lines, image_printed);
        self.config = live_cfg;
        self.frames = frames;
        self.ascii_width = ascii_width;
        self.force_plain_logo = force_plain_logo;
        self.refresh_content();
        true
    }
}

fn live_modules(config: &Config) -> Vec<String> {
    match &config.daemon_live_modules {
        Some(list) => list.clone(),
        None => crate::info::platform::windows::live::LIVE_MODULES
            .iter()
            .map(|module| module.to_string())
            .collect(),
    }
}

fn live_config(config: &Config, modules: &[String]) -> Config {
    let mut live = config.clone();
    live.modules = modules
        .iter()
        .map(|key| ModuleConfig::Simple(key.clone()))
        .collect();
    live
}

fn build_content_lines(info: &Info, config: &Config, ascii_width: usize) -> Vec<String> {
    let nodes = prepare_render_tree(info, &config.modules, config);
    let term_width = size().map(|(w, _)| w as usize).unwrap_or(80);
    let gap_base = config.logo_gap.unwrap_or(12) as usize;
    let gap = console::measure_text_width(LOGO_INFO_GAP) + gap_base;
    let mut available_width = term_width.saturating_sub(ascii_width + gap);
    if available_width < 10 && term_width > 40 {
        available_width = term_width.saturating_sub(ascii_width.max(12));
    }
    layout::get_content_lines(&nodes, config, Some(available_width))
}

fn theme_file_path(name: &str) -> PathBuf {
    default_themes_dir().join(format!("{name}.jsonc"))
}

fn file_mtime(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
}

#[derive(Default)]
struct ReloadWatch {
    config: Option<SystemTime>,
    theme: Option<SystemTime>,
}

impl ReloadWatch {
    fn snapshot(&mut self, config: &Config, config_path: Option<&str>) {
        self.config = config_path.and_then(|path| file_mtime(Path::new(path)));
        self.theme = config
            .theme
            .as_ref()
            .and_then(|theme| file_mtime(&theme_file_path(theme)));
    }

    fn changed(&self, config: &Config, config_path: Option<&str>) -> bool {
        let cfg_now = config_path.and_then(|path| file_mtime(Path::new(path)));
        let theme_now = config
            .theme
            .as_ref()
            .and_then(|theme| file_mtime(&theme_file_path(theme)));
        self.config != cfg_now || self.theme != theme_now
    }
}

fn run_live_worker(mut block: LiveBlock, config_path: Option<String>) {
    let files = daemon_files(true);
    setup_worker_console(&files);
    let event = create_stop_event();

    let mut out = stdout();
    let mut state = daemon_state(
        &block.frames,
        block.ascii_width,
        &block.content_lines,
        &block.config,
    );
    let mut last_size = size().ok();
    let mut frame_index = 0usize;
    let mut last_refresh = Instant::now();
    let mut first = true;

    loop {
        if stop_requested(event) || console_hung_up() {
            break;
        }

        let cur_size = match size() {
            Ok(size) => size,
            Err(_) => break,
        };
        let resized = Some(cur_size) != last_size;
        if resized {
            last_size = Some(cur_size);
            state = daemon_state(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
        }

        let now = Instant::now();
        let default_refresh = crate::info::platform::windows::live::DEFAULT_LIVE_REFRESH_SECS;
        let interval = Duration::from_secs(
            block
                .config
                .daemon_live_refresh
                .unwrap_or(default_refresh)
                .max(1),
        );
        let refreshed = now.duration_since(last_refresh) >= interval;
        if refreshed {
            block.refresh_content();
            state = daemon_state(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
            last_refresh = now;
        }

        let reloaded = block.maybe_reload(config_path.as_deref());
        if reloaded {
            state = daemon_state(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
            frame_index = 0;
            last_refresh = now;
        }

        let animated = block.frames.len() > 1;
        if first || animated || resized || refreshed || reloaded {
            let frame = &block.frames[frame_index];
            let buffer = build_frame_buffer(
                frame,
                &state,
                &block.content_lines,
                &block.config,
                block.force_plain_logo,
            );
            if out.write_all(buffer.as_bytes()).is_err() || out.flush().is_err() {
                break;
            }
            frame_index = (frame_index + 1) % block.frames.len();
            first = false;
        }

        let sleep_ms = if animated {
            block.frames[frame_index].delay_ms.max(1)
        } else {
            POLL_MS
        };
        std::thread::sleep(Duration::from_millis(sleep_ms));
    }

    remove_files(&files);
    restore_terminal();
    std::process::exit(0);
}

/// Entry point for the live stats daemon (`daemon_live: true` in config).
pub fn draw_live_daemon(_info: &Info, config: &Config, config_path: Option<String>, reload: bool) {
    if worker_mode() == Some(WORKER_LIVE) {
        let block = LiveBlock::build(config, config_path.as_deref(), reload);
        run_live_worker(block, config_path);
        return;
    }
    if !stdout().is_terminal() {
        return;
    }
    if let Ok((w, h)) = size()
        && (w == 0 || h == 0)
    {
        return;
    }
    if !enable_vt() {
        eprintln!("xfetch: live daemon mode requires a terminal with ANSI support.");
        return;
    }

    let block = LiveBlock::build(config, config_path.as_deref(), reload);

    stop_live_daemon();
    std::thread::sleep(Duration::from_millis(50));

    let state = daemon_state(
        &block.frames,
        block.ascii_width,
        &block.content_lines,
        &block.config,
    );
    let mut out = stdout();
    let buffer = build_frame_buffer(
        &block.frames[0],
        &state,
        &block.content_lines,
        &block.config,
        block.force_plain_logo,
    );
    let _ = out.write_all(buffer.as_bytes());
    move_to_prompt(&mut out, &state);
    let _ = out.flush();

    let Some(pid) = spawn_worker(WORKER_LIVE) else {
        restore_terminal();
        return;
    };
    let files = daemon_files(true);
    write_pid_file(&files.pid, pid);
    write_rows_file(&files.rows, state.block_height);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::info::platform::windows::live::LIVE_MODULES;

    #[test]
    fn test_event_name_is_namespaced_and_null_terminated() {
        let name = event_name(4321);
        let text = String::from_utf16_lossy(&name[..name.len() - 1]);
        assert_eq!(text, "Local\\xfetch-daemon-4321");
        assert_eq!(*name.last().unwrap(), 0);
    }

    #[test]
    fn test_live_modules_default_to_windows_policy() {
        let config = Config::default();
        let modules = live_modules(&config);
        let expected: Vec<String> = LIVE_MODULES.iter().map(|m| m.to_string()).collect();
        assert_eq!(modules, expected);
    }

    #[test]
    fn test_live_modules_override() {
        let config = Config {
            daemon_live_modules: Some(vec!["battery".to_string()]),
            ..Config::default()
        };
        assert_eq!(live_modules(&config), vec!["battery"]);
    }

    #[test]
    fn test_live_config_uses_simple_modules() {
        let config = Config::default();
        let live = live_config(&config, &["cpu".to_string(), "battery".to_string()]);
        assert_eq!(live.modules.len(), 2);
        for (idx, expected) in ["cpu", "battery"].iter().enumerate() {
            match &live.modules[idx] {
                ModuleConfig::Simple(key) => assert_eq!(key, expected),
                _ => panic!("expected a Simple module"),
            }
        }
    }

    #[test]
    fn test_truncate_line_preserves_ansi_and_appends_ellipsis() {
        let line = "\u{1b}[32mabcdef\u{1b}[0m";
        assert_eq!(truncate_line(line, 10), line);
        assert_eq!(truncate_line(line, 3), "\u{1b}[32mabc...");
    }

    #[test]
    fn test_scale_index_handles_scale_and_empty_sources() {
        let state = DaemonState {
            geometry: compute_frame_geometry(&[], 0, &[], &Config::default()),
            block_height: 10,
            term_height: 30,
            scale: 2.0,
        };
        assert_eq!(scale_index(0, 0, &state), usize::MAX);
        assert_eq!(scale_index(0, 10, &state), 0);
        assert_eq!(scale_index(5, 10, &state), 9);
        let unscaled = DaemonState {
            geometry: compute_frame_geometry(&[], 0, &[], &Config::default()),
            block_height: 10,
            term_height: 30,
            scale: 1.0,
        };
        assert_eq!(scale_index(7, 10, &unscaled), 7);
    }

    #[test]
    fn test_build_frame_buffer_pins_rows_and_restores_cursor() {
        let frame = AnimationFrame::new(50, vec!["LOGO".to_string()]);
        let config = Config::default();
        let state = DaemonState {
            geometry: FrameGeometry {
                max_logo_width: 4,
                max_lines: 1,
                available_content_width: 20,
                scroll_margin: 8,
            },
            block_height: 2,
            term_height: 24,
            scale: 1.0,
        };
        let content = vec!["info".to_string()];
        let buffer = build_frame_buffer(&frame, &state, &content, &config, true);
        assert!(buffer.starts_with("\x1b[?25l\x1b7"), "hide + save cursor");
        assert!(
            buffer.contains("\x1b[3;24r"),
            "scroll region starts below the block"
        );
        assert!(buffer.contains("\x1b[1;1H"), "first pinned row");
        assert!(buffer.contains("\x1b[2;1H"), "second pinned row");
        assert!(buffer.contains("LOGO") && buffer.contains("info"));
        assert!(buffer.ends_with("\x1b8"), "cursor restored last");
    }

    #[test]
    fn test_reload_watch_detects_change() {
        let dir = std::env::temp_dir().join(format!("xfetch_win_reload_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.jsonc");
        std::fs::write(&path, "{\"show_colors\": true}").unwrap();

        let config = Config::default();
        let mut watch = ReloadWatch::default();
        let p = path.to_str().unwrap();
        watch.snapshot(&config, Some(p));
        assert!(
            !watch.changed(&config, Some(p)),
            "unchanged file must not reload"
        );

        std::thread::sleep(Duration::from_millis(30));
        std::fs::write(&path, "{\"show_colors\": false}").unwrap();
        assert!(watch.changed(&config, Some(p)), "edited file must reload");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
