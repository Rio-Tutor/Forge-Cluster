use crate::protocol::Operation;

pub trait ComputeBackend: Send + Sync {
    fn name(&self) -> &str;

    fn is_available(&self) -> bool;

    fn execute(&self, operation: &Operation) -> Result<i64, String>;
}
