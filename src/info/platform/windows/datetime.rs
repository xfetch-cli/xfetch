use std::time::Duration;

use crate::info::platform::shared::commands::run_cmd_with_timeout;

const POWERSHELL_CMD: &str = "powershell";
/// `-NoProfile` keeps the user's profile script out of the probe output;
/// `-NonInteractive` forbids prompts. `[Console]::OutputEncoding=UTF8` fixes
/// the OEM codepage PowerShell 5.1 uses for redirected output.
const DATE_FMT: &str =
    "[Console]::OutputEncoding=[Text.Encoding]::UTF8; Get-Date -Format 'yyyy-MM-dd HH:mm:ss'";
const DATE_TIMEOUT: Duration = Duration::from_secs(10);

pub fn get_datetime_info() -> String {
    if let Some(output) = run_cmd_with_timeout(
        POWERSHELL_CMD,
        &["-NoProfile", "-NonInteractive", "-Command", DATE_FMT],
        DATE_TIMEOUT,
    )
    .filter(|o| o.status.success())
    {
        return String::from_utf8_lossy(&output.stdout).trim().to_string();
    }
    crate::info::unknown()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_datetime_info() {
        let dt = get_datetime_info();
        assert!(
            dt.len() >= 10,
            "datetime should be at least YYYY-MM-DD: got '{}'",
            dt
        );
    }
}
