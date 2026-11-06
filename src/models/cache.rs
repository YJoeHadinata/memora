use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheItem {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub size_bytes: u64,
    pub is_safe: bool,
    pub selected: bool,
    pub description: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheScanResult {
    pub items: Vec<CacheItem>,
    pub total_bytes: u64,
    pub scanned_at: Option<SystemTime>,
}

impl CacheScanResult {
    pub fn selected_total_bytes(&self) -> u64 {
        self.items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.size_bytes)
            .sum()
    }

    pub fn selected_count(&self) -> usize {
        self.items.iter().filter(|item| item.selected).count()
    }

    pub fn select_safe(&mut self) {
        for item in &mut self.items {
            item.selected = item.is_safe;
        }
    }

    pub fn select_all(&mut self) {
        for item in &mut self.items {
            item.selected = true;
        }
    }

    pub fn deselect_all(&mut self) {
        for item in &mut self.items {
            item.selected = false;
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurgeRamResult {
    pub reclaimed_bytes: u64,
    pub before_used_bytes: u64,
    pub after_used_bytes: u64,
    pub success: bool,
    pub message: String,
}
