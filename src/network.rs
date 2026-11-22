use std::error::Error;

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
};

use crate::protocol::{Message, TaskRequest, TaskResponse};

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
        return Err(
            "Worker closed the connection without a response".into()
        );
    }

    match serde_json::from_str::<Message>(&response_line)? {
        Message::TaskResult(response) => Ok(response),

        Message::RunTask(_) => {
            Err("Worker sent an unexpected task request".into())
        }
    }
}
