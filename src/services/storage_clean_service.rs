use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::models::{InstallerItem, StorageJunkReport, TrashReport};

pub struct StorageCleanService;

impl StorageCleanService {
    /// Returns the current user's home directory path.
    fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }

    /// Scans the macOS Trash directory using Finder AppleScript (bypassing macOS TCC restrictions)
    /// with fallback to direct filesystem inspection.
    pub fn scan_trash() -> TrashReport {
        if let Some(report) = Self::scan_trash_via_finder() {
            return report;
        }

        Self::scan_trash_filesystem()
    }

    fn scan_trash_via_finder() -> Option<TrashReport> {
        let count_out = Command::new("osascript")
            .args([
                "-e",
                "tell application \"Finder\" to count every item of trash",
            ])
            .output()
            .ok()?;

        if !count_out.status.success() {
            return None;
        }

        let count_str = String::from_utf8_lossy(&count_out.stdout)
            .trim()
            .to_string();
        let item_count = count_str.parse::<usize>().unwrap_or(0);

        if item_count == 0 {
            return Some(TrashReport {
                item_count: 0,
                total_bytes: 0,
            });
        }

        let size_out = Command::new("osascript")
            .args([
                "-e",
                "tell application \"Finder\" to get physical size of items of trash",
            ])
            .output()
            .ok()?;

        let mut total_bytes = 0u64;
        if size_out.status.success() {
            let size_str = String::from_utf8_lossy(&size_out.stdout);
            for part in size_str.split(',') {
                let trimmed = part.trim();
                if let Ok(bytes_f) = trimmed.replace(',', ".").parse::<f64>() {
                    total_bytes += bytes_f.round() as u64;
                }
            }
        }

        Some(TrashReport {
            item_count,
            total_bytes,
        })
    }

    fn scan_trash_filesystem() -> TrashReport {
        let Some(trash_dir) = Self::home_dir().map(|h| h.join(".Trash")) else {
            return TrashReport::default();
        };

        if !trash_dir.is_dir() {
            return TrashReport::default();
        }

        let Ok(entries) = fs::read_dir(&trash_dir) else {
            return TrashReport::default();
        };

        let mut item_count = 0;
        let mut total_bytes = 0u64;

        for entry in entries.flatten() {
            let path = entry.path();
            item_count += 1;
            total_bytes += calculate_path_size(&path);
        }

        TrashReport {
            item_count,
            total_bytes,
        }
    }

    /// Empties items inside the macOS Trash via Finder AppleScript with filesystem fallback.
    pub fn empty_trash() -> Result<u64, String> {
        let pre_report = Self::scan_trash();
        let freed_bytes = pre_report.total_bytes;

        let status = Command::new("osascript")
            .args(["-e", "tell application \"Finder\" to empty trash"])
            .status();

        match status {
            Ok(s) if s.success() => Ok(freed_bytes),
            _ => Self::empty_trash_filesystem(),
        }
    }

    fn empty_trash_filesystem() -> Result<u64, String> {
        let Some(trash_dir) = Self::home_dir().map(|h| h.join(".Trash")) else {
            return Err("Cannot determine home directory".to_string());
        };

        if !trash_dir.is_dir() {
            return Ok(0);
        }

        let entries =
            fs::read_dir(&trash_dir).map_err(|e| format!("Failed to read ~/.Trash: {}", e))?;

        let mut freed_bytes = 0u64;
        for entry in entries.flatten() {
            let path = entry.path();
            let size = calculate_path_size(&path);
            if path.is_dir() {
                if fs::remove_dir_all(&path).is_ok() {
                    freed_bytes += size;
                }
            } else if fs::remove_file(&path).is_ok() {
                freed_bytes += size;
            }
        }

        Ok(freed_bytes)
    }

    /// Scans user folders (Downloads, Desktop, Documents) and external volumes for disk images and installers.
    pub fn scan_installers() -> Vec<InstallerItem> {
        let mut roots = Vec::new();

        if let Some(home) = Self::home_dir() {
            for sub in ["Downloads", "Desktop", "Documents"] {
                let p = home.join(sub);
                if p.is_dir() {
                    roots.push(p);
                }
            }
        }

        // Also check mounted data volumes in /Volumes (e.g. /Volumes/Adata900)
        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
                    continue;
                }
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str != "Recovery"
                    && name_str != "Preboot"
                    && name_str != "VM"
                    && path.is_dir()
                {
                    roots.push(path);
                }
            }
        }

        let now = SystemTime::now();
        let mut items = Vec::new();

        for root in roots {
            let Ok(entries) = fs::read_dir(&root) else {
                continue;
            };

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }

                let Some(ext) = path.extension().and_then(|s| s.to_str()) else {
                    continue;
                };

                let is_installer = ext.eq_ignore_ascii_case("dmg")
                    || ext.eq_ignore_ascii_case("pkg")
                    || ext.eq_ignore_ascii_case("iso");

                if !is_installer {
                    continue;
                }

                let Ok(meta) = path.metadata() else {
                    continue;
                };

                let size_bytes = meta.len();
                if size_bytes == 0 {
                    continue;
                }

                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default();

                let modified_days_ago = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .map(|d| d.as_secs() / 86400)
                    .unwrap_or(0);

                let selected = true;

                items.push(InstallerItem {
                    path,
                    name,
                    size_bytes,
                    modified_days_ago,
                    selected,
                });
            }
        }

        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        items
    }

    /// Safely deletes selected installer files.
    pub fn clean_installers(paths: &[PathBuf]) -> Result<u64, String> {
        let mut total_freed = 0u64;
        for path in paths {
            if path.is_file() {
                let path_str = path.to_string_lossy();
                if path_str.starts_with("/System") || path_str.starts_with("/Library") {
                    continue;
                }
                let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                if fs::remove_file(path).is_ok() {
                    total_freed += size;
                }
            }
        }

        Ok(total_freed)
    }

    /// Queries APFS local Time Machine snapshots via `tmutil listlocalsnapshots /`.
    pub fn scan_apfs_snapshots() -> Vec<String> {
        let Ok(output) = Command::new("tmutil")
            .args(["listlocalsnapshots", "/"])
            .output()
        else {
            return Vec::new();
        };

        if !output.status.success() {
            return Vec::new();
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut snapshots = Vec::new();

        for line in stdout.lines() {
            let trimmed = line.trim();
            // Typical line format: com.apple.TimeMachine.2023-11-01-120000.local
            if let Some(start) = trimmed.find("com.apple.TimeMachine.") {
                let date_part = &trimmed[start + "com.apple.TimeMachine.".len()..];
                let date_str = date_part.trim_end_matches(".local");
                if !date_str.is_empty() {
                    snapshots.push(date_str.to_string());
                }
            }
        }

        snapshots
    }

    /// Deletes local APFS Time Machine snapshots to reclaim thin-provisioned storage.
    pub fn clean_apfs_snapshots() -> Result<usize, String> {
        let snapshots = Self::scan_apfs_snapshots();
        if snapshots.is_empty() {
            return Ok(0);
        }

        let mut deleted_count = 0;
        for date_str in &snapshots {
            let res = Command::new("tmutil")
                .args(["deletelocalsnapshots", date_str])
                .output();

            let success = match res {
                Ok(out) if out.status.success() => true,
                _ => {
                    // Try elevated via osascript if standard tmutil requires administrative privileges
                    let script = format!(
                        "do shell script \"tmutil deletelocalsnapshots {}\" with administrator privileges",
                        date_str
                    );
                    Command::new("osascript")
                        .args(["-e", &script])
                        .output()
                        .map(|out| out.status.success())
                        .unwrap_or(false)
                }
            };

            if success {
                deleted_count += 1;
            }
        }

        Ok(deleted_count)
    }

    /// Removes unavailable iOS simulator devices left over from previous Xcode versions.
    pub fn clean_unavailable_simulators() -> Result<String, String> {
        let output = Command::new("xcrun")
            .args(["simctl", "delete", "unavailable"])
            .output()
            .map_err(|e| format!("Failed to execute xcrun: {}", e))?;

        if output.status.success() {
            Ok("Successfully purged unavailable iOS simulator runtimes.".to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(if stderr.is_empty() {
                "Failed to prune unavailable simulators".to_string()
            } else {
                stderr.trim().to_string()
            })
        }
    }

    /// Runs a full scan of all storage junk categories.
    pub fn scan_all_junk() -> StorageJunkReport {
        let trash = Self::scan_trash();
        let installers = Self::scan_installers();
        let apfs_snapshots = Self::scan_apfs_snapshots();

        StorageJunkReport {
            trash,
            installers,
            apfs_snapshots,
            scanned_at: Some(SystemTime::now()),
        }
    }
}

/// Recursively calculates path size up to max depth 4.
fn calculate_path_size(path: &Path) -> u64 {
    if path.is_file() {
        return path.metadata().map(|m| m.len()).unwrap_or(0);
    }

    if !path.is_dir() {
        return 0;
    }

    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += calculate_path_size_depth(&p, 3);
            } else {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

fn calculate_path_size_depth(path: &Path, depth: usize) -> u64 {
    if depth == 0 {
        return 0;
    }
    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                total += calculate_path_size_depth(&p, depth - 1);
            } else {
                total += p.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_trash_does_not_panic() {
        let report = StorageCleanService::scan_trash();
        // Should execute cleanly on macOS without crashing
        let _ = report.item_count;
        let _ = report.total_bytes;
    }

    #[test]
    fn test_scan_installers_does_not_panic() {
        let installers = StorageCleanService::scan_installers();
        for item in installers {
            assert!(
                item.name.ends_with(".dmg")
                    || item.name.ends_with(".pkg")
                    || item.name.ends_with(".iso")
            );
        }
    }
}
