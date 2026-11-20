
use std::fs;
use std::process::Command;

use super::HardwareInfo;

pub(super) fn detect() -> HardwareInfo {
    HardwareInfo {
        device_model: read_property("ro.product.model"),
        soc_model: read_property("ro.soc.model"),
        android_version: read_property("ro.build.version.release"),

        architecture: std::env::consts::ARCH.to_string(),

        logical_cores: std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1),

        total_ram_mb: detect_ram("MemTotal:"),
        available_ram_mb: detect_ram("MemAvailable:"),

        gpu_egl: read_property("ro.hardware.egl"),
        gpu_vulkan: read_property("ro.hardware.vulkan"),
    }
}

fn read_property(property: &str) -> Option<String> {
    let output = Command::new("getprop")
        .arg(property)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let value = String::from_utf8_lossy(&output.stdout)
        .trim()
        .to_string();

    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

fn detect_ram(field: &str) -> Option<u64> {
    let memory_info = fs::read_to_string("/proc/meminfo").ok()?;

    for line in memory_info.lines() {
        if let Some(value) = line.strip_prefix(field) {
            let kilobytes = value
                .split_whitespace()
                .next()?
                .parse::<u64>()
                .ok()?;

            return Some(kilobytes / 1024);
        }
    }

    None
}

