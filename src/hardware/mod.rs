#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "linux")]
mod linux;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCluster {
    pub cluster_id: u32,
    pub core_count: usize,
    pub max_frequency_khz: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareInfo {
    pub operating_system: String,
    pub device_model: Option<String>,
    pub cpu_model: Option<String>,
    pub soc_model: Option<String>,
    pub android_version: Option<String>,
    pub architecture: String,
    pub logical_cores: usize,
    pub cpu_clusters: Vec<CpuCluster>,
    pub total_ram_mb: Option<u64>,
    pub available_ram_mb: Option<u64>,
    pub gpu_egl: Option<String>,
    pub gpu_vulkan: Option<String>,
}

pub fn detect_hardware() -> HardwareInfo {
    #[cfg(target_os = "android")]
    {
        return android::detect();
    }

    #[cfg(target_os = "linux")]
    {
        return linux::detect();
    }

    #[cfg(not(any(target_os = "android", target_os = "linux")))]
    {
        HardwareInfo {
            operating_system: std::env::consts::OS.to_string(),
            device_model: None,
            cpu_model: None,
            soc_model: None,
            android_version: None,
            architecture: std::env::consts::ARCH.to_string(),
            logical_cores: std::thread::available_parallelism()
                .map(|count| count.get())
                .unwrap_or(1),
            cpu_clusters: Vec::new(),
            total_ram_mb: None,
            available_ram_mb: None,
            gpu_egl: None,
            gpu_vulkan: None,
        }
    }
}
