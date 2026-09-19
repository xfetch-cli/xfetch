use std::mem::zeroed;

use windows_sys::Win32::Foundation::SYSTEMTIME;
use windows_sys::Win32::System::SystemInformation::GetLocalTime;

/// Local date/time read straight from the Win32 API. The previous
/// PowerShell probe cost ~200 ms per fetch just to start the interpreter;
/// `GetLocalTime` returns the same `yyyy-MM-dd HH:mm:ss` in microseconds.
pub fn get_datetime_info() -> String {
    let mut time: SYSTEMTIME = unsafe { zeroed() };
    unsafe { GetLocalTime(&mut time) };
    if time.wYear == 0 {
        return crate::info::unknown();
    }
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        time.wYear, time.wMonth, time.wDay, time.wHour, time.wMinute, time.wSecond
    )
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

    #[test]
    fn test_get_datetime_info_is_zero_padded() {
        let dt = get_datetime_info();
        let (date, clock) = dt.split_once(' ').expect("date and time split");
        assert_eq!(date.len(), 10, "date '{}' should be YYYY-MM-DD", date);
        assert_eq!(clock.len(), 8, "clock '{}' should be HH:MM:SS", clock);
    }
}
