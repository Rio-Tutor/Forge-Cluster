use crate::protocol::Operation;
use super::backend::ComputeBackend;

pub struct CpuBackend;

impl ComputeBackend for CpuBackend {
    fn name(&self) -> &str {
        "CPU"
    }

    fn is_available(&self) -> bool {
        true
    }

    fn execute(&self, operation: &Operation) -> Result<i64, String> {
        match operation {
            Operation::Add { a, b } => {
                a.checked_add(*b)
                    .ok_or_else(|| "Addition overflow".to_string())
            }

            Operation::Multiply { a, b } => {
                a.checked_mul(*b)
                    .ok_or_else(|| "Multiplication overflow".to_string())
            }

            Operation::SumSquares { start, end } => {
                if *start < 0 || end < start {
                    return Err("Invalid SumSquares range".into());
                }

                let mut total = 0_i64;

                for n in *start..=*end {
                    let square = n.checked_mul(n)
                        .ok_or("Square overflow")?;

                    total = total.checked_add(square)
                        .ok_or("Sum overflow")?;
                }

                Ok(total)
            }
        }
    }
}
