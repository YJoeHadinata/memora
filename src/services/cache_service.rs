use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, SystemTime};

use crate::models::cache::{friendly_cache_name, is_safe_cache_to_clean, DEV_CACHE_TARGETS};
use crate::models::{CacheItem, CacheScanResult, PurgeRamResult};
use crate::services::MemoryService;
use crate::utils::fs_ops::{calculate_path_size, home_dir};

pub struct CacheService {
    memory_service: Arc<MemoryService>,
}

impl CacheService {
    pub fn new(memory_service: Arc<MemoryService>) -> Self {
        Self { memory_service }
    }

    pub fn get_user_cache_dir() -> Option<PathBuf> {
        home_dir().map(|h| h.join("Library").join("Caches"))
    }

    pub fn scan_user_caches() -> CacheScanResult {
        let Some(read_dir) = Self::get_user_cache_dir().and_then(|d| fs::read_dir(d).ok()) else {
            return CacheScanResult::default();
        };

        let mut items = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let file_name = path
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            let size_bytes = calculate_path_size(&path);
            if size_bytes == 0 {
                continue;
            }
            let is_safe = is_safe_cache_to_clean(&file_name);
            items.push(CacheItem {
                name: friendly_cache_name(&file_name),
                description: format!("Stored at ~/Library/Caches/{}", file_name),
                id: file_name,
                path,
                size_bytes,
                is_safe,
                selected: is_safe && size_bytes > 10 * 1024 * 1024,
            });
        }

        if let Some(h) = home_dir() {
            for (id, name, rel, desc) in DEV_CACHE_TARGETS {
                let path = h.join(rel);
                if path.is_dir() {
                    let size_bytes = calculate_path_size(&path);
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

        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        let total_bytes = items.iter().map(|item| item.size_bytes).sum();
        CacheScanResult {
            items,
            total_bytes,
            scanned_at: Some(SystemTime::now()),
        }
    }

    pub fn clean_cache_items(paths: &[PathBuf]) -> Result<u64, String> {
        let mut total_freed = 0u64;
        for dir in paths {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let sub = entry.path();
                let size = calculate_path_size(&sub);
                let ok = if sub.is_dir() {
                    fs::remove_dir_all(&sub).is_ok()
                } else {
                    fs::remove_file(&sub).is_ok()
                };
                if ok {
                    total_freed += size;
                }
            }
        }
        Ok(total_freed)
    }

    pub fn purge_system_ram_cache(&self) -> Result<PurgeRamResult, String> {
        let before = self
            .memory_service
            .snapshot()
            .map_err(|e| format!("Failed to capture initial snapshot: {}", e))?;
        let mut elevated = false;
        let ok = match Command::new("/usr/sbin/purge").output() {
            Ok(out) if out.status.success() => true,
            _ => {
                elevated = true;
                let script = r#"do shell script "/usr/sbin/purge" with administrator privileges"#;
                Command::new("osascript")
                    .args(["-e", script])
                    .output()
                    .map_err(|e| format!("Failed to invoke authorization prompt: {}", e))?
                    .status
                    .success()
            }
        };
        if !ok {
            return Err("Cache purge was cancelled or unauthorized by user.".to_string());
        }

        thread::sleep(Duration::from_millis(350));
        let after = self
            .memory_service
            .snapshot()
            .map_err(|e| format!("Failed to capture post-purge snapshot: {}", e))?;
        let reclaimed_bytes = before.used_bytes.saturating_sub(after.used_bytes).max(
            before
                .cached_files_bytes
                .saturating_sub(after.cached_files_bytes),
        );
        let method = if elevated {
            "macOS System Purge (with Admin Authorization)"
        } else {
            "macOS System Purge"
        };

        Ok(PurgeRamResult {
            reclaimed_bytes,
            before_used_bytes: before.used_bytes,
            after_used_bytes: after.used_bytes,
            success: true,
            message: format!(
                "{} flushed inactive buffer cache successfully. Reclaimed {} of RAM.",
                method,
                crate::utils::format::format_bytes(reclaimed_bytes)
            ),
        })
    }

    pub fn flush_dns_cache() -> Result<(), String> {
        let out = Command::new("dscacheutil")
            .arg("-flushcache")
            .output()
            .map_err(|e| format!("Failed to execute dscacheutil: {}", e))?;
        out.status
            .success()
            .then_some(())
            .ok_or_else(|| "Failed to flush DNS cache".to_string())
    }

    pub fn flush_quicklook_cache() -> Result<(), String> {
        let out = Command::new("qlmanage")
            .args(["-r", "cache"])
            .output()
            .map_err(|e| format!("Failed to execute qlmanage: {}", e))?;
        out.status
            .success()
            .then_some(())
            .ok_or_else(|| "Failed to reset QuickLook cache".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_helpers() {
        assert_eq!(friendly_cache_name("Google"), "Google Chrome & Updater");
        assert!(is_safe_cache_to_clean("Google"));
        assert!(!is_safe_cache_to_clean("CloudKit"));
    }
}
