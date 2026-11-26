use std::{
    error::Error,
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

use crate::{
    discovery,
    protocol::{
        Message, RegistrationResponse, TaskRequest, TaskResponse, WorkerHeartbeat, WorkerRegistration,
    },
    registry::DeviceRegistry,
    scheduler,
    compute,
    hardware,
    worker,
};

pub async fn send_task(
    worker_address: &str,
    task: TaskRequest,
) -> Result<TaskResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(worker_address).await?;
    let expected_task_id = task.task_id;
    let message = Message::RunTask(task);

    let mut encoded = serde_json::to_vec(&message)?;
    encoded.push(b'\n');
    stream.write_all(&encoded).await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err("Worker closed the connection without a response".into());
    }

    match serde_json::from_str::<Message>(&response_line)? {
        Message::TaskResult(response) => {
            if response.task_id != expected_task_id {
                return Err(format!("Task ID mismatch: expected {}, received {}", expected_task_id, response.task_id).into());
            }
            Ok(response)
        }
        Message::TaskError(failure) => {
            if failure.task_id != expected_task_id {
                return Err(format!("Task ID mismatch in failure: expected {}, received {}", expected_task_id, failure.task_id).into());
            }
            Err(format!("Worker {} rejected task {}: {}", failure.worker, failure.task_id, failure.error).into())
        }
        Message::RunTask(_) => Err("Worker sent an unexpected task request".into()),
    }
}

pub async fn send_heartbeat(
    master_address: &str,
    worker_id: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(master_address).await?;
    let heartbeat = WorkerHeartbeat::new(worker_id);
    let mut encoded = serde_json::to_vec(&heartbeat)?;
    encoded.push(b'\n');
    stream.write_all(&encoded).await?;
    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;
    if response_line.is_empty() {
        return Err("Master closed the connection without a heartbeat response".into());
    }
    let response: RegistrationResponse = serde_json::from_str(&response_line)?;
    if !response.accepted {
        return Err(response.message.into());
    }
    Ok(())
}

pub async fn register_worker(
    master_address: &str,
    registration: &WorkerRegistration,
) -> Result<RegistrationResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(master_address).await?;
    let mut encoded = serde_json::to_vec(registration)?;
    encoded.push(b'\n');
    stream.write_all(&encoded).await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err("Master closed the connection without a registration response".into());
    }

    Ok(serde_json::from_str(&response_line)?)
}

/// Run the master registration listener and interactive scheduler console.
pub async fn run_master(
    bind_address: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_address).await?;
    let master_port: u16 = bind_address.rsplit_once(':')
        .and_then(|(_, port)| port.parse().ok())
        .ok_or("Master bind address must include a valid TCP port")?;
    tokio::spawn(async move {
        if let Err(error) = discovery::broadcast_master(master_port).await {
            eprintln!("FORGE discovery service stopped: {error}");
        }
    });
    // The master is also a compute node. It runs the same task server as
    // ordinary workers and registers its own hardware with the scheduler.
    let master_task_listener = TcpListener::bind("0.0.0.0:7878").await?;
    let master_task_socket = master_task_listener.local_addr()?;
    let master_task_port = master_task_socket.port();
    println!("Master compute listener listening at: {master_task_socket}");
    let master_lan_ip = discovery::local_ipv4_address().await?;
    let master_task_address = format!("{master_lan_ip}:{master_task_port}");
    tokio::spawn(async move {
        if let Err(error) = worker::run_task_server(master_task_listener).await {
            eprintln!("Master compute server stopped: {error}");
        }
    });

    let hardware_info = hardware::detect_hardware();
    println!("Benchmarking master CPU...");
    let benchmark_score = compute::benchmark_cpu(500_000);
    let master_registration = WorkerRegistration {
        worker_id: format!("master-{}-{master_task_address}", hardware_info.architecture),
        worker_name: format!("FORGE Master ({})", hardware_info.device_model.as_deref().or(hardware_info.cpu_model.as_deref()).unwrap_or("This machine")),
        task_address: master_task_address.clone(),
        benchmark_score,
        hardware: hardware_info,
    };

    let registry = Arc::new(Mutex::new(DeviceRegistry::new()));
    let master_worker_id = master_registration.worker_id.clone();
    {
        let mut workers = registry.lock().await;
        workers.register_or_update(master_registration);
    }
    println!("Master compute node registered at {master_task_address}");
    println!("Master CPU benchmark: {benchmark_score} iterations/sec");

    // Keep the local master entry alive using the same liveness mechanism
    // used for remote workers.
    let master_heartbeat_registry = Arc::clone(&registry);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            master_heartbeat_registry.lock().await.heartbeat(&master_worker_id);
        }
    });

    let cleanup_registry = Arc::clone(&registry);
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
        loop {
            interval.tick().await;
            let mut registry = cleanup_registry.lock().await;
            for worker_id in registry.remove_stale(std::time::Duration::from_secs(20)) {
                println!("Worker timed out and was removed: {worker_id}");
            }
        }
    });

    println!("========================================");
    println!("             FORGE MASTER");
    println!("========================================");
    println!("Worker registration listening at: {bind_address}");
    println!("LAN discovery beacon: UDP 7877");
    println!();
    println!("Commands:");
    println!("  list");
    println!("  sum-squares <limit> <chunk-size>");
    println!("  help");
    println!("  quit");
    println!();
    println!("Master is contributing compute. Waiting for remote workers...");

    let stdin = tokio::io::stdin();
    let mut commands = BufReader::new(stdin).lines();

    loop {
        tokio::select! {
            accepted = listener.accept() => {
                let (stream, peer) = accepted?;
                let registry = Arc::clone(&registry);

                tokio::spawn(async move {
                    if let Err(error) = handle_registration(
                        stream,
                        registry,
                        &peer.to_string()
                    ).await {
                        eprintln!("Registration from {peer} failed: {error}");
                    }
                });
            }

            command = commands.next_line() => {
                match command? {
                    Some(line) => {
                        let mut parts = line.split_whitespace();

                        match parts.next() {
                            Some("list") => {
                                let registry = registry.lock().await;
                                let workers = registry.get_all();

                                if workers.is_empty() {
                                    println!("No workers are registered yet.");
                                } else {
                                    println!("REGISTERED WORKERS: {}", workers.len());
                                    for (index, worker) in workers.iter().enumerate() {
                                        println!(
                                            "{}. {} | {} | {}",
                                            index + 1,
                                            worker.worker_name,
                                            worker.task_address,
                                            worker.hardware.cpu_model.as_deref().unwrap_or("CPU unknown")
                                        );
                                        println!("   CPU benchmark: {} iterations/sec", worker.benchmark_score);
                                    }
                                }
                            }

                            Some("sum-squares") => {
                                let limit = parts.next().and_then(|value| value.parse::<i64>().ok());
                                let chunk_size = parts.next().and_then(|value| value.parse::<i64>().ok());

                                match (limit, chunk_size) {
                                    (Some(limit), Some(chunk_size)) => {
                                        let workers = {
                                            let registry = registry.lock().await;
                                            registry.get_all()
                                        };

                                        if workers.is_empty() {
                                            println!("No compute nodes are registered.");
                                        } else if let Err(error) = scheduler::run_sum_squares(
                                            workers,
                                            limit,
                                            chunk_size
                                        ).await {
                                            eprintln!("Computation failed: {error}");
                                        }
                                    }
                                    _ => println!("Usage: sum-squares <positive-limit> <positive-chunk-size>"),
                                }
                            }

                            Some("help") => {
                                println!("Available commands:");
                                println!("  list");
                                println!("  sum-squares <limit> <chunk-size>");
                                println!("  help");
                                println!("  quit");
                            }

                            Some("quit") | Some("exit") => {
                                println!("Shutting down FORGE master.");
                                break;
                            }

                            Some(other) => println!("Unknown command: {other}. Type 'help'."),
                            None => {}
                        }
                    }
                    None => break,
                }
            }
        }
    }

    Ok(())
}

