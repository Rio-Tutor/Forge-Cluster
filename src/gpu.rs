use std::error::Error;

#[cfg(target_os = "android")]
pub fn print_vulkan_adapters() -> Result<(), Box<dyn Error + Send + Sync>> {
    println!("FORGE GPU DIAGNOSTICS");
    println!("Platform: Android");
    println!("Native Vulkan: available (verified separately with vulkaninfo)");
    println!("wgpu Vulkan adapter enumeration: temporarily disabled on Android");
    println!("Reason: wgpu 28 / ash initialization currently panics while loading");
    println!("        VK_EXT_debug_utils in the Termux environment.");
    println!("CPU compute remains available. GPU compute is not enabled on Android yet.");
    Ok(())
}

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
    println!("Adapter detection only. Run 'forge gpu-test' to execute a real GPU compute shader.");
    Ok(())
}

/// Execute a small vector-addition compute shader and verify the read-back
/// values. This is a GPU execution smoke test, not an AI inference benchmark.
#[cfg(not(target_os = "android"))]
pub fn run_compute_smoke_test() -> Result<(), Box<dyn Error + Send + Sync>> {
    pollster::block_on(run_compute_smoke_test_async())
}

#[cfg(not(target_os = "android"))]
async fn run_compute_smoke_test_async() -> Result<(), Box<dyn Error + Send + Sync>> {
    use std::sync::mpsc;
    use wgpu::util::DeviceExt;

    const ELEMENTS: u32 = 1024;
    const WORKGROUP_SIZE: u32 = 64;

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            force_fallback_adapter: false,
            compatible_surface: None,
        })
        .await?;

    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await?;

    let mut input_values = Vec::with_capacity((ELEMENTS * 2) as usize);
    for index in 0..ELEMENTS {
        input_values.push(index);
        input_values.push(index * 3);
    }

    let mut input_bytes = Vec::with_capacity(input_values.len() * 4);
    for value in &input_values {
        input_bytes.extend_from_slice(&value.to_le_bytes());
    }

    let input_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("FORGE vector-add input"),
        contents: &input_bytes,
        usage: wgpu::BufferUsages::STORAGE,
    });

    let output_size = (ELEMENTS * 4) as u64;
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("FORGE vector-add output"),
        size: output_size,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("FORGE vector-add readback"),
        size: output_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("FORGE vector-add WGSL"),
        source: wgpu::ShaderSource::Wgsl(
            r#"
                @group(0) @binding(0)
                var<storage, read> input_values: array<u32>;

                @group(0) @binding(1)
                var<storage, read_write> output_values: array<u32>;

                @compute @workgroup_size(64)
                fn main(@builtin(global_invocation_id) id: vec3<u32>) {
                    let index = id.x;
                    if (index < 1024u) {
                        output_values[index] =
                            input_values[index * 2u] + input_values[index * 2u + 1u];
                    }
                }
            "#.into(),
        ),
    });

    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("FORGE vector-add bind-group layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });

    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("FORGE vector-add bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output_buffer.as_entire_binding(),
            },
        ],
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("FORGE vector-add pipeline layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("FORGE vector-add compute pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: Some("main"),
        compilation_options: Default::default(),
        cache: None,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("FORGE vector-add encoder"),
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("FORGE vector-add compute pass"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        pass.dispatch_workgroups(ELEMENTS.div_ceil(WORKGROUP_SIZE), 1, 1);
    }

    encoder.copy_buffer_to_buffer(&output_buffer, 0, &readback_buffer, 0, output_size);
    queue.submit([encoder.finish()]);

    let slice = readback_buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    device.poll(wgpu::PollType::wait_indefinitely())?;
    receiver.recv()??;

    let mapped = slice.get_mapped_range();
    let results: Vec<u32> = mapped
        .chunks_exact(4)
        .map(|bytes| u32::from_le_bytes(bytes.try_into().expect("four-byte value")))
        .collect();
    drop(mapped);
    readback_buffer.unmap();

    for (index, result) in results.iter().enumerate() {
        let expected = (index as u32) * 4;
        if *result != expected {
            return Err(format!(
                "GPU verification failed at index {index}: expected {expected}, got {result}"
            ).into());
        }
    }

    println!("FORGE GPU COMPUTE TEST");
    println!("Adapter : {}", info.name);
    println!("Backend : {:?}", info.backend);
    println!("Elements: {ELEMENTS}");
    println!("Shader  : vector addition (WGSL)");
    println!("Result  : all values verified successfully");

    Ok(())
}

#[cfg(target_os = "android")]
pub fn run_compute_smoke_test() -> Result<(), Box<dyn Error + Send + Sync>> {
    Err(
        "GPU compute is not enabled on Android yet. The current wgpu 28 Vulkan initialization workaround still disables this path."
            .into()
    )
}
