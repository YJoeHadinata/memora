use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: i32,
    pub name: String,
    pub path: Option<String>,
    pub memory_bytes: u64,
    pub resident_bytes: u64,
    pub peak_memory_bytes: u64,
    pub pageins: u64,
    pub disk_read_bytes: u64,
    pub disk_write_bytes: u64,
    pub cpu_usage: Option<f32>,
    pub is_application: bool,
    pub is_system_process: bool,
    pub is_protected: bool,
    pub user_id: Option<u32>,
    pub bundle_id: Option<String>,
    pub architecture: ProcessArchitecture,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessArchitecture {
    AppleSilicon,
    Intel,
    Unknown,
}

impl ProcessArchitecture {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::AppleSilicon => "Apple Silicon",
            Self::Intel => "Intel",
            Self::Unknown => "Unknown",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            Self::AppleSilicon => "Apple",
            Self::Intel => "Intel",
            Self::Unknown => "-",
        }
    }
}

impl ProcessInfo {
    pub fn memory_mb(&self) -> f64 {
        self.memory_bytes as f64 / (1024.0 * 1024.0)
    }

    pub fn memory_gb(&self) -> f64 {
        self.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn display_type(&self) -> &'static str {
        if self.is_application {
            "Application"
        } else if self.is_system_process {
            "System"
        } else {
            "Background"
        }
    }

    /// Calculates relative Energy & Battery Impact score,
    /// combining active CPU percentage and I/O activity.
    pub fn energy_impact(&self) -> f32 {
        let cpu = self.cpu_usage.unwrap_or(0.0);
        let io_mb = (self.disk_read_bytes + self.disk_write_bytes) as f32 / (1024.0 * 1024.0);
        cpu + (io_mb * 0.05).min(20.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessSortColumn {
    Memory,
    Cpu,
    Name,
    Pid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessFilterCategory {
    All,
    Applications,
    Background,
    System,
    HighMemory,
    HighEnergy,
}

impl ProcessFilterCategory {
    pub fn label(&self) -> &'static str {
        match self {
            ProcessFilterCategory::All => "All",
            ProcessFilterCategory::Applications => "Applications",
            ProcessFilterCategory::Background => "Background",
            ProcessFilterCategory::System => "System",
            ProcessFilterCategory::HighMemory => "High Memory (>500MB)",
            ProcessFilterCategory::HighEnergy => "Energy Hogs (>10% CPU)",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_info_helpers() {
        let proc = ProcessInfo {
            pid: 1234,
            name: "TestApp".to_string(),
            path: Some("/Applications/TestApp.app/Contents/MacOS/TestApp".to_string()),
            memory_bytes: 200 * 1024 * 1024,
            resident_bytes: 180 * 1024 * 1024,
            peak_memory_bytes: 220 * 1024 * 1024,
            pageins: 1500,
            disk_read_bytes: 10 * 1024 * 1024,
            disk_write_bytes: 5 * 1024 * 1024,
            cpu_usage: Some(5.2),
            is_application: true,
            is_system_process: false,
            is_protected: false,
            user_id: Some(501),
            bundle_id: Some("com.example.testapp".to_string()),
            architecture: ProcessArchitecture::AppleSilicon,
        };

        assert_eq!(proc.memory_mb(), 200.0);
        assert_eq!(proc.display_type(), "Application");
        assert_eq!(proc.architecture.display_name(), "Apple Silicon");
        assert_eq!(proc.peak_memory_bytes, 220 * 1024 * 1024);
        assert!(proc.energy_impact() > 5.0);
        assert_eq!(
            ProcessFilterCategory::HighEnergy.label(),
            "Energy Hogs (>10% CPU)"
        );
    }
}
