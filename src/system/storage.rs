use crate::models::StorageSnapshot;

pub trait StorageCollector: Send + Sync {
    fn collect_storage(&self) -> Result<StorageSnapshot, String>;
}
