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


#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Operation;

    #[test]
    fn adds_without_overflow() {
        let result = CpuBackend.execute(&Operation::Add { a: 20, b: 22 });
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn reports_addition_overflow() {
        let result = CpuBackend.execute(&Operation::Add { a: i64::MAX, b: 1 });
        assert!(result.is_err());
    }

    #[test]
    fn multiplies_without_overflow() {
        let result = CpuBackend.execute(&Operation::Multiply { a: 7, b: 6 });
        assert_eq!(result, Ok(42));
    }

    #[test]
    fn reports_multiplication_overflow() {
        let result = CpuBackend.execute(&Operation::Multiply { a: i64::MAX, b: 2 });
        assert!(result.is_err());
    }

    #[test]
    fn sums_square_range() {
        let result = CpuBackend.execute(&Operation::SumSquares { start: 1, end: 3 });
        assert_eq!(result, Ok(14));
    }

    #[test]
    fn rejects_invalid_square_range() {
        let result = CpuBackend.execute(&Operation::SumSquares { start: 4, end: 3 });
        assert!(result.is_err());
    }

    #[test]
    fn reports_square_overflow() {
        let result = CpuBackend.execute(&Operation::SumSquares { start: i64::MAX, end: i64::MAX });
        assert!(result.is_err());
    }
}
