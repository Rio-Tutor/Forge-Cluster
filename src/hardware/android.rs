use std::collections::BTreeMap;
use std::fs;
use std::process::Command;

use super::{CpuCluster, HardwareInfo};

pub(super) fn detect() -> HardwareInfo {
    HardwareInfo {
        operating_system: "Android".to_string(),
        device_model: read_property("ro.product.model"),
        cpu_model: read_cpu_model(),
        soc_model: read_property("ro.soc.model"),
        android_version: read_property("ro.build.version.release"),
        architecture: std::env::consts::ARCH.to_string(),
        logical_cores: std::thread::available_parallelism()
            .map(|count| count.get())
            .unwrap_or(1),
        cpu_clusters: detect_cpu_clusters(),
        total_ram_mb: detect_ram("MemTotal:"),
        available_ram_mb: detect_ram("MemAvailable:"),
        gpu_egl: read_property("ro.hardware.egl"),
        gpu_vulkan: read_property("ro.hardware.vulkan"),
    }
}

fn read_cpu_model() -> Option<String> {
    let contents = fs::read_to_string("/proc/cpuinfo").ok()?;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else { continue };
        if matches!(key.trim(), "model name" | "Hardware" | "Processor") {
            let value = value.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn read_property(property: &str) -> Option<String> {
    let output = Command::new("getprop").arg(property).output().ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if value.is_empty() { None } else { Some(value) }
}

fn detect_ram(field: &str) -> Option<u64> {
    let memory_info = fs::read_to_string("/proc/meminfo").ok()?;
    for line in memory_info.lines() {
        if let Some(value) = line.strip_prefix(field) {
            let kilobytes = value.split_whitespace().next()?.parse::<u64>().ok()?;
            return Some(kilobytes / 1024);
        }
    }
    None
}

fn detect_cpu_clusters() -> Vec<CpuCluster> {
    let cpu_directory = "/sys/devices/system/cpu";
    let Ok(entries) = fs::read_dir(cpu_directory) else {
        return Vec::new();
    };
    let mut clusters: BTreeMap<u32, (usize, Option<u64>)> = BTreeMap::new();

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Some(cpu_number) = name.strip_prefix("cpu") else { continue };
        if cpu_number.is_empty() || !cpu_number.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        let topology_path = entry.path().join("topology/cluster_id");
        let Ok(cluster_value) = fs::read_to_string(topology_path) else { continue };
        let Ok(cluster_id) = cluster_value.trim().parse::<u32>() else { continue };

        let frequency = fs::read_to_string(entry.path().join("cpufreq/cpuinfo_max_freq"))
            .ok()
            .and_then(|value| value.trim().parse::<u64>().ok());

        clusters.entry(cluster_id)
            .and_modify(|(count, max_frequency)| {
                *count += 1;
                if let Some(new_frequency) = frequency {
                    *max_frequency = Some(max_frequency.map_or(new_frequency, |current| current.max(new_frequency)));
                }
            })
            .or_insert((1, frequency));
    }

    clusters.into_iter()
        .map(|(cluster_id, (core_count, max_frequency_khz))| CpuCluster {
            cluster_id,
            core_count,
            max_frequency_khz,
        })
        .collect()
}
