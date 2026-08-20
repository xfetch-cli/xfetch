//! Live stats daemon (`daemon_live` in config).
//!
//! Sibling of `ui/daemon.rs` (the animated-logo daemon), kept in its own file
//! so the existing daemon stays untouched. It pins a fetch block at the top of
//! the terminal and re-probes a lightweight subset of modules every
//! `daemon_live_refresh` seconds, re-rendering the block with fresh values
//! (cpu/memory/battery/...). When `logo_animation` is configured, the logo
//! keeps animating while the content updates live; otherwise the logo is
//! static.
//!
//! With `daemon_live_reload` (or `--daemon-live-reload`) the block also
//! watches the config file (and the active theme) and re-applies changes —
//! modules, colors, layout, logo, refresh cadence — without restarting.
//!
//! The engine is platform-agnostic: it reuses the probes from
//! `info::platform` and the per-OS refresh cadence/module policy from
//! `platform/<os>/live.rs`.

use crate::config::{Config, ModuleConfig, config_dir, default_themes_dir, load_config};
use crate::info::Info;
use crate::info::platform::{LivePolicy, live_policy};
use crate::plugins::AnimationFrame;
use crate::ui::layout;
use crate::ui::logo;
use crate::ui::nodes::prepare_render_tree;
use crate::ui::print::{
    DaemonState, LOGO_INFO_GAP, build_daemon_frame_buffer, daemon_move_to_prompt, daemon_prepare,
    restore_terminal,
};
use crossterm::terminal::size;
use std::io::{IsTerminal, Write, stdout};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant, SystemTime};

pub static INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Poll cadence (ms) when the logo is static: we re-render only on resize,
/// refresh or reload ticks, so this only controls how quickly those are
/// noticed.
const POLL_MS: u64 = 100;

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
    config_dir().join("xfetch").join("daemon_live.pid")
}

