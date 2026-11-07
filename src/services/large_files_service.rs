use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::models::{LargeFileCategory, LargeFileItem, LargeFilesScanResult};
use crate::utils::fs_ops::{
    home_dir as get_home, move_paths_to_trash, move_single_to_trash,
    reveal_in_finder as open_finder,
};

pub struct LargeFilesService;

impl LargeFilesService {
    pub fn home_dir() -> Option<PathBuf> {
        get_home()
    }

    pub fn default_scan_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();
        if let Some(home) = Self::home_dir() {
            for name in [
                "Downloads",
                "Documents",
                "Movies",
                "Music",
                "Desktop",
                "Pictures",
                "Public",
            ] {
                let p = home.join(name);
                if p.is_dir() {
                    roots.push(p);
                }
            }
            if let Ok(entries) = fs::read_dir(&home) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if !name.starts_with('.')
                        && name != "Library"
                        && path.is_dir()
                        && !roots.contains(&path)
                    {
                        roots.push(path);
                    }
                }
            }
        }

        if let Ok(entries) = fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
                    continue;
                }
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                let excluded = ["Recovery", "Preboot", "VM", "Macintosh HD"];
                if !excluded.contains(&name_str.as_ref())
                    && !name_str.starts_with('.')
                    && path.is_dir()
                    && !roots.contains(&path)
                {
                    roots.push(path);
                }
            }
        }
        roots
    }

    pub fn scan_large_files(
        min_bytes: u64,
        custom_roots: Option<Vec<PathBuf>>,
    ) -> LargeFilesScanResult {
        let roots = custom_roots.unwrap_or_else(Self::default_scan_roots);
        let mut found_items = Vec::new();
        let now = SystemTime::now();

        for root in roots {
            scan_dir_recursive(&root, min_bytes, now, 10, &mut found_items);
        }
        found_items.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));
        LargeFilesScanResult {
            files: found_items,
            scanned_at: Some(now),
        }
    }

    pub fn reveal_in_finder(path: &Path) -> Result<(), String> {
        open_finder(path)
    }

    pub fn move_to_trash(path: &Path) -> Result<u64, String> {
        if !path.is_file() {
            return Err("Path is not a regular file".to_string());
        }
        let size = path.metadata().map_or(0, |m| m.len());
        move_single_to_trash(path).map(|()| size)
    }

    pub fn move_multiple_to_trash(paths: &[PathBuf]) -> Result<u64, String> {
        move_paths_to_trash(paths)
    }
}

fn scan_dir_recursive(
    dir: &Path,
    min_bytes: u64,
    now: SystemTime,
    depth: usize,
    results: &mut Vec<LargeFileItem>,
) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
            continue;
        }
        let Some(file_name) = path.file_name().map(|n| n.to_string_lossy().to_string()) else {
            continue;
        };
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            let skip = ["node_modules", "target", "venv", ".build", "Library"];
            if !skip.contains(&file_name.as_str()) {
                scan_dir_recursive(&path, min_bytes, now, depth - 1, results);
            }
        } else if path.is_file() {
            let Ok(meta) = path.metadata() else { continue };
            let size_bytes = meta.len();
            if size_bytes >= min_bytes {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let category = LargeFileCategory::from_extension(ext);
                let modified_days_ago = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .map_or(0, |d| d.as_secs() / 86400);
                results.push(LargeFileItem {
                    path,
                    name: file_name,
                    size_bytes,
                    category,
                    modified_days_ago,
                    selected: false,
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_scan_large_files_in_temp_dir() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();
        File::create(root.join("small.txt"))
            .unwrap()
            .write_all(&[0u8; 100])
            .unwrap();
        File::create(root.join("archive.zip"))
            .unwrap()
            .write_all(&[0u8; 10 * 1024])
            .unwrap();

        let result = LargeFilesService::scan_large_files(5 * 1024, Some(vec![root]));
        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "archive.zip");
    }
}
