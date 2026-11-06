use serde::{Deserialize, Serialize};
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryPressure {
    Normal,
    Warning,
    Critical,
    Unknown,
}

impl MemoryPressure {
    pub fn label(&self) -> &'static str {
        match self {
            MemoryPressure::Normal => "Normal",
            MemoryPressure::Warning => "Warning",
            MemoryPressure::Critical => "Critical",
            MemoryPressure::Unknown => "Unknown",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MemoryPressure::Normal => "System has adequate memory available",
            MemoryPressure::Warning => "Memory pressure is increasing; macOS is compressing memory",
            MemoryPressure::Critical => {
                "System is under severe memory pressure; paging and swap active"
            }
            MemoryPressure::Unknown => "Memory pressure status unavailable",
        }
    }

    pub fn level_numeric(&self) -> f64 {
        match self {
            MemoryPressure::Normal => 1.0,
            MemoryPressure::Warning => 2.0,
            MemoryPressure::Critical => 3.0,
            MemoryPressure::Unknown => 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySnapshot {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub app_memory_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub cached_files_bytes: u64,
    pub purgeable_bytes: u64,
    pub free_bytes: u64,
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub pressure: MemoryPressure,
    pub timestamp: SystemTime,
}

impl MemorySnapshot {
    pub fn used_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.used_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }

    pub fn available_percent(&self) -> f32 {
        if self.total_bytes == 0 {
            0.0
        } else {
            (self.available_bytes as f64 / self.total_bytes as f64 * 100.0) as f32
        }
    }

    pub fn swap_used_percent(&self) -> f32 {
        if self.swap_total_bytes == 0 {
            0.0
        } else {
            (self.swap_used_bytes as f64 / self.swap_total_bytes as f64 * 100.0) as f32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_snapshot_percentages() {
        let snapshot = MemorySnapshot {
            total_bytes: 16 * 1024 * 1024 * 1024,
            used_bytes: 12 * 1024 * 1024 * 1024,
            available_bytes: 4 * 1024 * 1024 * 1024,
            app_memory_bytes: 7 * 1024 * 1024 * 1024,
            wired_bytes: 3 * 1024 * 1024 * 1024,
            compressed_bytes: 2 * 1024 * 1024 * 1024,
            cached_files_bytes: 2 * 1024 * 1024 * 1024,
            purgeable_bytes: 512 * 1024 * 1024,
            free_bytes: 2 * 1024 * 1024 * 1024,
            swap_total_bytes: 2 * 1024 * 1024 * 1024,
            swap_used_bytes: 512 * 1024 * 1024,
            pressure: MemoryPressure::Normal,
            timestamp: SystemTime::now(),
        };

        assert_eq!(snapshot.used_percent(), 75.0);
        assert_eq!(snapshot.available_percent(), 25.0);
        assert_eq!(snapshot.swap_used_percent(), 25.0);
    }
}
