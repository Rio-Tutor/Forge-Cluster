use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

use crate::{
    compute,
    protocol::{Message, TaskResponse},
};

pub async fn run_worker(
    bind_address: &str,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    let listener = TcpListener::bind(bind_address).await?;
    let hardware = crate::hardware::detect_hardware();

    println!("FORGE WORKER HARDWARE REPORT");
    println!("{hardware:#?}");
    println!("FORGE WORKER listening on {bind_address}");

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
