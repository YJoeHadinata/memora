use std::fs;
use std::path::{Path, PathBuf};

use crate::models::{AppLeftoverItem, AppLeftoverKind, AppUninstallScanResult, InstalledAppItem};
use crate::utils::fs_ops::{
    calculate_path_size as calc_size, home_dir, move_paths_to_trash as trash_paths,
};
use crate::utils::plist::extract_plist_string;

pub struct AppUninstallService;

impl AppUninstallService {
    pub fn scan_installed_apps() -> AppUninstallScanResult {
        let mut apps = Vec::new();
        let mut app_dirs = vec![(PathBuf::from("/Applications"), false)];

        if let Some(ua) = home_dir()
            .map(|h| h.join("Applications"))
            .filter(|p| p.is_dir())
        {
            app_dirs.push((ua, false));
        }
        let sys_apps = PathBuf::from("/System/Applications");
        if sys_apps.is_dir() {
            app_dirs.push((sys_apps, true));
        }

        for (dir, is_system_root) in app_dirs {
            Self::scan_apps_in_dir(&dir, is_system_root, &mut apps);
        }

        apps.sort_by_key(|a| std::cmp::Reverse(a.total_size_bytes()));
        apps.dedup_by(|a, b| a.app_path == b.app_path);
        AppUninstallScanResult { apps }
    }

    fn scan_apps_in_dir(dir: &Path, is_system_root: bool, results: &mut Vec<InstalledAppItem>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|ft| ft.is_symlink()).unwrap_or(true) {
                continue;
            }
            let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if fname.starts_with('.') || !path.is_dir() {
                continue;
            }

            if fname.ends_with(".app") {
                if let Some(item) = Self::inspect_app_bundle(&path, is_system_root) {
                    results.push(item);
                }
            } else if !fname.contains('.') {
                let Ok(sub_entries) = fs::read_dir(&path) else {
                    continue;
                };
                for sub in sub_entries.flatten() {
                    let sub_p = sub.path();
                    let is_app = sub_p
                        .file_name()
                        .and_then(|n| n.to_str())
                        .is_some_and(|n| n.ends_with(".app"));
                    if sub_p.is_dir() && is_app {
                        if let Some(item) = Self::inspect_app_bundle(&sub_p, is_system_root) {
                            results.push(item);
                        }
                    }
                }
            }
        }
    }

    pub fn inspect_app_bundle(app_path: &Path, is_system_root: bool) -> Option<InstalledAppItem> {
        let info_plist = app_path.join("Contents/Info.plist");
        if !info_plist.is_file() {
            return None;
        }

        let def_name = app_path
            .file_stem()
            .and_then(|n| n.to_str())
            .unwrap_or("App")
            .to_string();
        let bundle_id = extract_plist_string(&info_plist, "CFBundleIdentifier").unwrap_or_default();
        let display_name = extract_plist_string(&info_plist, "CFBundleDisplayName")
            .or_else(|| extract_plist_string(&info_plist, "CFBundleName"))
            .unwrap_or(def_name);
        let version = extract_plist_string(&info_plist, "CFBundleShortVersionString")
            .or_else(|| extract_plist_string(&info_plist, "CFBundleVersion"))
            .unwrap_or_else(|| "1.0".to_string());

        let is_system_app = is_system_root
            || app_path.starts_with("/System")
            || (bundle_id.starts_with("com.apple.")
                && app_path.starts_with("/Applications/Safari.app"));

        let app_size = calc_size(app_path);
        let mut leftovers = vec![AppLeftoverItem {
            path: app_path.to_path_buf(),
            kind: AppLeftoverKind::ApplicationBundle,
            size_bytes: app_size,
            selected: !is_system_app,
        }];

        if let Some(lib) = home_dir().map(|h| h.join("Library")).filter(|l| l.is_dir()) {
            Self::collect_leftovers(&lib, &display_name, &bundle_id, &mut leftovers);
        }

        Some(InstalledAppItem {
            name: display_name,
            bundle_id,
            version,
            app_path: app_path.to_path_buf(),
            app_size_bytes: app_size,
            is_system_app,
            leftovers,
        })
    }

    fn collect_leftovers(
        library: &Path,
        app_name: &str,
        bundle_id: &str,
        leftovers: &mut Vec<AppLeftoverItem>,
    ) {
        let mut candidates = Vec::new();
        let dirs = [
            (
                "Application Support",
                AppLeftoverKind::ApplicationSupport,
                true,
                true,
            ),
            ("Caches", AppLeftoverKind::Caches, true, true),
            ("Containers", AppLeftoverKind::Containers, false, true),
            ("HTTPStorages", AppLeftoverKind::HTTPStorages, false, true),
            ("WebKit", AppLeftoverKind::WebKit, false, true),
            ("Logs", AppLeftoverKind::Logs, true, true),
        ];

        for (folder, kind, by_name, by_id) in dirs {
            let base = library.join(folder);
            if by_name && !app_name.is_empty() {
                candidates.push((base.join(app_name), kind));
            }
            if by_id && !bundle_id.is_empty() && bundle_id != app_name {
                candidates.push((base.join(bundle_id), kind));
            }
        }
        if !bundle_id.is_empty() {
            let pref = library
                .join("Preferences")
                .join(format!("{}.plist", bundle_id));
            let state = library
                .join("Saved Application State")
                .join(format!("{}.savedState", bundle_id));
            candidates.push((pref, AppLeftoverKind::Preferences));
            candidates.push((state, AppLeftoverKind::SavedState));
        }

        for (path, kind) in candidates {
            if path.exists() && !leftovers.iter().any(|l| l.path == path) {
                leftovers.push(AppLeftoverItem {
                    size_bytes: calc_size(&path),
                    path,
                    kind,
                    selected: true,
                });
            }
        }
    }

    pub fn move_paths_to_trash(paths: &[PathBuf]) -> Result<u64, String> {
        trash_paths(paths)
    }

    pub fn calculate_path_size(path: &Path) -> u64 {
        calc_size(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_path_size_file() {
        let tmp = std::env::temp_dir().join("memora_uninstall_test_file.txt");
        let _ = fs::write(&tmp, b"hello memora uninstaller");
        assert_eq!(AppUninstallService::calculate_path_size(&tmp), 24);
        let _ = fs::remove_file(&tmp);
    }
}
