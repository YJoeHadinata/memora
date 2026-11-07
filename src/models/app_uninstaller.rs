use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppLeftoverKind {
    ApplicationBundle,
    ApplicationSupport,
    Caches,
    Preferences,
    SavedState,
    Containers,
    Logs,
    HTTPStorages,
    WebKit,
    Other,
}

impl AppLeftoverKind {
    pub fn display_label(&self) -> &'static str {
        match self {
            Self::ApplicationBundle => "Application Bundle (.app)",
            Self::ApplicationSupport => "Application Support",
            Self::Caches => "Caches",
            Self::Preferences => "Preferences (.plist)",
            Self::SavedState => "Saved State",
            Self::Containers => "Sandbox Containers",
            Self::Logs => "Application Logs",
            Self::HTTPStorages => "HTTP Storage & Cookies",
            Self::WebKit => "WebKit Local Storage",
            Self::Other => "Residual Files",
        }
    }
}

#[derive(Debug, Clone)]
pub struct AppLeftoverItem {
    pub path: PathBuf,
    pub kind: AppLeftoverKind,
    pub size_bytes: u64,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct InstalledAppItem {
    pub name: String,
    pub bundle_id: String,
    pub version: String,
    pub app_path: PathBuf,
    pub app_size_bytes: u64,
    pub is_system_app: bool,
    pub leftovers: Vec<AppLeftoverItem>,
}

impl InstalledAppItem {
    pub fn total_size_bytes(&self) -> u64 {
        self.leftovers.iter().map(|l| l.size_bytes).sum()
    }

    pub fn selected_size_bytes(&self) -> u64 {
        self.leftovers
            .iter()
            .filter(|l| l.selected)
            .map(|l| l.size_bytes)
            .sum()
    }

    pub fn selected_paths(&self) -> Vec<PathBuf> {
        self.leftovers
            .iter()
            .filter(|l| l.selected)
            .map(|l| l.path.clone())
            .collect()
    }

    pub fn has_selected(&self) -> bool {
        self.leftovers.iter().any(|l| l.selected)
    }

    pub fn leftover_count(&self) -> usize {
        // Exclude the .app bundle itself from leftover count
        self.leftovers
            .iter()
            .filter(|l| l.kind != AppLeftoverKind::ApplicationBundle)
            .count()
    }

    pub fn leftover_size_bytes(&self) -> u64 {
        self.leftovers
            .iter()
            .filter(|l| l.kind != AppLeftoverKind::ApplicationBundle)
            .map(|l| l.size_bytes)
            .sum()
    }
}

#[derive(Debug, Clone, Default)]
pub struct AppUninstallScanResult {
    pub apps: Vec<InstalledAppItem>,
}

impl AppUninstallScanResult {
    pub fn total_apps(&self) -> usize {
        self.apps.len()
    }

    pub fn total_reclaimable_bytes(&self) -> u64 {
        self.apps
            .iter()
            .filter(|a| !a.is_system_app)
            .map(|a| a.total_size_bytes())
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_installed_app_item_totals() {
        let app = InstalledAppItem {
            name: "Test App".to_string(),
            bundle_id: "com.test.app".to_string(),
            version: "1.0.0".to_string(),
            app_path: PathBuf::from("/Applications/Test.app"),
            app_size_bytes: 100_000,
            is_system_app: false,
            leftovers: vec![
                AppLeftoverItem {
                    path: PathBuf::from("/Applications/Test.app"),
                    kind: AppLeftoverKind::ApplicationBundle,
                    size_bytes: 100_000,
                    selected: true,
                },
                AppLeftoverItem {
                    path: PathBuf::from("/Users/test/Library/Caches/com.test.app"),
                    kind: AppLeftoverKind::Caches,
                    size_bytes: 50_000,
                    selected: true,
                },
                AppLeftoverItem {
                    path: PathBuf::from("/Users/test/Library/Preferences/com.test.app.plist"),
                    kind: AppLeftoverKind::Preferences,
                    size_bytes: 2_000,
                    selected: false,
                },
            ],
        };

        assert_eq!(app.total_size_bytes(), 152_000);
        assert_eq!(app.selected_size_bytes(), 150_000);
        assert_eq!(app.leftover_count(), 2);
        assert_eq!(app.leftover_size_bytes(), 52_000);
        assert_eq!(app.selected_paths().len(), 2);
    }
}
