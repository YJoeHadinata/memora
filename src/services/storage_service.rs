use std::sync::Arc;

use crate::models::StorageSnapshot;
use crate::system::storage::StorageCollector;

pub struct StorageService {
    collector: Arc<dyn StorageCollector>,
}

impl StorageService {
    pub fn new(collector: Arc<dyn StorageCollector>) -> Self {
        Self { collector }
    }

    pub fn snapshot(&self) -> Result<StorageSnapshot, String> {
        self.collector.collect_storage()
    }
}
