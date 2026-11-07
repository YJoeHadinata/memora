use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupItemKind {
    /// Plist in ~/Library/LaunchAgents (User session)
    UserAgent,
    /// Plist in /Library/LaunchAgents (All users)
    SystemAgent,
    /// Plist in /Library/LaunchDaemons (Root system daemon)
    SystemDaemon,
}

impl StartupItemKind {
    pub fn label(&self) -> &'static str {
        match self {
            StartupItemKind::UserAgent => "User Agent",
            StartupItemKind::SystemAgent => "System Agent",
            StartupItemKind::SystemDaemon => "System Daemon",
        }
    }

    pub fn is_user_writable(&self) -> bool {
        matches!(self, StartupItemKind::UserAgent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupItemStatus {
    Running(u32), // PID
    Loaded,       // Loaded in launchctl, but idle
    Disabled,     // Disabled or .disabled extension
    NotLoaded,    // File exists, but not active in current launchd session
}

impl StartupItemStatus {
    pub fn is_running(&self) -> bool {
        matches!(self, StartupItemStatus::Running(_))
    }

    pub fn display_text(&self) -> String {
        match self {
            StartupItemStatus::Running(pid) => format!("Running (PID {})", pid),
            StartupItemStatus::Loaded => "Idle / Loaded".to_string(),
            StartupItemStatus::Disabled => "Disabled".to_string(),
            StartupItemStatus::NotLoaded => "Not Loaded".to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct StartupItem {
    pub id: String,
    pub label: String,
    pub display_name: String,
    pub program_path: Option<String>,
    pub plist_path: PathBuf,
    pub kind: StartupItemKind,
    pub status: StartupItemStatus,
    pub run_at_load: bool,
    pub keep_alive: bool,
    pub is_disabled_file: bool,
    pub file_size_bytes: u64,
}

impl StartupItem {
    pub fn is_active(&self) -> bool {
        !self.is_disabled_file && !matches!(self.status, StartupItemStatus::Disabled)
    }
}

#[derive(Debug, Clone, Default)]
pub struct StartupScanResult {
    pub items: Vec<StartupItem>,
    pub scan_duration: Duration,
}

impl StartupScanResult {
    pub fn running_count(&self) -> usize {
        self.items.iter().filter(|i| i.status.is_running()).count()
    }

    pub fn disabled_count(&self) -> usize {
        self.items.iter().filter(|i| !i.is_active()).count()
    }

    pub fn user_agents_count(&self) -> usize {
        self.items
            .iter()
            .filter(|i| i.kind == StartupItemKind::UserAgent)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_item_status() {
        let running = StartupItemStatus::Running(1234);
        assert!(running.is_running());
        assert_eq!(running.display_text(), "Running (PID 1234)");

        let loaded = StartupItemStatus::Loaded;
        assert!(!loaded.is_running());
        assert_eq!(loaded.display_text(), "Idle / Loaded");

        let disabled = StartupItemStatus::Disabled;
        assert!(!disabled.is_running());
        assert_eq!(disabled.display_text(), "Disabled");
    }

    #[test]
    fn test_startup_scan_result_counts() {
        let items = vec![
            StartupItem {
                id: "1".into(),
                label: "com.test.one".into(),
                display_name: "One".into(),
                program_path: None,
                plist_path: PathBuf::from("/path/to/one.plist"),
                kind: StartupItemKind::UserAgent,
                status: StartupItemStatus::Running(100),
                run_at_load: true,
                keep_alive: false,
                is_disabled_file: false,
                file_size_bytes: 500,
            },
            StartupItem {
                id: "2".into(),
                label: "com.test.two".into(),
                display_name: "Two".into(),
                program_path: None,
                plist_path: PathBuf::from("/path/to/two.plist.disabled"),
                kind: StartupItemKind::UserAgent,
                status: StartupItemStatus::Disabled,
                run_at_load: false,
                keep_alive: false,
                is_disabled_file: true,
                file_size_bytes: 400,
            },
            StartupItem {
                id: "3".into(),
                label: "com.test.three".into(),
                display_name: "Three".into(),
                program_path: None,
                plist_path: PathBuf::from("/Library/LaunchDaemons/three.plist"),
                kind: StartupItemKind::SystemDaemon,
                status: StartupItemStatus::Loaded,
                run_at_load: true,
                keep_alive: true,
                is_disabled_file: false,
                file_size_bytes: 600,
            },
        ];

        let result = StartupScanResult {
            items,
            scan_duration: Duration::from_millis(50),
        };

        assert_eq!(result.running_count(), 1);
        assert_eq!(result.disabled_count(), 1);
        assert_eq!(result.user_agents_count(), 2);
    }
}