fn rows_file_path() -> PathBuf {
    config_dir().join("xfetch").join("daemon_live.rows")
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

fn remove_files() {
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

/// Stops a running live daemon (`--daemon-live-stop`). Mirrors
/// `daemon::stop_daemon`, against the live pid file.
pub fn stop_live_daemon() -> bool {
    let Ok(content) = std::fs::read_to_string(pid_file_path()) else {
        return false;
    };
    let Ok(pid) = content.trim().parse::<i32>() else {
        return false;
    };
    if !is_xfetch_process(pid) {
        remove_files();
        return false;
    }
    unsafe {
        libc::kill(pid, libc::SIGTERM);
    }
    remove_files();
    true
}

/// Module keys shown by the live daemon: the user's `daemon_live_modules` or
/// the platform's default live set.
fn live_modules(config: &Config, policy: &LivePolicy) -> Vec<String> {
    match &config.daemon_live_modules {
        Some(list) => list.clone(),
        None => policy.modules.iter().map(|s| s.to_string()).collect(),
    }
}

/// A `Config` whose modules are restricted to the live set. Everything else
/// (logo, colors, layout, daemon sizing) is inherited from the user config.
fn live_config(config: &Config, modules: &[String]) -> Config {
    let mut live = config.clone();
    live.modules = modules
        .iter()
        .map(|k| ModuleConfig::Simple(k.clone()))
        .collect();
    live
}

/// Renders the module tree into content lines, fitting the logo column and
/// terminal width (same math as the regular draw path).
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
    std::fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

/// Snapshot of the config + theme mtimes, used to detect edits.
#[derive(Default)]
struct ReloadWatch {
    config: Option<SystemTime>,
    theme: Option<SystemTime>,
}

impl ReloadWatch {
    fn snapshot(&mut self, config: &Config, config_path: Option<&str>) {
        self.config = config_path.and_then(|p| file_mtime(Path::new(p)));
        self.theme = config
            .theme
            .as_ref()
            .and_then(|t| file_mtime(&theme_file_path(t)));
    }

    fn changed(&self, config: &Config, config_path: Option<&str>) -> bool {
        let cfg_now = config_path.and_then(|p| file_mtime(Path::new(p)));
        let theme_now = config
            .theme
            .as_ref()
            .and_then(|t| file_mtime(&theme_file_path(t)));
        self.config != cfg_now || self.theme != theme_now
    }
}

/// Everything the live loop re-renders: logo frames, module lines and the
/// effective config, plus the hot-reload state.
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
        let policy = live_policy();
        let modules = live_modules(config, &policy);
        let live_cfg = live_config(config, &modules);
        let (ascii_lines, image_printed, ascii_width, _) = logo::get_logo_data(&live_cfg);
        let (frames, force_plain_logo) =
            logo::build_logo_frames(&live_cfg, &ascii_lines, image_printed);
        // The initial render probes the *live* module set (the `Info` from
        // `main` probed the full config modules, which may not cover them —
        // e.g. the datetime probe only runs when the key is requested).
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

    /// Re-applies the config file (and the active theme) when either mtime
    /// changed. Returns `true` when a reload happened.
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
        let policy = live_policy();
        let modules = live_modules(&fresh, &policy);
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

/// Child loop: polls for resize/stop, refreshes the live subset on the cadence
/// set by `daemon_live_refresh` (or the platform policy), hot-reloads the
/// config when enabled, and re-renders the pinned block (animating the logo
/// when `frames.len() > 1`).
fn print_live_output(block: &mut LiveBlock, config_path: Option<&str>) {
    let mut out = stdout();
    let mut state = daemon_prepare(
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
        if INTERRUPTED.load(Ordering::SeqCst) {
            break;
        }

        let cur_size = match size() {
            Ok(s) => s,
            Err(_) => break,
        };
        let resized = Some(cur_size) != last_size;
        if resized {
            last_size = Some(cur_size);
            state = daemon_prepare(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
        }

        let now = Instant::now();
        let policy = live_policy();
        let interval = Duration::from_secs(
            block
                .config
                .daemon_live_refresh
                .unwrap_or(policy.default_refresh_secs)
                .max(1),
        );
        let refreshed = now.duration_since(last_refresh) >= interval;
        if refreshed {
            block.refresh_content();
            state = daemon_prepare(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
            last_refresh = now;
        }

        let reloaded = block.maybe_reload(config_path);
        if reloaded {
            state = daemon_prepare(
                &block.frames,
                block.ascii_width,
                &block.content_lines,
                &block.config,
            );
            frame_index = 0;
            last_refresh = now;
        }

        // With a static logo only re-render on resize/refresh/reload ticks;
        // animated logos re-render on every frame.
        let animated = block.frames.len() > 1;
        if first || animated || resized || refreshed || reloaded {
            let frame = &block.frames[frame_index];
            let buffer = build_daemon_frame_buffer(
                frame,
                &state,
                &block.content_lines,
                &block.config,
                block.force_plain_logo,
            );
            let _ = out.write_all(buffer.as_bytes());
            let _ = out.flush();
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
}

/// Entry point for the live stats daemon (`daemon_live: true` in config, not
/// disabled by `--no-daemon-live`).
///
/// `config_path` is the config file the hot reload watches (when `reload` is
/// enabled); `reload` comes from `daemon_live_reload` or `--daemon-live-reload`.
///
/// Forks to the background the same way `daemon::draw_daemon` does: the parent
/// writes `daemon_live.pid` and returns immediately; the child re-probes and
/// re-renders forever. Stop it with `--daemon-live-stop`.
pub fn draw_live_daemon(_info: &Info, config: &Config, config_path: Option<String>, reload: bool) {
    if !stdout().is_terminal() {
        return;
    }
    // Guard against degenerate terminal sizes (e.g. size 0x0 in a scripted
    // pty), where the frame geometry math would divide by zero.
    if let Ok((w, h)) = size()
        && (w == 0 || h == 0)
    {
        return;
    }

    let mut block = LiveBlock::build(config, config_path.as_deref(), reload);

    stop_live_daemon();
    std::thread::sleep(std::time::Duration::from_millis(50));

    let state: DaemonState = daemon_prepare(
        &block.frames,
        block.ascii_width,
        &block.content_lines,
        &block.config,
    );
    let mut out = stdout();

    let buffer = build_daemon_frame_buffer(
        &block.frames[0],
        &state,
        &block.content_lines,
        &block.config,
        block.force_plain_logo,
    );
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

    print_live_output(&mut block, config_path.as_deref());

    remove_files();
    restore_terminal();
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy() -> LivePolicy {
        LivePolicy {
            modules: &["cpu", "memory"],
            default_refresh_secs: 2,
        }
    }

    fn temp_config(content: &str) -> (PathBuf, PathBuf) {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("xfetch_live_test_{}_{}", std::process::id(), n));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.jsonc");
        std::fs::write(&path, content).unwrap();
        (dir, path)
    }

    #[test]
    fn test_live_modules_defaults_to_policy() {
        let config = Config::default();
        assert_eq!(live_modules(&config, &policy()), vec!["cpu", "memory"]);
    }

    #[test]
    fn test_live_modules_override() {
        let config = Config {
            daemon_live_modules: Some(vec!["battery".to_string()]),
            ..Config::default()
        };
        assert_eq!(live_modules(&config, &policy()), vec!["battery"]);
    }

    #[test]
    fn test_live_config_uses_simple_modules() {
        let config = Config::default();
        let live = live_config(&config, &["cpu".to_string(), "battery".to_string()]);
        assert_eq!(live.modules.len(), 2);
        for (idx, expected) in ["cpu", "battery"].iter().enumerate() {
            match &live.modules[idx] {
                ModuleConfig::Simple(k) => assert_eq!(k, expected),
                _ => panic!("expected a Simple module"),
            }
        }
    }

    #[test]
    fn test_reload_watch_detects_change() {
        let (dir, path) = temp_config("{\"show_colors\": true}");
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

    #[test]
    fn test_maybe_reload_disabled_is_noop() {
        let (dir, path) = temp_config("{\"show_colors\": true}");
        let mut block = LiveBlock::build(&Config::default(), Some(path.to_str().unwrap()), false);
        assert!(!block.maybe_reload(Some(path.to_str().unwrap())));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_maybe_reload_applies_config() {
        let (dir, path) = temp_config("{\"daemon_live_modules\": [\"os\"]}");
        let p = path.to_str().unwrap().to_string();
        let mut block = LiveBlock::build(&Config::default(), Some(&p), true);
        assert!(!block.maybe_reload(Some(&p)));

        std::thread::sleep(Duration::from_millis(30));
        std::fs::write(&path, "{\"daemon_live_modules\": [\"cpu\"]}").unwrap();
        assert!(block.maybe_reload(Some(&p)), "edited config must reload");
        assert_eq!(
            block.config.daemon_live_modules,
            Some(vec!["cpu".to_string()])
        );
        assert_eq!(block.config.modules.len(), 1);
        match &block.config.modules[0] {
            ModuleConfig::Simple(k) => assert_eq!(k, "cpu"),
            _ => panic!("expected a Simple module"),
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
