use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::models::{CacheItem, CacheScanResult, PurgeRamResult};
use crate::services::MemoryService;

pub struct CacheService {
    memory_service: Arc<MemoryService>,
}

impl CacheService {
    pub fn new(memory_service: Arc<MemoryService>) -> Self {
        Self { memory_service }
    }

    /// Returns the standard user cache directory path (`~/Library/Caches`).
    pub fn get_user_cache_dir() -> Option<PathBuf> {
        dirs_cache_dir().or_else(|| {
            std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Caches"))
        })
    }

    /// Scans user application caches in `~/Library/Caches`.
    pub fn scan_user_caches() -> CacheScanResult {
        let Some(cache_dir) = Self::get_user_cache_dir() else {
            return CacheScanResult::default();
        };

        if !cache_dir.is_dir() {
            return CacheScanResult::default();
        }

        let mut items = Vec::new();
        let read_dir = match fs::read_dir(&cache_dir) {
            Ok(entries) => entries,
            Err(_) => return CacheScanResult::default(),
        };

        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();

            // Calculate directory size safely
            let size_bytes = calculate_dir_size(&path, 3);
            if size_bytes == 0 {
                continue;
            }

            let name = friendly_cache_name(&file_name);
            let is_safe = is_safe_cache_to_clean(&file_name);
            let description = format!("Stored at ~/Library/Caches/{}", file_name);

            // Default to selected if safe and larger than 10 MB
            let selected = is_safe && size_bytes > 10 * 1024 * 1024;

            items.push(CacheItem {
                id: file_name,
                name,
                path,
                size_bytes,
                is_safe,
                selected,
                description,
            });
        }

        // Also discover well-known Developer & Log build caches
        if let Some(home) = std::env::var_os("HOME") {
            let home_path = PathBuf::from(home);
            let dev_targets = [
                (
                    "xcode_derived_data",
                    "Xcode DerivedData (Build Cache)",
                    home_path.join("Library/Developer/Xcode/DerivedData"),
                    "Xcode intermediate build artifacts, indices, and module cache",
                ),
                (
                    "user_logs",
                    "User Application Logs",
                    home_path.join("Library/Logs"),
                    "Diagnostic crash reports and session traces in ~/Library/Logs",
                ),
                (
                    "cargo_cache",
                    "Rust Cargo Package Cache",
                    home_path.join(".cargo/registry/cache"),
                    "Downloaded .crate archive packages in ~/.cargo",
                ),
                (
                    "gradle_cache",
                    "Gradle Build Cache",
                    home_path.join(".gradle/caches"),
                    "Downloaded dependencies and build caches in ~/.gradle",
                ),
            ];

            for (id, name, path, desc) in dev_targets {
                if path.is_dir() {
                    let size_bytes = calculate_dir_size(&path, 3);
                    if size_bytes > 5 * 1024 * 1024 {
                        items.push(CacheItem {
                            id: id.to_string(),
                            name: name.to_string(),
                            path,
                            size_bytes,
                            is_safe: true,
                            selected: false,
                            description: desc.to_string(),
                        });
                    }
                }
            }
        }

        // Sort largest caches first
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));

        let total_bytes = items.iter().map(|item| item.size_bytes).sum();

        CacheScanResult {
            items,
            total_bytes,
            scanned_at: Some(SystemTime::now()),
        }
    }

    /// Asynchronously cleans selected cache items, removing their internal contents while preserving folder roots.
    pub fn clean_cache_items(paths: &[PathBuf]) -> Result<u64, String> {
        let mut total_freed = 0u64;

        for dir in paths {
            if !dir.is_dir() {
                continue;
            }

            let read_dir = match fs::read_dir(dir) {
                Ok(entries) => entries,
                Err(_) => continue,
            };

            for entry in read_dir.flatten() {
                let sub_path = entry.path();
                let file_size = if let Ok(meta) = sub_path.symlink_metadata() {
                    meta.len()
                } else {
                    0
                };

                if sub_path.is_dir() {
                    let dir_size = calculate_dir_size(&sub_path, 4);
                    if fs::remove_dir_all(&sub_path).is_ok() {
                        total_freed += dir_size;
                    }
                } else if fs::remove_file(&sub_path).is_ok() {
                    total_freed += file_size;
                }
            }
        }

        Ok(total_freed)
    }

    /// Purges system inactive and purgeable disk buffer caches via macOS `/usr/sbin/purge`.
    pub fn purge_system_ram_cache(&self) -> Result<PurgeRamResult, String> {
        let before_snap = self
            .memory_service
            .snapshot()
            .map_err(|e| format!("Failed to capture initial memory snapshot: {}", e))?;

        // First attempt standard `/usr/sbin/purge`
        let mut executed_elevated = false;
        let direct_output = Command::new("/usr/sbin/purge").output();

        let success = match direct_output {
            Ok(output) if output.status.success() => true,
            _ => {
                // If direct purge fails (e.g. Operation not permitted), prompt via AppleScript administrator privileges
                executed_elevated = true;
                let script = r#"do shell script "/usr/sbin/purge" with administrator privileges"#;
                let osascript_output = Command::new("osascript")
                    .args(["-e", script])
                    .output()
                    .map_err(|e| format!("Failed to invoke authorization prompt: {}", e))?;

                osascript_output.status.success()
            }
        };

        if !success {
            return Err("Cache purge was cancelled or unauthorized by user.".to_string());
        }

        // Wait briefly for macOS Mach VM statistics to reconcile
        thread::sleep(Duration::from_millis(350));

        let after_snap = self
            .memory_service
            .snapshot()
            .map_err(|e| format!("Failed to capture post-purge snapshot: {}", e))?;

        let reclaimed_used = before_snap.used_bytes.saturating_sub(after_snap.used_bytes);
        let reclaimed_cache = before_snap
            .cached_files_bytes
            .saturating_sub(after_snap.cached_files_bytes);
        let reclaimed_bytes = reclaimed_used.max(reclaimed_cache);

        let method_str = if executed_elevated {
            "macOS System Purge (with Admin Authorization)"
        } else {
            "macOS System Purge"
        };

        let message = format!(
            "{} flushed inactive buffer cache successfully. Reclaimed {} of RAM.",
            method_str,
            crate::utils::format::format_bytes(reclaimed_bytes)
        );

        Ok(PurgeRamResult {
            reclaimed_bytes,
            before_used_bytes: before_snap.used_bytes,
            after_used_bytes: after_snap.used_bytes,
            success: true,
            message,
        })
    }

    /// Flushes system DNS cache using `dscacheutil -flushcache`.
    pub fn flush_dns_cache() -> Result<(), String> {
        let output = Command::new("dscacheutil")
            .arg("-flushcache")
            .output()
            .map_err(|e| format!("Failed to execute dscacheutil: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err("Failed to flush DNS cache".to_string())
        }
    }

    /// Resets macOS QuickLook thumbnail cache using `qlmanage -r cache`.
    pub fn flush_quicklook_cache() -> Result<(), String> {
        let output = Command::new("qlmanage")
            .args(["-r", "cache"])
            .output()
            .map_err(|e| format!("Failed to execute qlmanage: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            Err("Failed to reset QuickLook cache".to_string())
        }
    }
}

fn dirs_cache_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join("Library").join("Caches"))
}

