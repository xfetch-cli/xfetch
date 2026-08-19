use sysinfo::{Disks, System};

pub fn get_cpu_info(sys: &System) -> String {
    let cpus = sys.cpus();
    if cpus.is_empty() {
        return super::unknown();
    }
    let brand = cpus[0].brand();
    let freq = cpus[0].frequency();
    let cores = cpus.len();
    format!("{} ({}) @ {:.2} GHz", brand, cores, freq as f64 / 1000.0)
}

pub fn get_memory_info(sys: &System) -> String {
    let total = super::b_to_gib(sys.total_memory());
    let used = super::b_to_gib(sys.used_memory());
    let percent = (used / total) * 100.0;
    format!("{:.2} GiB / {:.2} GiB ({:.0}%)", used, total, percent)
}

pub fn get_swap_info(sys: &System) -> String {
    let total = super::b_to_gib(sys.total_swap());
    let used = super::b_to_gib(sys.used_swap());
    if total == 0.0 {
        return "0 B / 0 B (0%)".to_string();
    }
    let percent = (used / total) * 100.0;
    format!("{:.2} GiB / {:.2} GiB ({:.0}%)", used, total, percent)
}

pub fn get_disk_info(disks: &Disks) -> Vec<String> {
    let mut disk_list = Vec::new();
    for disk in disks {
        let total = super::b_to_gib(disk.total_space());
        if total == 0.0 {
            continue;
        }
        let available = super::b_to_gib(disk.available_space());
        let used = total - available;
        let percent = (used / total) * 100.0;
        let fs = disk
            .file_system()
            .to_str()
            .map(|s| s.to_string())
            .unwrap_or_else(super::unknown);
        disk_list.push(format!(
            "{:.2} GiB / {:.2} GiB ({:.0}%) - {}",
            used, total, percent, fs
        ));
    }
    disk_list
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_memory_info() {
        let mem = get_memory_info(&System::new_all());
        assert!(mem.contains("GiB"), "memory should show GiB");
    }

    #[test]
    fn test_get_swap_info() {
        let swap = get_swap_info(&System::new_all());
        assert!(swap.contains("GiB") || swap == "0 B / 0 B (0%)");
    }
}
