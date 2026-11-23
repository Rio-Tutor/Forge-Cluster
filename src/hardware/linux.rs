use std::fs;

use super::HardwareInfo;

pub(super) fn detect() -> HardwareInfo {
    HardwareInfo {
        operating_system: format!("Linux ({})", read_os_name().unwrap_or_else(|| "unknown distribution".to_string())),
        device_model: read_dmi_value("/sys/class/dmi/id/product_name"),
        cpu_model: read_cpu_model(),
        soc_model: None,
        android_version: None,
        architecture: std::env::consts::ARCH.to_string(),
        logical_cores: std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1),
        cpu_clusters: Vec::new(),
        total_ram_mb: detect_ram("MemTotal:"),
        available_ram_mb: detect_ram("MemAvailable:"),
        gpu_egl: None,
        gpu_vulkan: None,
    }
}

fn read_os_name() -> Option<String> {
    let contents = fs::read_to_string("/etc/os-release").ok()?;
    contents.lines()
        .find_map(|line| line.strip_prefix("PRETTY_NAME="))
        .map(|value| value.trim_matches('"').to_string())
}

fn read_dmi_value(path: &str) -> Option<String> {
    let value = fs::read_to_string(path).ok()?.trim().to_string();
    if value.is_empty() || value == "To Be Filled By O.E.M." {
        None
    } else {
        Some(value)
    }
}

fn read_cpu_model() -> Option<String> {
    let contents = fs::read_to_string("/proc/cpuinfo").ok()?;
    contents.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if matches!(key.trim(), "model name" | "Hardware" | "Processor") {
            let value = value.trim();
            if !value.is_empty() { Some(value.to_string()) } else { None }
        } else {
            None
        }
    })
}

fn detect_ram(field: &str) -> Option<u64> {
    let contents = fs::read_to_string("/proc/meminfo").ok()?;
    contents.lines().find_map(|line| {
        let value = line.strip_prefix(field)?;
        let kilobytes = value.split_whitespace().next()?.parse::<u64>().ok()?;
        Some(kilobytes / 1024)
    })
}
