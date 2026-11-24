use serde::{Deserialize, Serialize};

use crate::hardware::HardwareInfo;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    RunTask(TaskRequest),
    TaskResult(TaskResponse),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRegistration {
    pub worker_id: String,
    pub worker_name: String,
    pub task_address: String,
    pub hardware: HardwareInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerHeartbeat {
    pub kind: String,
    pub worker_id: String,
}

impl WorkerHeartbeat {
    pub fn new(worker_id: impl Into<String>) -> Self {
        Self { kind: "heartbeat".into(), worker_id: worker_id.into() }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationResponse {
    pub accepted: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRequest {
    pub task_id: u64,
    pub operation: Operation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Operation {
    Add { a: i64, b: i64 },
    Multiply { a: i64, b: i64 },
    SumSquares { start: i64, end: i64 },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TaskResponse {
    pub task_id: u64,
    pub result: i64,
    pub worker: String,
}
