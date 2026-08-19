//! Subprocess machinery shared by the system probes and the
//! plugin/extension runners.
//!
//! All spawns go through `run_cmd_with_stdin_timeout`: pipes are drained on
//! detached reader threads with a bounded grace period (grandchildren that
//! inherited the pipe cannot hang the caller forever), the deadline kills the
//! whole process tree (Windows specifics in
//! `platform/windows/process.rs`), and stdin can be fed for the
//! plugin/extension JSON protocol.
//!
//! Historical note: this used to live in `info/platform/shared/commands.rs`;
//! it moved here so the plugin/extension runners can use it without creating
//! a module cycle (`info` already imports `plugins`). The old path is kept as
//! a re-export for backwards compatibility.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Child, Output, Stdio};
use std::sync::mpsc::{Receiver, channel};
use std::thread;
use std::time::{Duration, Instant};

/// True for WSL-style Windows mounts (`/mnt/c`, `/mnt/d`, ...): execvp stats
/// every PATH entry when spawning, and stat()ing those 9p/drvfs mounts is
/// orders of magnitude slower than native directories — with no `pacman` or
/// `lspci` installed, a failed spawn can burn hundreds of milliseconds.
#[cfg(unix)]
fn is_windows_mount(dir: &Path) -> bool {
    let Some(rest) = dir.strip_prefix("/mnt/").ok() else {
        return false;
    };
    matches!(
        rest.components().next().map(|c| c.as_os_str().to_str()),
        Some(Some(s)) if s.len() == 1 && s.as_bytes()[0].is_ascii_alphabetic()
    )
}

/// Whether `cmd` is reachable through PATH ignoring WSL Windows mounts, the
/// cheap way: spawn would pay the execvp search cost just to fail. Semantics
/// match execvp for the remaining entries, so skipping is always safe.
#[cfg(unix)]
fn binary_reachable(cmd: &str) -> bool {
    match std::env::var_os("PATH") {
        // Let execvp use its own default search when PATH is not set.
        None => true,
        Some(path) => std::env::split_paths(&path)
            .filter(|d| !is_windows_mount(d))
            .any(|d| d.join(cmd).is_file()),
    }
}

/// Runs a command with piped stdout/stderr and a deadline, optionally feeding
/// `stdin_data` (used by the plugin/extension JSON protocol).
///
/// Returns `None` when the command cannot be started, fails, or does not exit
/// within `timeout` (in which case its whole process tree is killed). With
/// `timeout: None` the command may run indefinitely (historical plugin
/// behavior).
///
/// Output is collected on detached reader threads drained with a bounded
/// grace period after the child exits: on Windows, tools such as winget can
/// leave a grandchild holding the pipe, so EOF may never arrive. The wait is
/// bounded instead of hanging the fetch forever.
pub fn run_cmd_with_stdin_timeout(
    cmd: &Path,
    args: &[&str],
    stdin_data: Option<&[u8]>,
    timeout: Option<Duration>,
) -> Option<Output> {
    #[cfg(unix)]
    if let Some(s) = cmd.to_str()
        && !binary_reachable(s)
    {
        return None;
    }
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .stdin(if stdin_data.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    if let (Some(mut stdin), Some(data)) = (child.stdin.take(), stdin_data) {
        // The child may exit early (e.g. invalid request): ignore the write
        // error, the wait loop reports the real status.
        let _ = stdin.write_all(data);
    }

    let out_rx = spawn_pipe_reader(child.stdout.take()?);
    let err_rx = spawn_pipe_reader(child.stderr.take()?);

    let deadline = timeout.map(|t| Instant::now() + t);
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if let Some(d) = deadline
                    && Instant::now() >= d
                {
                    kill_child_tree(&mut child);
                    let _ = drain_bounded(&out_rx);
                    let _ = drain_bounded(&err_rx);
                    return None;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                kill_child_tree(&mut child);
                let _ = drain_bounded(&out_rx);
                let _ = drain_bounded(&err_rx);
                return None;
            }
        }
    };

    let stdout = drain_bounded(&out_rx)?;
    let stderr = drain_bounded(&err_rx)?;
    Some(Output {
        status,
        stdout,
        stderr,
    })
}

/// Convenience wrapper with a mandatory deadline and closed stdin.
pub fn run_cmd_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Option<Output> {
    run_cmd_with_stdin_timeout(Path::new(cmd), args, None, Some(timeout))
}

/// Grace period for a pipe reader to finish after the direct child exits.
/// Grandchildren that inherited the pipe may keep it open; beyond this window
/// the output is discarded (the detached reader thread is leaked) rather than
/// blocking the fetch.
const READER_GRACE: Duration = Duration::from_millis(500);

