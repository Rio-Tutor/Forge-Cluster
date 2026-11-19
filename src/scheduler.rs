use std::error::Error;

use crate::{
    network,
    protocol::{Operation, TaskRequest},
};

pub async fn run_demo_tasks(worker_address: &str) -> Result<(), Box<dyn Error>> {
    let tasks = vec![
        TaskRequest {
            task_id: 1,
            operation: Operation::Add { a: 10, b: 20 },
        },
        TaskRequest {
            task_id: 2,
            operation: Operation::Multiply { a: 7, b: 6 },
        },
    ];

    for task in tasks {
        println!("Sending task {} to worker...", task.task_id);
        let response = network::send_task(worker_address, task).await?;

        println!(
            "Task {} completed by {}. Result: {}",
            response.task_id, response.worker, response.result
        );
    }

    Ok(())
}
