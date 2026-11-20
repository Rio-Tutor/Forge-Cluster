mod android;

#[derive(Debug)]
pub struct CpuCluster {
    pub cluster_id: u32,
    pub core_count: usize,
    pub max_frequency_khz: Option<u64>,
}


#[derive(Debug)]
pub struct HardwareInfo {
    pub device_model: Option<String>,
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
    android::detect()
}
