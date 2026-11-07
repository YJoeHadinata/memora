use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::models::{InstallerItem, StorageJunkReport, TrashReport};
use crate::utils::fs_ops::{calculate_path_size, home_dir, run_cmd_stdout};

pub struct StorageCleanService;

impl StorageCleanService {
    pub fn scan_trash() -> TrashReport {
        Self::scan_trash_via_finder().unwrap_or_else(Self::scan_trash_filesystem)
    }

    fn scan_trash_via_finder() -> Option<TrashReport> {
        let count_cmd = "tell application \"Finder\" to count every item of trash";
        let s = run_cmd_stdout("osascript", &["-e", count_cmd])?;
        let n = s.trim().parse().unwrap_or(0);
        if n == 0 {
            return Some(TrashReport::default());
        }
        let sz_cmd = "tell application \"Finder\" to get physical size of items of trash";
        let sz_out = run_cmd_stdout("osascript", &["-e", sz_cmd])?;
        let b = sz_out
            .split(',')
            .filter_map(|p| p.trim().replace(',', ".").parse::<f64>().ok())
            .map(|v| v.round() as u64)
            .sum();
        Some(TrashReport {
            item_count: n,
            total_bytes: b,
        })
    }

    fn scan_trash_filesystem() -> TrashReport {
        let Some(entries) = home_dir()
            .map(|h| h.join(".Trash"))
            .and_then(|d| fs::read_dir(d).ok())
        else {
            return TrashReport::default();
        };
        let (mut n, mut b) = (0, 0u64);
        for entry in entries.flatten() {
            n += 1;
            b += calculate_path_size(&entry.path());
        }
        TrashReport {
            item_count: n,
            total_bytes: b,
        }
    }

    pub fn empty_trash() -> Result<u64, String> {
        let freed_bytes = Self::scan_trash().total_bytes;
        let cmd = "tell application \"Finder\" to empty trash";
        if run_cmd_stdout("osascript", &["-e", cmd]).is_some() {
            return Ok(freed_bytes);
        }
        let trash_dir = home_dir().ok_or("Cannot determine home")?.join(".Trash");
        if !trash_dir.is_dir() {
            return Ok(0);
        }
        let entries = fs::read_dir(&trash_dir).map_err(|e| format!("Read ~/.Trash: {}", e))?;
        let mut freed = 0u64;
        for entry in entries.flatten() {
            let path = entry.path();
            let size = calculate_path_size(&path);
            let ok = if path.is_dir() {
                fs::remove_dir_all(&path).is_ok()
            } else {
                fs::remove_file(&path).is_ok()
            };
            if ok {
                freed += size;
            }
        }
        Ok(freed)
    }

    pub fn scan_installers() -> Vec<InstallerItem> {
        let mut roots = Vec::new();
        if let Some(home) = home_dir() {
            for s in ["Downloads", "Desktop", "Documents"] {
                let p = home.join(s);
                if p.is_dir() {
                    roots.push(p);
                }
            }
        }
        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let is_sym = entry.file_type().map_or(true, |ft| ft.is_symlink());
                let path = entry.path();
                let name = entry.file_name();
                let skip = ["Recovery", "Preboot", "VM"].contains(&name.to_string_lossy().as_ref());
                if !is_sym && !skip && path.is_dir() {
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
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let exts = ["dmg", "pkg", "iso"];
                let is_pkg = exts.iter().any(|e| ext.eq_ignore_ascii_case(e));
                let Ok(meta) = path.metadata() else { continue };
                if !path.is_file() || !is_pkg || meta.len() == 0 {
                    continue;
                }
                let modified_days_ago = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .map_or(0, |d| d.as_secs() / 86400);
                let name = path
                    .file_name()
                    .map(|s| s.to_string_lossy().into())
                    .unwrap_or_default();
                items.push(InstallerItem {
                    name,
                    path,
                    size_bytes: meta.len(),
                    modified_days_ago,
                    selected: true,
                });
            }
        }
        items.sort_by_key(|item| std::cmp::Reverse(item.size_bytes));
        items
    }

    pub fn clean_installers(paths: &[PathBuf]) -> Result<u64, String> {
        let mut total_freed = 0u64;
        for path in paths {
            let s = path.to_string_lossy();
            if path.is_file() && !s.starts_with("/System") && !s.starts_with("/Library") {
                let size = path.metadata().map_or(0, |m| m.len());
                if fs::remove_file(path).is_ok() {
                    total_freed += size;
                }
            }
        }
        Ok(total_freed)
    }

    pub fn scan_apfs_snapshots() -> Vec<String> {
        let Some(stdout) = run_cmd_stdout("tmutil", &["listlocalsnapshots", "/"]) else {
            return Vec::new();
        };
        let pfx = "com.apple.TimeMachine.";
        stdout
            .lines()
            .filter_map(|l| {
                let s = l.trim()[l.trim().find(pfx)? + pfx.len()..].trim_end_matches(".local");
                (!s.is_empty()).then(|| s.to_string())
            })
            .collect()
    }

    pub fn clean_apfs_snapshots() -> Result<usize, String> {
        let mut deleted = 0;
        for date_str in &Self::scan_apfs_snapshots() {
            let cmd = format!(
                "do shell script \"tmutil deletelocalsnapshots {}\" with administrator privileges",
                date_str
            );
            if run_cmd_stdout("tmutil", &["deletelocalsnapshots", date_str]).is_some()
                || run_cmd_stdout("osascript", &["-e", &cmd]).is_some()
            {
                deleted += 1;
            }
        }
        Ok(deleted)
    }

    pub fn clean_unavailable_simulators() -> Result<String, String> {
        run_cmd_stdout("xcrun", &["simctl", "delete", "unavailable"])
            .map(|_| "Successfully purged unavailable iOS simulator runtimes.".into())
            .ok_or_else(|| "Failed to prune unavailable simulators".into())
    }

    pub fn scan_all_junk() -> StorageJunkReport {
        StorageJunkReport {
            trash: Self::scan_trash(),
            installers: Self::scan_installers(),
            apfs_snapshots: Self::scan_apfs_snapshots(),
            scanned_at: Some(SystemTime::now()),
        }
    }
}
