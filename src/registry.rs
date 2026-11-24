use crate::protocol::WorkerRegistration;

#[derive(Default)]
pub struct DeviceRegistry {
    workers: Vec<WorkerRegistration>,
}

impl DeviceRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a worker, or refresh its profile when it reconnects.
    pub fn register_or_update(&mut self, worker: WorkerRegistration) {
        if let Some(existing) = self
            .workers
            .iter_mut()
            .find(|existing| existing.worker_id == worker.worker_id)
        {
            *existing = worker;
        } else {
            self.workers.push(worker);
        }
    }

    pub fn count(&self) -> usize {
        self.workers.len()
    }

    pub fn get_by_id(&self, worker_id: &str) -> Option<&WorkerRegistration> {
        self.workers
            .iter()
            .find(|worker| worker.worker_id == worker_id)
    }

    pub fn get_all(&self) -> &[WorkerRegistration] {
        &self.workers
    }

    pub fn get_worker_addresses(&self) -> Vec<String> {
        self.workers
            .iter()
            .map(|worker| worker.task_address.clone())
            .collect()
    }
}
