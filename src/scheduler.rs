use std::error::Error;

use tokio::task::JoinSet;

use crate::{
    network,
    protocol::{Operation, TaskRequest},
};

pub async fn run_sum_squares(
    worker_addresses: Vec<String>,
    limit: i64,
    chunk_size: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if worker_addresses.is_empty() {
        return Err("No workers provided".into());
    }

    if limit < 1 || chunk_size < 1 {
        return Err("Limit and chunk size must be positive".into());
    }

    let mut assignments = Vec::new();
    let mut start = 1_i64;
    let mut task_id = 1_u64;
    let mut worker_index = 0;

    // Divide the workload into fixed chunks.
    while start <= limit {
        let end = start
            .saturating_add(chunk_size - 1)
            .min(limit);

        let task = TaskRequest {
            task_id,
            operation: Operation::SumSquares { start, end },
        };

        let address = worker_addresses[worker_index].clone();

        assignments.push((address, task));

        start = end + 1;
        task_id += 1;
        worker_index = (worker_index + 1) % worker_addresses.len();
    }

    println!("FORGE STATIC SCHEDULER");
    println!("Workers: {}", worker_addresses.len());
    println!("Chunks: {}", assignments.len());

    // Dispatch all assigned chunks concurrently.
    let mut jobs = JoinSet::new();

    for (address, task) in assignments {
        println!(
            "Assigning task {} to {}",
            task.task_id, address
        );

        jobs.spawn(async move {
            let response = network::send_task(&address, task).await;
            (address, response)
        });
    }

    // Collect and aggregate partial results.
    let mut total = 0_i64;

    while let Some(job) = jobs.join_next().await {
        let (address, response) = job?;
        let response = response?;

        println!(
            "Task {} completed by {}. Partial result: {}",
            response.task_id,
            response.worker,
            response.result
        );

        total = total
            .checked_add(response.result)
            .ok_or("Final result overflow")?;

        println!("Response received from {}", address);
    }

    println!("\nFORGE COMPUTATION COMPLETE");
    println!("Sum of squares from 1 to {limit}: {total}");

    Ok(())
}
