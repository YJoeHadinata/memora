use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrashReport {
    pub item_count: usize,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallerItem {
    pub path: PathBuf,
    pub name: String,
    pub size_bytes: u64,
    pub modified_days_ago: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StorageJunkReport {
    pub trash: TrashReport,
    pub installers: Vec<InstallerItem>,
    pub apfs_snapshots: Vec<String>,
    pub scanned_at: Option<SystemTime>,
}

impl StorageJunkReport {
    pub fn total_reclaimable_bytes(&self) -> u64 {
        let installer_selected_bytes: u64 = self
            .installers
            .iter()
            .filter(|i| i.selected)
            .map(|i| i.size_bytes)
            .sum();
        self.trash.total_bytes + installer_selected_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_junk_report_total() {
        let mut report = StorageJunkReport {
            trash: TrashReport {
                item_count: 5,
                total_bytes: 1000,
            },
            installers: vec![
                InstallerItem {
                    path: PathBuf::from("/tmp/app.dmg"),
                    name: "app.dmg".to_string(),
                    size_bytes: 500,
                    modified_days_ago: 3,
                    selected: true,
                },
                InstallerItem {
                    path: PathBuf::from("/tmp/other.pkg"),
                    name: "other.pkg".to_string(),
                    size_bytes: 700,
                    modified_days_ago: 10,
                    selected: false,
                },
            ],
            apfs_snapshots: vec!["2023-11-01-120000".to_string()],
            scanned_at: None,
        };

        assert_eq!(report.total_reclaimable_bytes(), 1500);
        report.installers[1].selected = true;
        assert_eq!(report.total_reclaimable_bytes(), 2200);
    }
}
