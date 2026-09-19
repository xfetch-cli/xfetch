//! Native Windows CPU probe.
//!
//! sysinfo's CPU refresh opens PDH counters for every logical processor
//! (`\Processor(n)\% Idle Time`) even when only the brand and frequency are
//! read, which costs ~270 ms per fetch on a 16-thread machine. xfetch only
//! renders brand, logical count and current clock, so those values are read
//! directly from the Win32 API (registry + `CallNtPowerInformation`) in under
//! a millisecond.

use std::ffi::c_void;

use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Power::{
    CallNtPowerInformation, PROCESSOR_POWER_INFORMATION, ProcessorInformation,
};
use windows_sys::Win32::System::Registry::{
    HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegGetValueW,
};
use windows_sys::Win32::System::Threading::{ALL_PROCESSOR_GROUPS, GetActiveProcessorCount};

const CPU_KEY: &str = r"HARDWARE\DESCRIPTION\System\CentralProcessor\0";
const BRAND_VALUE: &str = "ProcessorNameString";
const FREQ_VALUE: &str = "~MHz";

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn reg_string(subkey: &str, value: &str) -> Option<String> {
    let subkey = to_wide(subkey);
    let value = to_wide(value);
    let mut size = 0u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    let mut buf = vec![0u16; (size as usize).div_ceil(2)];
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buf.as_mut_ptr().cast::<c_void>(),
            &mut size,
        )
    };
    if status != ERROR_SUCCESS {
        return None;
    }
    // `size` includes the terminating NUL.
    let len = (size as usize / 2).saturating_sub(1).min(buf.len());
    let name = String::from_utf16_lossy(&buf[..len]);
    let name = name.trim().to_string();
    (!name.is_empty()).then_some(name)
}

fn reg_dword(subkey: &str, value: &str) -> Option<u32> {
    let subkey = to_wide(subkey);
    let value = to_wide(value);
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&mut data as *mut u32).cast::<c_void>(),
            &mut size,
        )
    };
    (status == ERROR_SUCCESS).then_some(data)
}

/// Current clock of the first logical processor, matching sysinfo's
/// `frequency()` (which also samples `CallNtPowerInformation`).
fn current_mhz(count: usize) -> Option<u32> {
    if count == 0 {
        return None;
    }
    let mut infos: Vec<PROCESSOR_POWER_INFORMATION> = Vec::with_capacity(count);
    let size = (count * std::mem::size_of::<PROCESSOR_POWER_INFORMATION>()) as u32;
    let status = unsafe {
        CallNtPowerInformation(
            ProcessorInformation,
            std::ptr::null(),
            0,
            infos.as_mut_ptr().cast::<c_void>(),
            size,
        )
    };
    if status != 0 {
        return None;
    }
    unsafe { infos.set_len(count) };
    infos
        .first()
        .map(|info| info.CurrentMhz)
        .filter(|mhz| *mhz > 0)
}

pub fn get_cpu_info() -> String {
    let cores = unsafe { GetActiveProcessorCount(ALL_PROCESSOR_GROUPS) } as usize;
    let cores = if cores == 0 {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    } else {
        cores
    };
    let brand = reg_string(CPU_KEY, BRAND_VALUE).unwrap_or_else(crate::info::unknown);
    let mhz = current_mhz(cores)
        .or_else(|| reg_dword(CPU_KEY, FREQ_VALUE))
        .unwrap_or(0);
    format!("{} ({}) @ {:.2} GHz", brand, cores, mhz as f64 / 1000.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_cpu_info_is_formatted() {
        let cpu = get_cpu_info();
        assert!(cpu.contains("GHz"), "cpu '{}' should show GHz", cpu);
        assert!(
            cpu.contains('('),
            "cpu '{}' should show the core count",
            cpu
        );
    }

    #[test]
    fn test_registry_brand_is_available() {
        let brand = reg_string(CPU_KEY, BRAND_VALUE);
        assert!(brand.is_some(), "ProcessorNameString should be readable");
    }
}
