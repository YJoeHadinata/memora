use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskVolume {
    pub name: String,
    pub mount_point: String,
    pub device_path: String,
    pub filesystem_type: String,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub is_system: bool,
    pub is_removable: bool,
}

impl DiskVolume {
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

    pub fn is_low_space(&self) -> bool {
        self.available_bytes < 15 * 1024 * 1024 * 1024 // less than 15 GB
    }

    /// Determines if this is an internal APFS helper/system volume
    /// (e.g. swap VM, Preboot, Recovery, Update) rather than a primary user drive.
    pub fn is_helper_volume(&self) -> bool {
        self.mount_point.starts_with("/System/Volumes/VM")
            || self.mount_point.starts_with("/System/Volumes/Preboot")
            || self.mount_point.starts_with("/System/Volumes/Update")
            || self.mount_point.contains("Recovery")
            || self.mount_point.contains("Developer/CoreDevice")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSnapshot {
    pub volumes: Vec<DiskVolume>,
    pub primary_volume: Option<DiskVolume>,
    pub swap_headroom_bytes: u64,
    pub swap_headroom_healthy: bool,
}

impl StorageSnapshot {
    /// Returns primary and user-facing mounted volumes (e.g. Data and external drives).
    pub fn user_volumes(&self) -> Vec<&DiskVolume> {
        self.volumes
            .iter()
            .filter(|v| !v.is_helper_volume())
            .collect()
    }

    /// Returns internal APFS helper and diagnostic volumes.
    pub fn helper_volumes(&self) -> Vec<&DiskVolume> {
        self.volumes
            .iter()
            .filter(|v| v.is_helper_volume())
            .collect()
    }

    pub fn swap_headroom_status(&self) -> (&'static str, &'static str) {
        if self.swap_headroom_bytes > 25 * 1024 * 1024 * 1024 {
            ("Healthy", "Adequate SSD headroom for dynamic swapfiles")
        } else if self.swap_headroom_bytes > 12 * 1024 * 1024 * 1024 {
            (
                "Moderate",
                "SSD free space is getting tight; watch heavy swap usage",
            )
        } else {
            (
                "Critical",
                "SSD space critically low (< 12 GB); macOS swap allocation may fail!",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_disk_volume_percentages() {
        let vol = DiskVolume {
            name: "Macintosh HD".to_string(),
            mount_point: "/System/Volumes/Data".to_string(),
            device_path: "/dev/disk3s5".to_string(),
            filesystem_type: "apfs".to_string(),
            total_bytes: 200 * 1024 * 1024 * 1024,
            used_bytes: 150 * 1024 * 1024 * 1024,
            available_bytes: 50 * 1024 * 1024 * 1024,
            is_system: true,
            is_removable: false,
        };

        assert_eq!(vol.used_percent(), 75.0);
        assert_eq!(vol.available_percent(), 25.0);
        assert!(!vol.is_low_space());
        assert!(!vol.is_helper_volume());

        let helper_vol = DiskVolume {
            mount_point: "/System/Volumes/VM".to_string(),
            ..vol
        };
        assert!(helper_vol.is_helper_volume());
    }
}
