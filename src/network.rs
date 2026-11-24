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
    protocol::{
        Message, RegistrationResponse, TaskRequest, TaskResponse, WorkerRegistration,
    },
    registry::DeviceRegistry,
};

pub async fn send_task(
    worker_address: &str,
    task: TaskRequest,
) -> Result<TaskResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(worker_address).await?;
    let message = Message::RunTask(task);

    let mut encoded = serde_json::to_vec(&message)?;
    encoded.push(b'\\n');
    stream.write_all(&encoded).await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err("Worker closed the connection without a response".into());
    }

    match serde_json::from_str::<Message>(&response_line)? {
        Message::TaskResult(response) => Ok(response),
        Message::RunTask(_) => Err("Worker sent an unexpected task request".into()),
    }
}

pub async fn register_worker(
    master_address: &str,
    registration: &WorkerRegistration,
) -> Result<RegistrationResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(master_address).await?;
    let mut encoded = serde_json::to_vec(registration)?;
    encoded.push(b'\\n');
    stream.write_all(&encoded).await?;

    let mut reader = BufReader::new(stream);
    let mut response_line = String::new();
    reader.read_line(&mut response_line).await?;

    if response_line.is_empty() {
        return Err("Master closed the connection without a registration response".into());
    }

    Ok(serde_json::from_str(&response_line)?)
}

pub async fn run_registration_server(
    bind_address: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_address).await?;
    let registry = Arc::new(Mutex::new(DeviceRegistry::new()));

    println!("========================================");
    println!("             FORGE MASTER");
    println!("========================================");
    println!("Worker registration listening at: {bind_address}");
    println!("Waiting for workers to register...");

    loop {
        let (stream, peer) = listener.accept().await?;
        let registry = Arc::clone(&registry);

        tokio::spawn(async move {
            if let Err(error) = handle_registration(stream, registry, &peer.to_string()).await {
                eprintln!("Registration from {peer} failed: {error}");
            }
        });
    }
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

    let registration: WorkerRegistration = serde_json::from_str(&line)?;
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
    encoded.push(b'\\n');
    reader.get_mut().write_all(&encoded).await?;

    Ok(())
}
