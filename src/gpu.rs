use std::error::Error;

/// Android's system Vulkan loader can enumerate the Adreno device, but the
/// current wgpu 28 / ash initialization path panics while loading
/// VK_EXT_debug_utils in Termux. Do not enter that path on Android until the
/// upstream compatibility issue is resolved. This keeps diagnostics safe and
/// leaves the CPU worker usable.
#[cfg(target_os = "android")]
pub fn print_vulkan_adapters() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("FORGE GPU DIAGNOSTICS");
    println!("Platform: Android");
    println!("Native Vulkan: available (verified separately with vulkaninfo)");
    println!("wgpu Vulkan adapter enumeration: temporarily disabled on Android");
    println!("Reason: wgpu 28 / ash initialization currently panics while loading");
    println!("        VK_EXT_debug_utils in the Termux environment.");
    println!("CPU compute remains available. No GPU tasks will be dispatched.");
    Ok(())
}

/// Enumerate Vulkan adapters through wgpu on platforms where its Vulkan
/// initialization path is currently supported by FORGE.
#[cfg(not(target_os = "android"))]
pub fn print_vulkan_adapters() -> Result<(), Box<dyn Error + Send + Sync>> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapters = pollster::block_on(
        instance.enumerate_adapters(wgpu::Backends::VULKAN)
    );

    println!("FORGE GPU DIAGNOSTICS");
    println!("Backend requested: Vulkan");
    println!("Adapters found: {}", adapters.len());

    if adapters.is_empty() {
        println!("No Vulkan GPU adapter was exposed to FORGE.");
        println!("CPU compute remains available.");
        return Ok(());
    }

    for (index, adapter) in adapters.iter().enumerate() {
        let info = adapter.get_info();
        let limits = adapter.limits();

        println!();
        println!("GPU #{}", index + 1);
        println!("  Name       : {}", info.name);
        println!("  Vendor ID  : {:#06x}", info.vendor);
        println!("  Device ID  : {:#06x}", info.device);
        println!("  Type       : {:?}", info.device_type);
        println!("  Driver     : {}", info.driver);
        println!("  Driver info: {}", info.driver_info);
        println!("  Backend    : {:?}", info.backend);
        println!("  Compute workgroup X limit : {}", limits.max_compute_workgroup_size_x);
        println!("  Compute invocations limit: {}", limits.max_compute_invocations_per_workgroup);
        println!("  Max storage buffer        : {} bytes", limits.max_storage_buffer_binding_size);
        println!("  Timestamp queries         : {}", adapter.features().contains(wgpu::Features::TIMESTAMP_QUERY));
    }

    println!();
    println!("This is adapter detection only. GPU task execution is the next stage.");
    Ok(())
}
