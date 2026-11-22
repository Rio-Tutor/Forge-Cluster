use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum Message {
    RunTask(TaskRequest),
    TaskResult(TaskResponse),
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
