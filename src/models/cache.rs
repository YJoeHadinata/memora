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

pub const DEV_CACHE_TARGETS: [(&str, &str, &str, &str); 4] = [
    (
        "xcode_derived_data",
        "Xcode DerivedData (Build Cache)",
        "Library/Developer/Xcode/DerivedData",
        "Xcode intermediate build artifacts, indices, and module cache",
    ),
    (
        "user_logs",
        "User Application Logs",
        "Library/Logs",
        "Diagnostic crash reports and session traces in ~/Library/Logs",
    ),
    (
        "cargo_cache",
        "Rust Cargo Package Cache",
        ".cargo/registry/cache",
        "Downloaded .crate archive packages in ~/.cargo",
    ),
    (
        "gradle_cache",
        "Gradle Build Cache",
        ".gradle/caches",
        "Downloaded dependencies and build caches in ~/.gradle",
    ),
];

pub fn friendly_cache_name(folder_name: &str) -> String {
    match folder_name {
        "Google" => "Google Chrome & Updater".to_string(),
        "Homebrew" => "Homebrew Packages & Downloads".to_string(),
        "CocoaPods" => "CocoaPods Dependency Cache".to_string(),
        "Steam" => "Steam Game Assets & Client".to_string(),
        "colima" => "Colima Container Cache".to_string(),
        "JNA" => "Java Native Access Temporary Files".to_string(),
        "JetPackCache" | "JetPackResourceBundles" => "JetBrains IDE Resources".to_string(),
        "DaVinci Resolve Welcome" => "DaVinci Resolve Assets".to_string(),
        n if n.starts_with("com.google.Chrome") => "Google Chrome Cache".to_string(),
        n if n.starts_with("com.spotify.client") => "Spotify Audio & Media Cache".to_string(),
        n if n.starts_with("com.microsoft.VSCode") => "Visual Studio Code Cache".to_string(),
        n if n.starts_with("com.tinyspeck.slackmacgap") => "Slack Desktop Cache".to_string(),
        n if n.starts_with("com.apple.dt.Xcode") => "Xcode Build & Module Cache".to_string(),
        n if n.starts_with("org.mozilla.firefox") => "Firefox Browser Cache".to_string(),
        n if n.starts_with("com.brave.Browser") => "Brave Browser Cache".to_string(),
        n if n.starts_with("com.apple.") => {
            format!("Apple Service ({})", n.trim_start_matches("com.apple."))
        }
        other => other.to_string(),
    }
}

pub fn is_safe_cache_to_clean(folder_name: &str) -> bool {
    const PROTECTED: &str = "CloudKit,FamilyCircle,familycircled,PassKit,GeoServices,\
com.apple.containermanagerd,com.apple.homed,com.apple.HomeKit,com.apple.Safari,\
com.apple.Safari.SafeBrowsing";
    !PROTECTED.split(',').any(|p| p == folder_name)
}
