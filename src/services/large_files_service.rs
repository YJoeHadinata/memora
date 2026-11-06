use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use crate::models::{LargeFileCategory, LargeFileItem, LargeFilesScanResult};

pub struct LargeFilesService;

impl LargeFilesService {
    /// Returns the current user's home directory.
    pub fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME").map(PathBuf::from)
    }

    /// Default search paths for large files across user home and all mounted data volumes.
    pub fn default_scan_roots() -> Vec<PathBuf> {
        let mut roots = Vec::new();

        if let Some(home) = Self::home_dir() {
            let candidate_names = [
                "Downloads",
                "Documents",
                "Movies",
                "Music",
                "Desktop",
                "Pictures",
                "Public",
            ];
            for name in candidate_names {
                let p = home.join(name);
                if p.is_dir() {
                    roots.push(p);
                }
            }

            // Also check for any custom top-level folders in user's home (excluding hidden files and Library)
            if let Ok(entries) = fs::read_dir(&home) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
                        continue;
                    }
                    let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    if name.starts_with('.') || name == "Library" {
                        continue;
                    }
                    if path.is_dir() && !roots.contains(&path) {
                        roots.push(path);
                    }
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
                    && name_str != "Macintosh HD"
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

    /// Scans candidate user folders for files larger than or equal to `min_bytes`.
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

        // Sort descending by file size (largest files first)
        found_items.sort_by_key(|f| std::cmp::Reverse(f.size_bytes));

        LargeFilesScanResult {
            files: found_items,
            scanned_at: Some(now),
        }
    }

    /// Reveals a given file in macOS Finder and selects it.
    pub fn reveal_in_finder(path: &Path) -> Result<(), String> {
        if !path.exists() {
            return Err("File does not exist".to_string());
        }

        let status = Command::new("open")
            .arg("-R")
            .arg(path)
            .status()
            .map_err(|e| format!("Failed to invoke open -R: {}", e))?;

        if status.success() {
            Ok(())
        } else {
            Err("Finder failed to reveal file".to_string())
        }
    }

    /// Moves a file safely into the macOS user Trash (`~/.Trash`).
    pub fn move_to_trash(path: &Path) -> Result<u64, String> {
        if !path.is_file() {
            return Err("Path is not a regular file".to_string());
        }

        let size = path.metadata().map(|m| m.len()).unwrap_or(0);

        // Try Finder AppleScript delete first (natively handles external volumes without EXDEV cross-device error)
        let path_str = path.to_string_lossy();
        let script = format!(
            r#"tell application "Finder" to delete POSIX file "{}""#,
            path_str
        );
        if let Ok(status) = Command::new("osascript").args(["-e", &script]).status() {
            if status.success() {
                return Ok(size);
            }
        }

        let Some(home) = Self::home_dir() else {
            return Err("Cannot locate home directory".to_string());
        };

        let trash_dir = home.join(".Trash");
        if !trash_dir.exists() {
            let _ = fs::create_dir_all(&trash_dir);
        }

        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
        let file_name = path
            .file_name()
            .ok_or_else(|| "Invalid file name".to_string())?;

        let mut dest = trash_dir.join(file_name);

        // Disambiguate if target already exists in Trash
        if dest.exists() {
            let timestamp = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("trashed_file");

            let new_name = if ext.is_empty() {
                format!("{}_{}", stem, timestamp)
            } else {
                format!("{}_{}.{}", stem, timestamp, ext)
            };
            dest = trash_dir.join(new_name);
        }

        // Try fast atomic rename first
        if fs::rename(path, &dest).is_ok() {
            return Ok(size);
        }

        // Fallback: cross-device copy & remove
        fs::copy(path, &dest).map_err(|e| format!("Failed to copy to trash: {}", e))?;
        fs::remove_file(path).map_err(|e| format!("Failed to remove source: {}", e))?;

        Ok(size)
    }

    /// Moves multiple files into Trash, returning the sum of freed bytes.
    pub fn move_multiple_to_trash(paths: &[PathBuf]) -> Result<u64, String> {
        let mut total_freed = 0u64;
        let mut errors = Vec::new();

        for path in paths {
            match Self::move_to_trash(path) {
                Ok(freed) => total_freed += freed,
                Err(e) => errors.push(format!("{}: {}", path.display(), e)),
            }
        }

        if errors.is_empty() {
            Ok(total_freed)
        } else {
            Err(errors.join(", "))
        }
    }
}

/// Helper function to traverse directories recursively with safety guardrails.
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

        // Skip symlinks to prevent loops and escaping user space
        if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
            continue;
        }

        let Some(file_name_os) = path.file_name() else {
            continue;
        };
        let file_name = file_name_os.to_string_lossy().to_string();

        // Skip hidden dot-files/folders (.git, .Trash, .DS_Store, etc.)
        if file_name.starts_with('.') {
            continue;
        }

        if path.is_dir() {
            // Skip noisy directories and dependency caches
            if file_name == "node_modules"
                || file_name == "target"
                || file_name == "venv"
                || file_name == ".build"
                || file_name == "Library"
            {
                continue;
            }

            scan_dir_recursive(&path, min_bytes, now, depth - 1, results);
        } else if path.is_file() {
            let Ok(meta) = path.metadata() else {
                continue;
            };

            let size_bytes = meta.len();
            if size_bytes >= min_bytes {
                let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let category = LargeFileCategory::from_extension(ext);

                let modified_days_ago = meta
                    .modified()
                    .ok()
                    .and_then(|m| now.duration_since(m).ok())
                    .map(|d| d.as_secs() / 86400)
                    .unwrap_or(0);

                results.push(LargeFileItem {
                    path,
                    name: file_name.to_string(),
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

        // Create a small file (100 bytes)
        let small_path = root.join("small.txt");
        let mut f1 = File::create(&small_path).unwrap();
        f1.write_all(&[0u8; 100]).unwrap();

        // Create a larger file (10 KB)
        let large_path = root.join("archive.zip");
        let mut f2 = File::create(&large_path).unwrap();
        f2.write_all(&[0u8; 10 * 1024]).unwrap();

        // Scan with threshold of 5 KB
        let result = LargeFilesService::scan_large_files(5 * 1024, Some(vec![root]));

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.files[0].name, "archive.zip");
        assert_eq!(result.files[0].category, LargeFileCategory::Archive);
        assert_eq!(result.files[0].size_bytes, 10 * 1024);
    }

    #[test]
    fn test_skip_hidden_and_node_modules() {
        let dir = tempdir().unwrap();
        let root = dir.path().to_path_buf();

        let hidden_dir = root.join(".hidden");
        fs::create_dir(&hidden_dir).unwrap();
        let mut f_hidden = File::create(hidden_dir.join("big.dmg")).unwrap();
        f_hidden.write_all(&[0u8; 10000]).unwrap();

        let node_dir = root.join("node_modules");
        fs::create_dir(&node_dir).unwrap();
        let mut f_node = File::create(node_dir.join("huge.pkg")).unwrap();
        f_node.write_all(&[0u8; 10000]).unwrap();

        let result = LargeFilesService::scan_large_files(1000, Some(vec![root]));
        assert_eq!(result.files.len(), 0);
    }

    #[test]
    fn test_default_scan_roots_non_empty() {
        let roots = LargeFilesService::default_scan_roots();
        assert!(
            !roots.is_empty(),
            "Default scan roots should include user folders or volumes"
        );
    }
}
