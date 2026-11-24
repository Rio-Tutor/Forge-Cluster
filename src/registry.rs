use std::{collections::HashMap, time::{Duration, Instant}};

use crate::protocol::WorkerRegistration;

struct RegisteredWorker {
    registration: WorkerRegistration,
    last_seen: Instant,
}

#[derive(Default)]
pub struct DeviceRegistry {
    workers: HashMap<String, RegisteredWorker>,
}

impl DeviceRegistry {
    pub fn new() -> Self { Self::default() }

    pub fn register_or_update(&mut self, worker: WorkerRegistration) {
        let id = worker.worker_id.clone();
        self.workers.insert(id, RegisteredWorker {
            registration: worker,
            last_seen: Instant::now(),
        });
    }

    pub fn heartbeat(&mut self, worker_id: &str) -> bool {
        if let Some(worker) = self.workers.get_mut(worker_id) {
            worker.last_seen = Instant::now();
            true
        } else {
            false
        }
    }

    pub fn remove_stale(&mut self, timeout: Duration) -> Vec<String> {
        let now = Instant::now();
        let stale: Vec<String> = self.workers.iter()
            .filter(|(_, worker)| now.duration_since(worker.last_seen) > timeout)
            .map(|(id, _)| id.clone())
            .collect();
        for id in &stale { self.workers.remove(id); }
        stale
    }

    pub fn count(&self) -> usize { self.workers.len() }

    pub fn get_all(&self) -> Vec<WorkerRegistration> {
        self.workers.values().map(|w| w.registration.clone()).collect()
    }

    pub fn get_worker_addresses(&self) -> Vec<String> {
        self.workers.values().map(|w| w.registration.task_address.clone()).collect()
    }
}
