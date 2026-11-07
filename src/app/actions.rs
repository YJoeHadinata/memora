use std::thread;

use crate::app::{BackgroundMessage, MemoraApp};
use crate::services::{
    AppUninstallService, CacheService, LargeFilesService, StartupService, StorageCleanService,
};
use crate::ui::components::{ConfirmAction, ConfirmDialogState};
use crate::ui::CleanupActionRequest;

impl MemoraApp {
    pub(crate) fn spawn_bg(&self, f: impl FnOnce() -> BackgroundMessage + Send + 'static) {
        let tx = self.sender.clone();
        thread::spawn(move || {
            let _ = tx.send(f());
        });
    }

    fn set_confirm(&mut self, action: ConfirmAction) {
        self.confirm_dialog = Some(ConfirmDialogState {
            process: None,
            action,
        });
    }

    pub(crate) fn handle_cleanup_action(&mut self, req: CleanupActionRequest) {
        match req {
            CleanupActionRequest::ScanStorageJunk => {
                self.cache_view_state.is_scanning_storage = true;
                self.spawn_bg(|| {
                    BackgroundMessage::StorageJunkScanned(StorageCleanService::scan_all_junk())
                });
            }
            CleanupActionRequest::EmptyTrash => {
                if let Some(ref junk) = self.cache_view_state.storage_junk {
                    self.set_confirm(ConfirmAction::EmptyTrash {
                        item_count: junk.trash.item_count,
                        total_bytes: junk.trash.total_bytes,
                    });
                }
            }
            CleanupActionRequest::DeleteSelectedInstallers(paths) => {
                let total_bytes = paths
                    .iter()
                    .map(|p| p.metadata().map_or(0, |m| m.len()))
                    .sum();
                self.set_confirm(ConfirmAction::DeleteInstallers {
                    count: paths.len(),
                    total_bytes,
                    paths,
                });
            }
            CleanupActionRequest::PurgeApfsSnapshots => {
                self.cache_view_state.is_cleaning_storage = true;
                self.spawn_bg(|| {
                    BackgroundMessage::ApfsSnapshotsCleaned(
                        StorageCleanService::clean_apfs_snapshots(),
                    )
                });
            }
            CleanupActionRequest::PruneUnavailableSimulators => {
                self.spawn_bg(|| {
                    BackgroundMessage::SimulatorsPruned(
                        StorageCleanService::clean_unavailable_simulators(),
                    )
                });
            }
            CleanupActionRequest::ScanLargeFiles(min_bytes) => {
                self.cache_view_state.is_scanning_large_files = true;
                self.spawn_bg(move || {
                    BackgroundMessage::LargeFilesScanned(LargeFilesService::scan_large_files(
                        min_bytes, None,
                    ))
                });
            }
            CleanupActionRequest::RevealLargeFileInFinder(p)
            | CleanupActionRequest::RevealAppPathInFinder(p) => {
                let _ = LargeFilesService::reveal_in_finder(&p);
            }
            CleanupActionRequest::DeleteSelectedLargeFiles(paths) => {
                let total_bytes = paths
                    .iter()
                    .map(|p| p.metadata().map_or(0, |m| m.len()))
                    .sum();
                self.set_confirm(ConfirmAction::DeleteLargeFiles {
                    count: paths.len(),
                    total_bytes,
                    paths,
                });
            }
            CleanupActionRequest::ScanInstalledApps => {
                self.cache_view_state.is_scanning_uninstaller = true;
                self.spawn_bg(|| {
                    BackgroundMessage::AppsScanned(AppUninstallService::scan_installed_apps())
                });
            }
            CleanupActionRequest::UninstallAppSelected { app_name, paths } => {
                let total_bytes = paths
                    .iter()
                    .map(|p| AppUninstallService::calculate_path_size(p))
                    .sum();
                self.set_confirm(ConfirmAction::UninstallApp {
                    app_name,
                    count: paths.len(),
                    total_bytes,
                    paths,
                });
            }
            CleanupActionRequest::ScanStartupItems => {
                self.cache_view_state.is_scanning_startup = true;
                self.spawn_bg(|| {
                    BackgroundMessage::StartupItemsScanned(StartupService::scan_items())
                });
            }
            CleanupActionRequest::ToggleStartupItem(idx) => {
                let item = self
                    .cache_view_state
                    .startup_result
                    .as_ref()
                    .and_then(|r| r.items.get(idx))
                    .cloned();
                if let Some(item) = item {
                    self.spawn_bg(move || {
                        let res = StartupService::toggle_item_enabled(&item).map(|en| {
                            let st = if en { "Enabled" } else { "Disabled" };
                            format!("{} background service: {}", st, item.display_name)
                        });
                        BackgroundMessage::StartupItemToggled(res)
                    });
                }
            }
            CleanupActionRequest::TrashStartupItem(idx) => {
                let item = self
                    .cache_view_state
                    .startup_result
                    .as_ref()
                    .and_then(|r| r.items.get(idx))
                    .cloned();
                if let Some(item) = item {
                    self.set_confirm(ConfirmAction::TrashStartupItem {
                        label: item.display_name,
                        path: item.plist_path,
                    });
                }
            }
            CleanupActionRequest::RevealStartupItem(idx) => {
                if let Some(item) = self
                    .cache_view_state
                    .startup_result
                    .as_ref()
                    .and_then(|r| r.items.get(idx))
                {
                    let _ = StartupService::reveal_in_finder(&item.plist_path);
                }
            }
            CleanupActionRequest::ScanCaches => {
                self.cache_view_state.is_scanning_caches = true;
                self.spawn_bg(
                    || BackgroundMessage::CachesScanned(CacheService::scan_user_caches()),
                );
            }
            CleanupActionRequest::CleanSelectedCaches(paths) => {
                let total_bytes = self
                    .cache_view_state
                    .cache_scan_result
                    .as_ref()
                    .map_or(0, |s| s.selected_total_bytes());
                self.set_confirm(ConfirmAction::CleanCaches {
                    count: paths.len(),
                    total_bytes,
                    paths,
                });
            }
            CleanupActionRequest::PurgeRamCache => self.set_confirm(ConfirmAction::PurgeRam),
            CleanupActionRequest::FlushDns => match CacheService::flush_dns_cache() {
                Ok(()) => self.set_status("DNS cache flushed successfully".to_string(), false),
                Err(e) => self.set_status(format!("Failed to flush DNS cache: {}", e), true),
            },
            CleanupActionRequest::FlushQuickLook => match CacheService::flush_quicklook_cache() {
                Ok(()) => self.set_status(
                    "QuickLook thumbnail cache reset successfully".to_string(),
                    false,
                ),
                Err(e) => self.set_status(format!("Failed to reset QuickLook cache: {}", e), true),
            },
        }
    }
}
