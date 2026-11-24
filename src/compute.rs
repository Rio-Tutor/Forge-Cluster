pub mod backend;
pub mod cpu;

pub use backend::ComputeBackend;
pub use cpu::CpuBackend;

use crate::protocol::Operation;

pub fn execute(operation: Operation) -> Result<i64, String> {
    let backend = CpuBackend;
    backend.execute(&operation)
}

/// Measure a small deterministic integer workload and return approximate
/// iterations per second. This is a scheduling hint, not a hardware rating.
pub fn benchmark_cpu(iterations: u64) -> u64 {
    let start = std::time::Instant::now();
    let mut value = 0_u64;
    for n in 0..iterations {
        value = value.wrapping_add(std::hint::black_box(n).wrapping_mul(31).rotate_left(7));
    }
    std::hint::black_box(value);
    let elapsed = start.elapsed().as_secs_f64().max(0.000_001);
    (iterations as f64 / elapsed) as u64
}
