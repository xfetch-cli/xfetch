use std::io::Read as _;
use std::process::{Output, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Runs a command with piped stdout/stderr and a deadline.
///
/// Returns `None` when the command cannot be started, fails, or does not exit
/// within `timeout` (in which case it is killed). stdin is closed, so commands
/// can never block waiting for terminal input.
pub fn run_cmd_with_timeout(cmd: &str, args: &[&str], timeout: Duration) -> Option<Output> {
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
}
