use std::error::Error;

use tokio::{task::JoinSet, time::{timeout, Duration}};

use crate::{
    network,
    protocol::{Operation, TaskRequest, WorkerRegistration},
};

pub async fn run_sum_squares(
    workers: Vec<WorkerRegistration>,
    limit: i64,
    chunk_size: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if workers.is_empty() { return Err("No workers provided".into()); }
    if limit < 1 || chunk_size < 1 { return Err("Limit and chunk size must be positive".into()); }

    let max_score = workers.iter().map(|w| w.benchmark_score).max().unwrap_or(1).max(1);
    let mut weighted_workers = Vec::new();
    for worker in &workers {
        let weight = ((worker.benchmark_score.max(1) as f64 / max_score as f64) * 8.0)
            .ceil().clamp(1.0, 8.0) as usize;
        for _ in 0..weight { weighted_workers.push(worker.task_address.clone()); }
        println!("Worker {} score={} weight={weight}", worker.worker_name, worker.benchmark_score);
    }

    let mut assignments = Vec::new();
    let mut start = 1_i64;
    let mut task_id = 1_u64;
    let mut worker_index = 0_usize;
    while start <= limit {
        let end = start.saturating_add(chunk_size - 1).min(limit);
        assignments.push((weighted_workers[worker_index].clone(), TaskRequest {
            task_id,
            operation: Operation::SumSquares { start, end },
        }));
        start = end + 1;
        task_id += 1;
        worker_index = (worker_index + 1) % weighted_workers.len();
    }

    println!("FORGE PERFORMANCE-AWARE SCHEDULER");
    println!("Workers: {} | Chunks: {}", workers.len(), assignments.len());

    let mut jobs = JoinSet::new();
    for (address, task) in assignments {
        let display_address = address.clone();
        jobs.spawn(async move {
            let mut last_error = None;
            for attempt in 1..=3 {
                match timeout(Duration::from_secs(20), network::send_task(&address, task.clone())).await {
                    Ok(Ok(response)) => return Ok::<_, String>((display_address, response)),
                    Ok(Err(error)) => last_error = Some(error.to_string()),
                    Err(_) => last_error = Some("task timed out after 20 seconds".into()),
                }
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(300 * attempt)).await;
                }
            }
            Err(format!("Task {} failed after 3 attempts: {}", task.task_id, last_error.unwrap_or_else(|| "unknown error".into())))
        });
    }

    let mut total = 0_i64;
    let mut completed = 0_usize;
    while let Some(job) = jobs.join_next().await {
        let result = job??;
        let (address, response) = result;
        total = total.checked_add(response.result).ok_or("Final result overflow")?;
        completed += 1;
        println!("Task {} completed by {} at {}. Partial result: {}", response.task_id, response.worker, address, response.result);
    }

    println!("\nFORGE COMPUTATION COMPLETE");
    println!("Tasks completed: {completed}");
    println!("Sum of squares from 1 to {limit}: {total}");
    Ok(())
}
