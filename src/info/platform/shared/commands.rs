use std::io::Read as _;
#[cfg(unix)]
use std::path::Path;
use std::process::{Output, Stdio};
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

/// Runs a command with piped stdout/stderr and a deadline.
///
/// Returns `None` when the command cannot be started, fails, or does not exit
/// within `timeout` (in which case it is killed). stdin is closed, so commands
/// can never block waiting for terminal input.
pub fn run_cmd_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Option<Output> {
    #[cfg(unix)]
    if !binary_reachable(cmd) {
        return None;
    }
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    let mut stdout = child.stdout.take()?;
    let mut stderr = child.stderr.take()?;
    let out_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let err_reader = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr.read_to_end(&mut buf);
        buf
    });

    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                thread::sleep(Duration::from_millis(20));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };

    let stdout = out_reader.join().ok()?;
    let stderr = err_reader.join().ok()?;
    Some(Output {
        status,
        stdout,
        stderr,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cmd_timeout_kills_hanging_command() {
        #[cfg(unix)]
        let (cmd, args) = ("sleep", &["10"]);
        #[cfg(windows)]
        let (cmd, args) = ("timeout", &["10"]);

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