/// Spawns a thread that reads `pipe` to EOF and sends the buffer on a
/// channel. The reader is detached by design: nothing ever joins it, so a
/// pipe that never reaches EOF cannot block the caller.
fn spawn_pipe_reader<R: Read + Send + 'static>(pipe: R) -> Receiver<Vec<u8>> {
    let (tx, rx) = channel();
    thread::spawn(move || {
        let mut buf = Vec::new();
        let mut pipe = pipe;
        let _ = pipe.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });
    rx
}

/// Receives the reader's buffer, waiting at most `READER_GRACE`. Returns
/// `None` when the pipe is still open (no EOF) after the grace period.
fn drain_bounded(rx: &Receiver<Vec<u8>>) -> Option<Vec<u8>> {
    rx.recv_timeout(READER_GRACE).ok()
}

/// Kills the child. On Windows the whole descendant tree is terminated first
/// (`taskkill /T`, see `platform/windows/process.rs`), since a plain kill
/// would orphan grandchildren that keep the pipe open. On Unix the direct
/// child is killed as before.
fn kill_child_tree(child: &mut Child) {
    #[cfg(target_os = "windows")]
    crate::info::platform::windows::process::kill_process_tree(child.id());
    let _ = child.kill();
    let _ = child.wait();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_timeout_kills_hanging_command() {
        #[cfg(unix)]
        let (cmd, args) = ("sleep", &["10"]);
        // Windows `timeout.exe` exits immediately when stdin is redirected;
        // `ping -n 10` genuinely hangs for ~9 s without needing stdin.
        #[cfg(windows)]
        let (cmd, args) = ("ping", &["-n", "10", "127.0.0.1"]);

        let start = Instant::now();
        let result = run_cmd_with_timeout(cmd, args, Duration::from_secs(1));
        assert!(result.is_none(), "hanging command should time out");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "timeout must not wait for the full hang"
        );
    }

    #[test]
    fn test_cmd_missing_binary_fails_fast() {
        let start = Instant::now();
        let result = run_cmd_with_timeout(
            "nonexistent_cmd_xyz",
            &["--version"],
            Duration::from_secs(2),
        );
        assert!(result.is_none());
        assert!(start.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn test_cmd_fast_command_succeeds() {
        #[cfg(unix)]
        let (cmd, args) = ("true", &[] as &[&str]);
        #[cfg(windows)]
        let (cmd, args) = ("cmd", &["/c", "exit 0"]);

        let output = run_cmd_with_timeout(cmd, args, Duration::from_secs(5));
        assert!(output.is_some());
        assert!(output.unwrap().status.success());
    }

    #[cfg(windows)]
    #[test]
    fn test_grandchild_holding_pipe_does_not_hang() {
        let start = Instant::now();
        // `cmd /c start /b` spawns a grandchild (ping, ~15 s) that inherits
        // the stdout pipe while the direct child exits immediately. The pipe
        // never reaches EOF, so this must return via the bounded drain
        // instead of hanging forever.
        let result = run_cmd_with_timeout(
            "cmd",
            &["/c", "start", "/b", "ping", "-n", "15", "127.0.0.1"],
            Duration::from_secs(5),
        );
        assert!(
            start.elapsed() < Duration::from_secs(8),
            "must not hang when a grandchild holds the pipe"
        );
        assert!(result.is_none(), "unread EOF should yield None");
    }

    #[cfg(windows)]
    #[test]
    fn test_stdin_data_is_written() {
        // `findstr` reads stdin and echoes lines; verify the payload arrives.
        let output = run_cmd_with_stdin_timeout(
            Path::new("findstr"),
            &["hola"],
            Some(b"hola mundo\n"),
            Some(Duration::from_secs(5)),
        );
        assert!(output.is_some());
        let out = output.unwrap();
        assert!(out.status.success());
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("hola mundo"),
            "stdin payload should reach the child"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_stdin_no_timeout_still_drains() {
        // With timeout None a fast command still completes and the bounded
        // drain applies after exit.
        let output = run_cmd_with_stdin_timeout(
            Path::new("cmd"),
            &["/c", "echo ok"],
            Some(b""),
            None,
        );
        assert!(output.is_some());
        assert!(output.unwrap().status.success());
    }

    #[cfg(unix)]
    #[test]
    fn test_is_windows_mount() {
        assert!(is_windows_mount(Path::new("/mnt/c/Users")));
        assert!(is_windows_mount(Path::new("/mnt/d")));
        assert!(!is_windows_mount(Path::new("/mnt/data")));
        assert!(!is_windows_mount(Path::new("/usr/bin")));
        assert!(!is_windows_mount(Path::new("/")));
    }

    #[cfg(unix)]
    #[test]
    fn test_binary_reachable() {
        assert!(binary_reachable("sh"), "sh should be reachable");
        assert!(
            !binary_reachable("definitely_not_a_binary_xyz_123"),
            "missing binaries should fail the pre-check"
        );
    }
}