/// Recursively calculates directory size up to a max recursion depth.
fn calculate_dir_size(path: &Path, max_depth: usize) -> u64 {
    if max_depth == 0 {
        return 0;
    }

    let mut total_size = 0u64;
    let Ok(entries) = fs::read_dir(path) else {
        return 0;
    };

    for entry in entries.flatten() {
        let entry_path = entry.path();
        if let Ok(meta) = entry_path.symlink_metadata() {
            if meta.is_dir() {
                total_size += calculate_dir_size(&entry_path, max_depth - 1);
            } else {
                total_size += meta.len();
            }
        }
    }

    total_size
}

/// Converts bundle or folder names into friendly display labels.
fn friendly_cache_name(folder_name: &str) -> String {
    match folder_name {
        "Google" => "Google Chrome & Updater".to_string(),
        "Homebrew" => "Homebrew Packages & Downloads".to_string(),
        "CocoaPods" => "CocoaPods Dependency Cache".to_string(),
        "Steam" => "Steam Game Assets & Client".to_string(),
        "colima" => "Colima Container Cache".to_string(),
        "JNA" => "Java Native Access Temporary Files".to_string(),
        "JetPackCache" | "JetPackResourceBundles" => "JetBrains IDE Resources".to_string(),
        "DaVinci Resolve Welcome" => "DaVinci Resolve Assets".to_string(),
        name if name.starts_with("com.google.Chrome") => "Google Chrome Cache".to_string(),
        name if name.starts_with("com.spotify.client") => "Spotify Audio & Media Cache".to_string(),
        name if name.starts_with("com.microsoft.VSCode") => "Visual Studio Code Cache".to_string(),
        name if name.starts_with("com.tinyspeck.slackmacgap") => "Slack Desktop Cache".to_string(),
        name if name.starts_with("com.apple.dt.Xcode") => "Xcode Build & Module Cache".to_string(),
        name if name.starts_with("org.mozilla.firefox") => "Firefox Browser Cache".to_string(),
        name if name.starts_with("com.brave.Browser") => "Brave Browser Cache".to_string(),
        name if name.starts_with("com.apple.") => {
            let stripped = name.trim_start_matches("com.apple.");
            format!("Apple Service ({})", stripped)
        }
        other => other.to_string(),
    }
}

/// Returns whether a cache item is generally safe for routine clearing.
fn is_safe_cache_to_clean(folder_name: &str) -> bool {
    // Avoid critical macOS authentication, cloud sync, or system runtime caches
    let protected = [
        "CloudKit",
        "FamilyCircle",
        "familycircled",
        "PassKit",
        "GeoServices",
        "com.apple.containermanagerd",
        "com.apple.homed",
        "com.apple.HomeKit",
        "com.apple.Safari",
        "com.apple.Safari.SafeBrowsing",
    ];

    !protected.contains(&folder_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friendly_name_mapping() {
        assert_eq!(friendly_cache_name("Google"), "Google Chrome & Updater");
        assert_eq!(
            friendly_cache_name("com.spotify.client"),
            "Spotify Audio & Media Cache"
        );
        assert_eq!(
            friendly_cache_name("com.apple.dt.Xcode"),
            "Xcode Build & Module Cache"
        );
    }

    #[test]
    fn test_is_safe_cache() {
        assert!(is_safe_cache_to_clean("Google"));
        assert!(is_safe_cache_to_clean("Homebrew"));
        assert!(!is_safe_cache_to_clean("CloudKit"));
        assert!(!is_safe_cache_to_clean("FamilyCircle"));
    }
}