async fn handle_registration(
    stream: TcpStream,
    registry: Arc<Mutex<DeviceRegistry>>,
    peer: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.is_empty() {
        return Err("Connection closed before registration data arrived".into());
    }

    let value: serde_json::Value = serde_json::from_str(&line)?;
    if value.get("kind").and_then(|v| v.as_str()) == Some("heartbeat") {
        let heartbeat: WorkerHeartbeat = serde_json::from_value(value)?;
        let mut registry = registry.lock().await;
        let known = registry.heartbeat(&heartbeat.worker_id);
        drop(registry);
        let response = RegistrationResponse {
            accepted: known,
            message: if known { "Heartbeat accepted".into() } else { "Worker must register again".into() },
        };
        let mut encoded = serde_json::to_vec(&response)?;
        encoded.push(b'\n');
        reader.get_mut().write_all(&encoded).await?;
        return Ok(());
    }

    let registration: WorkerRegistration = serde_json::from_value(value)?;
    let validation_error = if registration.worker_id.trim().is_empty()
        || registration.worker_name.trim().is_empty()
        || registration.task_address.trim().is_empty()
    {
        Some("Worker ID, name, and task address are required")
    } else {
        None
    };

    let response = if let Some(message) = validation_error {
        RegistrationResponse {
            accepted: false,
            message: message.into(),
        }
    } else {
        let mut registry = registry.lock().await;
        registry.register_or_update(registration.clone());
        let count = registry.count();

        println!();
        println!("========================================");
        println!("          WORKER REGISTRATION");
        println!("========================================");
        println!("Worker           : {}", registration.worker_name);
        println!("Worker ID        : {}", registration.worker_id);
        println!("Task address     : {}", registration.task_address);
        println!("Operating system : {}", registration.hardware.operating_system);
        println!(
            "Processor        : {}",
            registration.hardware.cpu_model.as_deref().unwrap_or("Not detected")
        );
        println!("Architecture     : {}", registration.hardware.architecture);
        println!("Logical cores    : {}", registration.hardware.logical_cores);

        if let Some(ram) = registration.hardware.total_ram_mb {
            println!("Total memory     : {:.2} GB", ram as f64 / 1024.0);
        } else {
            println!("Total memory     : Not detected");
        }

        println!("Connected from   : {peer}");
        println!("Registered workers: {count}");
        println!("========================================");

        RegistrationResponse {
            accepted: true,
            message: format!(
                "Worker {} registered successfully",
                registration.worker_name
            ),
        }
    };

    let mut encoded = serde_json::to_vec(&response)?;
    encoded.push(b'\n');
    reader.get_mut().write_all(&encoded).await?;

    Ok(())
}
