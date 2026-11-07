use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

use crate::models::{StartupItem, StartupItemKind, StartupItemStatus, StartupScanResult};
use crate::utils::fs_ops::{
    home_dir, move_single_to_trash, reveal_in_finder as open_in_finder, run_cmd_stdout,
};
use crate::utils::plist::{extract_plist_bool, extract_plist_string};

pub struct StartupService;

impl StartupService {
    pub fn scan_items() -> StartupScanResult {
        let start = Instant::now();
        let live_map = Self::fetch_launchctl_map();
        let mut items = Vec::new();
        if let Some(home) = home_dir() {
            let ua = home.join("Library/LaunchAgents");
            Self::scan_directory(&ua, StartupItemKind::UserAgent, &live_map, &mut items);
        }
        for (dir, kind) in [
            ("/Library/LaunchAgents", StartupItemKind::SystemAgent),
            ("/Library/LaunchDaemons", StartupItemKind::SystemDaemon),
        ] {
            Self::scan_directory(Path::new(dir), kind, &live_map, &mut items);
        }
        items.sort_by(|a, b| {
            let (a_ua, b_ua) = (
                a.kind == StartupItemKind::UserAgent,
                b.kind == StartupItemKind::UserAgent,
            );
            b.status
                .is_running()
                .cmp(&a.status.is_running())
                .then_with(|| b_ua.cmp(&a_ua))
                .then_with(|| a.display_name.cmp(&b.display_name))
        });
        StartupScanResult {
            items,
            scan_duration: start.elapsed(),
        }
    }

    pub fn toggle_item_enabled(item: &StartupItem) -> Result<bool, String> {
        if !item.kind.is_user_writable() {
            return Err("System services require administrator permissions.".into());
        }
        let path = &item.plist_path;
        if !path.exists() {
            return Err("Target plist file does not exist.".into());
        }
        let fname = path.file_name().and_then(|n| n.to_str());
        let name = fname.unwrap_or("agent.plist");
        if item.is_disabled_file {
            let clean = name.trim_end_matches(".disabled");
            let new_path = path.with_file_name(clean);
            fs::rename(path, &new_path).map_err(|e| format!("Failed to enable plist: {}", e))?;
            let _ = run_cmd_stdout("launchctl", &["load", new_path.to_str().unwrap_or("")]);
            Ok(true)
        } else {
            let _ = run_cmd_stdout("launchctl", &["unload", path.to_str().unwrap_or("")]);
            let new_path = path.with_file_name(format!("{}.disabled", name));
            fs::rename(path, &new_path).map_err(|e| format!("Failed to disable plist: {}", e))?;
            Ok(false)
        }
    }

    pub fn move_item_to_trash(item: &StartupItem) -> Result<(), String> {
        if !item.kind.is_user_writable() {
            return Err("System services cannot be deleted without root permissions.".into());
        }
        if !item.plist_path.exists() {
            return Ok(());
        }
        let p_str = item.plist_path.to_str().unwrap_or("");
        let _ = run_cmd_stdout("launchctl", &["unload", p_str]);
        move_single_to_trash(&item.plist_path)
    }

    pub fn reveal_in_finder(path: &Path) -> Result<(), String> {
        open_in_finder(path)
    }

    fn scan_directory(
        dir: &Path,
        kind: StartupItemKind,
        live_map: &HashMap<String, Option<u32>>,
        items: &mut Vec<StartupItem>,
    ) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(fname) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let is_disabled = fname.ends_with(".plist.disabled");
            if !fname.ends_with(".plist") && !is_disabled {
                continue;
            }
            let file_size = entry.metadata().map_or(0, |m| m.len());
            let base = fname.trim_end_matches(".disabled");
            let base = base.trim_end_matches(".plist");
            let label = extract_plist_string(&path, "Label").unwrap_or_else(|| base.to_string());
            let program_path = extract_plist_string(&path, "Program")
                .or_else(|| extract_plist_string(&path, "ProgramArguments.0"));
            let run_at_load = extract_plist_bool(&path, "RunAtLoad").unwrap_or(false);
            let keep_alive = extract_plist_bool(&path, "KeepAlive").unwrap_or(false);
            let status = if is_disabled {
                StartupItemStatus::Disabled
            } else if let Some(pid_opt) = live_map.get(&label) {
                pid_opt.map_or(StartupItemStatus::Loaded, StartupItemStatus::Running)
            } else {
                StartupItemStatus::NotLoaded
            };
            let display_name = Self::generate_friendly_name(&label, program_path.as_deref());
            items.push(StartupItem {
                id: label.clone(),
                label,
                display_name,
                program_path,
                plist_path: path,
                kind,
                status,
                run_at_load,
                keep_alive,
                is_disabled_file: is_disabled,
                file_size_bytes: file_size,
            });
        }
    }

    fn fetch_launchctl_map() -> HashMap<String, Option<u32>> {
        let mut map = HashMap::new();
        let Some(stdout) = run_cmd_stdout("launchctl", &["list"]) else {
            return map;
        };
        for line in stdout.lines().skip(1) {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let p_str = parts[0].trim();
                let pid = (p_str != "-").then(|| p_str.parse().ok()).flatten();
                map.insert(parts[2].trim().to_string(), pid);
            }
        }
        map
    }

    pub fn generate_friendly_name(label: &str, program_path: Option<&str>) -> String {
        let known = [
            ("keystone", "Google Keystone Updater"),
            ("GoogleUpdater", "Google Software Updater"),
            ("steamclean", "Steam Clean Agent"),
            ("postgresql", "PostgreSQL Database Service"),
            ("cloudflare", "Cloudflare WARP Service"),
            ("hermes", "Hermes Gateway"),
            ("serverdeck", "ServerDeck Background Agent"),
            ("fujifilm", "Fujifilm Webcam Server"),
        ];
        if let Some((_, friendly)) = known.into_iter().find(|(n, _)| label.contains(n)) {
            return friendly.to_string();
        }
        if let Some((prog, app_idx)) = program_path.and_then(|p| p.find(".app").map(|i| (p, i))) {
            if let Some(slash_idx) = prog[..app_idx].rfind('/') {
                let app_name = &prog[slash_idx + 1..app_idx];
                if !app_name.is_empty() {
                    return format!("{} Background Service", app_name);
                }
            }
        }
        let segs: Vec<&str> = label.split('.').collect();
        if segs.len() > 1 && !segs.last().unwrap_or(&"").is_empty() {
            return format!("{}.{}", segs[segs.len() - 2], segs[segs.len() - 1]);
        }
        label.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_friendly_name_heuristics() {
        let n1 = StartupService::generate_friendly_name("com.google.keystone.agent", None);
        assert_eq!(n1, "Google Keystone Updater");
        let p = Some("/Applications/Spotify.app/Contents/MacOS/Spotify");
        let n2 = StartupService::generate_friendly_name("com.spotify.client", p);
        assert_eq!(n2, "Spotify Background Service");
    }
}
