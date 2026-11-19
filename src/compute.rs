use crate::protocol::Operation;

pub fn execute(operation: Operation) -> i64 {
    match operation {
        Operation::Add { a, b } => a + b,
        Operation::Multiply { a, b } => a * b,
    }
}
