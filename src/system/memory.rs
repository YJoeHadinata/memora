use crate::error::MemoraError;
use crate::models::MemorySnapshot;

pub trait MemoryCollector: Send + Sync {
    fn collect_memory(&self) -> Result<MemorySnapshot, MemoraError>;
}
