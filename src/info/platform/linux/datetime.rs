use std::time::Duration;

use crate::info::platform::shared::commands::run_cmd_with_timeout;

const DATE_CMD: &str = "date";
const DATE_FMT: &str = "+%Y-%m-%d %H:%M:%S";
const DATE_TIMEOUT: Duration = Duration::from_secs(10);

pub fn get_datetime_info() -> String {
    if let Some(output) = run_cmd_with_timeout(DATE_CMD, &[DATE_FMT], DATE_TIMEOUT) {
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
