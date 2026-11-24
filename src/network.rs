use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::protocol::{
    Message, RegistrationResponse, TaskRequest, TaskResponse, WorkerRegistration,
};

pub async fn send_task(
    worker_address: &str,
    task: TaskRequest,
) -> Result<TaskResponse, Box<dyn Error + Send + Sync>> {
    let mut stream = TcpStream::connect(worker_address).await?;
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

pub async fn run_registration_server(
    bind_address: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_address).await?;
    let mut registered_workers: Vec<WorkerRegistration> = Vec::new();

    println!("========================================");
    println!("             FORGE MASTER");
    println!("========================================");
    println!("Worker registration listening at: {bind_address}");
    println!("Waiting for workers to register...");

    loop {
        let (stream, peer) = listener.accept().await?;

        match handle_registration(stream).await {
            Ok((registration, response)) => {
                if response.accepted {
                    // A reconnect from the same worker refreshes its stored profile.
                    if let Some(existing) = registered_workers
                        .iter_mut()
                        .find(|worker| worker.worker_id == registration.worker_id)
                    {
                        *existing = registration.clone();
                        println!("Worker profile refreshed: {}", registration.worker_name);
                    } else {
                        registered_workers.push(registration.clone());
                        println!("Worker registered: {}", registration.worker_name);
                    }

                    println!("Worker ID        : {}", registration.worker_id);
                    println!("Task address     : {}", registration.task_address);
                    println!("Operating system : {}", registration.hardware.operating_system);
                    println!("Processor        : {}", registration.hardware.cpu_model.as_deref().unwrap_or("Not detected"));
                    println!("Logical cores    : {}", registration.hardware.logical_cores);
                    if let Some(ram) = registration.hardware.total_ram_mb {
                        println!("Total memory     : {:.2} GB", ram as f64 / 1024.0);
                    }
                    println!("Connected from   : {peer}");
                    println!("Registered workers: {}", registered_workers.len());
                }

                println!("Registration response: {}", response.message);
            }
            Err(error) => eprintln!("Registration from {peer} failed: {error}"),
        }
    }
}

async fn handle_registration(
    stream: TcpStream,
) -> Result<(WorkerRegistration, RegistrationResponse), Box<dyn Error + Send + Sync>> {
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line).await?;

    if line.is_empty() {
        return Err("Connection closed before registration data arrived".into());
    }

    let registration: WorkerRegistration = serde_json::from_str(&line)?;
    let response = if registration.worker_id.trim().is_empty()
        || registration.worker_name.trim().is_empty()
        || registration.task_address.trim().is_empty()
    {
        RegistrationResponse {
            accepted: false,
            message: "Worker ID, name, and task address are required".into(),
        }
    } else {
        RegistrationResponse {
            accepted: true,
            message: format!("Worker {} registered successfully", registration.worker_name),
        }
    };

    let mut encoded = serde_json::to_vec(&response)?;
    encoded.push(b'\n');
    reader.get_mut().write_all(&encoded).await?;

    Ok((registration, response))
}
