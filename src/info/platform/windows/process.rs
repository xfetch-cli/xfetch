//! Process-tree handling on Windows.
//!
//! `std::process::Child::kill()` performs a plain `TerminateProcess` on the
//! direct child only. Windows tools used by the probes (winget's COM server,
//! PowerShell helpers) can spawn grandchildren that survive that kill and
//! keep inherited handles (e.g. our stdout pipe) open, which would block the
//! pipe readers forever. `taskkill /T` walks and terminates the whole
//! descendant tree.

use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

const TASKKILL_TIMEOUT: Duration = Duration::from_secs(5);

/// Terminates `pid` and all its descendants. Best effort: does nothing when
/// `taskkill` cannot be started (it is a standard system utility).
pub fn kill_process_tree(pid: u32) {
    let pid_str = pid.to_string();
    let mut child = match Command::new("taskkill")
        .args(["/PID", pid_str.as_str(), "/T", "/F"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => return,
    };

    let deadline = Instant::now() + TASKKILL_TIMEOUT;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if Instant::now() >= deadline => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            _ => thread::sleep(Duration::from_millis(20)),
        }
    }
}
