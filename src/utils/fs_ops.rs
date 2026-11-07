use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Returns the current user's home directory from `HOME`.
pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// Runs a subprocess and returns its stdout as a String if it exits with 0.
pub fn run_cmd_stdout(prog: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(prog).args(args).output().ok()?;
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}

/// Recursively calculates the byte size of a file or directory without following symlinks.
pub fn calculate_path_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_symlink() {
        return 0;
    }
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }

    let mut total = 0u64;
    let mut stack = vec![path.to_path_buf()];

    while let Some(current) = stack.pop() {
        let Ok(entries) = fs::read_dir(&current) else {
            continue;
        };
        for entry in entries.flatten() {
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_file() {
                if let Ok(m) = entry.metadata() {
                    total = total.saturating_add(m.len());
                }
            } else if ft.is_dir() {
                stack.push(entry.path());
            }
        }
    }
    total
}

/// Reveals a file or directory in macOS Finder (`open -R`).
pub fn reveal_in_finder(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }
    let output = Command::new("open")
        .arg("-R")
        .arg(path)
        .output()
        .map_err(|e| format!("Failed to launch Finder: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

/// Safely moves a single file or directory to macOS Trash using Finder AppleScript,
/// `~/.Trash` rename fallback, or direct removal as a final fallback.
pub fn move_single_to_trash(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }

    let escaped = path.display().to_string().replace('"', "\\\"");
    let script = format!(
        "tell application \"Finder\" to delete POSIX file \"{}\"",
        escaped
    );
    if run_cmd_stdout("osascript", &["-e", &script]).is_some() {
        return Ok(());
    }

    if let Some(home) = home_dir() {
        let trash = home.join(".Trash");
        if let Some(name) = path.file_name() {
            let mut dest = trash.join(name);
            if dest.exists() {
                let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("item");
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                let unique = if ext.is_empty() {
                    format!("{}_{}", stem, ts)
                } else {
                    format!("{}_{}.{}", stem, ts, ext)
                };
                dest = trash.join(unique);
            }
            if fs::rename(path, &dest).is_ok() {
                return Ok(());
            }
        }
    }

    if path.is_dir() {
        fs::remove_dir_all(path).map_err(|e| e.to_string())
    } else {
        fs::remove_file(path).map_err(|e| e.to_string())
    }
}

/// Safely moves multiple paths to macOS Trash and returns the total bytes reclaimed.
pub fn move_paths_to_trash(paths: &[PathBuf]) -> Result<u64, String> {
    let mut total_freed = 0u64;
    let mut errors = Vec::new();

    for path in paths {
        if !path.exists() {
            continue;
        }
        let size = calculate_path_size(path);
        match move_single_to_trash(path) {
            Ok(()) => total_freed = total_freed.saturating_add(size),
            Err(err) => errors.push(format!("{}: {}", path.display(), err)),
        }
    }

    if errors.is_empty() {
        Ok(total_freed)
    } else {
        Err(errors.join("\n"))
    }
}
