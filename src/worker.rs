use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::{
    compute,
    discovery,
    network,
    protocol::{Message, TaskResponse, WorkerRegistration},
};

pub async fn run_worker(
    bind_address: &str,
    master_address: Option<&str>,
    advertised_address: Option<&str>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_address).await?;
    let hardware = crate::hardware::detect_hardware();

    let (master, advertised) = match (master_address, advertised_address) {
        (Some(master), Some(advertised)) => (master.to_string(), advertised.to_string()),
        (None, None) => {
            let worker_port = listener.local_addr()?.port();
            let (master, advertised) = discovery::discover_master(worker_port).await?;
            (master.to_string(), advertised.to_string())
        }
        _ => return Err("Provide both master and advertised worker addresses, or neither for automatic discovery".into()),
    };

    let worker_name = hardware
        .device_model
        .clone()
        .or_else(|| hardware.cpu_model.clone())
        .unwrap_or_else(|| "FORGE Worker".to_string());
    let worker_id = format!("{}-{}-{}", hardware.architecture, worker_name.replace(' ', "_"), advertised);
    let registration = WorkerRegistration {
        worker_id: worker_id.clone(),
        worker_name,
        task_address: advertised.to_string(),
        hardware: hardware.clone(),
    };

    println!("Registering with FORGE master at {master}...");
    let response = network::register_worker(&master, &registration).await?;
    if !response.accepted {
        return Err(format!("Master rejected worker registration: {}", response.message).into());
    }
    println!("Registration successful: {}", response.message);

    let heartbeat_master = master.clone();
    let heartbeat_worker = worker_id.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            if let Err(error) = network::send_heartbeat(&heartbeat_master, &heartbeat_worker).await {
                eprintln!("Heartbeat failed; attempting to reconnect: {error}");
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    if network::register_worker(&heartbeat_master, &registration).await.map(|r| r.accepted).unwrap_or(false) {
                        println!("Reconnected to FORGE master.");
                        break;
                    }
                }
            }
        }
    });

    println!("========================================");
    println!("          FORGE WORKER REPORT");
    println!("========================================");
    println!("Operating system : {}", hardware.operating_system);
    println!("Device           : {}", hardware.device_model.as_deref().unwrap_or("Not detected"));
    println!("Processor        : {}", hardware.cpu_model.as_deref().unwrap_or("Not detected"));
    println!("Architecture     : {}", hardware.architecture);
    println!("Logical cores    : {}", hardware.logical_cores);

    if let Some(total_ram) = hardware.total_ram_mb {
        println!("Total memory     : {:.2} GB", total_ram as f64 / 1024.0);
    } else {
        println!("Total memory     : Not detected");
    }

    if let Some(available_ram) = hardware.available_ram_mb {
        println!("Available memory : {:.2} GB", available_ram as f64 / 1024.0);
    } else {
        println!("Available memory : Not detected");
    }

    if let Some(soc) = hardware.soc_model.as_deref() {
        println!("SoC              : {soc}");
    }
    if let Some(version) = hardware.android_version.as_deref() {
        println!("Android version  : {version}");
    }
    if let Some(gpu) = hardware.gpu_egl.as_deref() {
        println!("GPU (EGL)        : {gpu}");
    }
    if let Some(gpu) = hardware.gpu_vulkan.as_deref() {
        println!("GPU (Vulkan)     : {gpu}");
    }

    if hardware.cpu_clusters.is_empty() {
        println!("CPU clusters     : Not available");
    } else {
        println!("CPU clusters:");
        for cluster in &hardware.cpu_clusters {
            let frequency = cluster.max_frequency_khz
                .map(|value| format!("{:.2} GHz", value as f64 / 1_000_000.0))
                .unwrap_or_else(|| "frequency unknown".to_string());
            println!("  - Cluster {}: {} cores, {}", cluster.cluster_id, cluster.core_count, frequency);
        }
    }

    println!("========================================");
    println!("Worker listening at: {bind_address}");
    println!("Advertised address : {advertised}");

    loop {
        let (stream, peer) = listener.accept().await?;
        println!("Accepted connection from {peer}");

        tokio::spawn(async move {
            if let Err(error) = handle_connection(stream).await {
                eprintln!("Worker request failed: {error}");
            }
        });
    }
}

async fn handle_connection(
    stream: TcpStream,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();

    reader.read_line(&mut request_line).await?;

    if request_line.is_empty() {
        return Ok(());
    }

    let message: Message = serde_json::from_str(&request_line)?;

    let response = match message {
        Message::RunTask(task) => {
            let result = compute::execute(task.operation)
                .map_err(std::io::Error::other)?;

            Message::TaskResult(TaskResponse {
                task_id: task.task_id,
                result,
                worker: "FORGE CPU Worker".to_string(),
            })
        }

        Message::TaskResult(_) => {
            return Err(
                "Worker received a result message instead of a task".into()
            );
        }
    };

    let mut encoded = serde_json::to_vec(&response)?;
    encoded.push(b'\n');

    reader.get_mut().write_all(&encoded).await?;

    Ok(())
}
