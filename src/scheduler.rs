use std::{collections::VecDeque, error::Error, time::{Duration, Instant}};

use tokio::{task::JoinSet, time::timeout};

use crate::{
    network,
    protocol::{Operation, TaskRequest, TaskResponse, WorkerRegistration},
};

const TASK_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_ATTEMPTS: usize = 3;
const MAX_CONCURRENCY: usize = 8;

type TaskOutcome = Result<(String, TaskResponse, Duration, usize), String>;

/// Execute a task against its preferred worker, then fail over across the
/// remaining registered workers. A maximum of MAX_ATTEMPTS connection/task
/// attempts is made for each task.
async fn execute_assignment(
    preferred_address: String,
    worker_addresses: Vec<String>,
    task: TaskRequest,
) -> TaskOutcome {
    let mut candidates = Vec::with_capacity(worker_addresses.len() + 1);
    candidates.push(preferred_address);
    for address in worker_addresses {
        if !candidates.contains(&address) {
            candidates.push(address);
        }
    }

    let started = Instant::now();
    let mut last_error = String::from("no worker addresses available");

    for attempt in 1..=MAX_ATTEMPTS {
        let address = candidates[(attempt - 1) % candidates.len()].clone();

        match timeout(TASK_TIMEOUT, network::send_task(&address, task.clone())).await {
            Ok(Ok(response)) => {
                return Ok((address, response, started.elapsed(), attempt));
            }
            Ok(Err(error)) => {
                last_error = format!("{address}: {error}");
            }
            Err(_) => {
                last_error = format!(
                    "{address}: task timed out after {} seconds",
                    TASK_TIMEOUT.as_secs()
                );
            }
        }

        if attempt < MAX_ATTEMPTS {
            tokio::time::sleep(Duration::from_millis(300 * attempt as u64)).await;
        }
    }

    Err(format!(
        "Task {} failed after {MAX_ATTEMPTS} attempts: {last_error}",
        task.task_id
    ))
}

pub async fn run_sum_squares(
    workers: Vec<WorkerRegistration>,
    limit: i64,
    chunk_size: i64,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    if workers.is_empty() { return Err("No workers provided".into()); }
    if limit < 1 || chunk_size < 1 { return Err("Limit and chunk size must be positive".into()); }

    let effective_score = |worker: &WorkerRegistration| {
        (worker.benchmark_score.max(1) as u128) * (worker.hardware.logical_cores.max(1) as u128)
    };
    let max_score = workers.iter().map(effective_score).max().unwrap_or(1).max(1);
    let mut weighted_workers = Vec::new();
    let mut worker_addresses = Vec::new();

    for worker in &workers {
        let weight = ((effective_score(worker) as f64 / max_score as f64) * 8.0)
            .ceil().clamp(1.0, 8.0) as usize;
        for _ in 0..weight { weighted_workers.push(worker.task_address.clone()); }
        if !worker_addresses.contains(&worker.task_address) {
            worker_addresses.push(worker.task_address.clone());
        }
        println!(
            "Worker {} score={} iterations/sec × {} cores, weight={weight}",
            worker.worker_name,
            worker.benchmark_score,
            worker.hardware.logical_cores
        );
    }

    let mut assignments = VecDeque::new();
    let mut start = 1_i64;
    let mut task_id = 1_u64;
    let mut worker_index = 0_usize;

    while start <= limit {
        let end = start.saturating_add(chunk_size - 1).min(limit);
        assignments.push_back((weighted_workers[worker_index].clone(), TaskRequest {
            task_id,
            operation: Operation::SumSquares { start, end },
        }));
        start = end + 1;
        task_id += 1;
        worker_index = (worker_index + 1) % weighted_workers.len();
    }

    let concurrency = (workers.len() * 2).clamp(1, MAX_CONCURRENCY);
    println!("FORGE PERFORMANCE-AWARE SCHEDULER");
    println!(
        "Workers: {} | Chunks: {} | Max in-flight tasks: {}",
        workers.len(),
        assignments.len(),
        concurrency
    );

    let computation_started = Instant::now();
    let mut jobs = JoinSet::new();

    // Start only a bounded number of tasks. Each completion opens one slot
    // for the next queued assignment instead of flooding every worker at once.
    while jobs.len() < concurrency {
        let Some((preferred, task)) = assignments.pop_front() else { break; };
        let candidates = worker_addresses.clone();
        jobs.spawn(async move {
            execute_assignment(preferred, candidates, task).await
        });
    }

    let mut total = 0_i64;
    let mut completed = 0_usize;
    let mut total_attempts = 0_usize;

    while let Some(job) = jobs.join_next().await {
        let outcome = job.map_err(std::io::Error::other)?;
        let (address, response, elapsed, attempts) =
            outcome.map_err(std::io::Error::other)?;

        total = total.checked_add(response.result).ok_or("Final result overflow")?;
        completed += 1;
        total_attempts += attempts;

        println!(
            "Task {} completed by {} at {} | attempts: {} | elapsed: {} ms | partial result: {}",
            response.task_id,
            response.worker,
            address,
            attempts,
            elapsed.as_millis(),
            response.result
        );

        if let Some((preferred, task)) = assignments.pop_front() {
            let candidates = worker_addresses.clone();
            jobs.spawn(async move {
                execute_assignment(preferred, candidates, task).await
            });
        }
    }

    println!("\nFORGE COMPUTATION COMPLETE");
    println!("Tasks completed: {completed}");
    println!("Total attempts: {total_attempts}");
    println!("Total elapsed: {} ms", computation_started.elapsed().as_millis());
    println!("Sum of squares from 1 to {limit}: {total}");

    Ok(())
}
