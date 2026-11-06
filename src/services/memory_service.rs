use std::sync::Arc;

use crate::error::MemoraError;
use crate::models::MemorySnapshot;
use crate::system::MemoryCollector;

pub struct MemoryService {
    collector: Arc<dyn MemoryCollector>,
}

impl MemoryService {
    pub fn new(collector: Arc<dyn MemoryCollector>) -> Self {
        Self { collector }
    }

    pub fn snapshot(&self) -> Result<MemorySnapshot, MemoraError> {
        self.collector.collect_memory()
    }
}
