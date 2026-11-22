pub mod backend;
pub mod cpu;

pub use backend::ComputeBackend;
pub use cpu::CpuBackend;

use crate::protocol::Operation;

pub fn execute(operation: Operation) -> Result<i64, String> {
    let backend = CpuBackend;
    backend.execute(&operation)
}
