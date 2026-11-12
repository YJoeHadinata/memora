use crate::app::{BackgroundMessage, MemoraApp};
use crate::services::{AppUninstallService, CacheService, LargeFilesService, StartupService};
use crate::utils::format::format_bytes;

impl MemoraApp {
    pub(crate) fn check_background_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                BackgroundMessage::MetricsUpdated {
                    snapshot,
                    storage,
                    thermal,
                    processes,
                } => {
                    if !self.is_history_paused {
                        self.history_service.record(&snapshot);
                    }
                    self.check_alerts(&snapshot, &processes);
                    self.latest_snapshot = Some(snapshot);
                    self.latest_storage = Some(*storage);
                    self.latest_thermal = Some(*thermal);
                    if !processes.is_empty() {
                        self.processes = processes;
                    }
                }
                BackgroundMessage::CollectionError(err) => self.set_status(err, true),
                BackgroundMessage::StorageJunkScanned(report) => {
                    self.cache_view_state.is_scanning_storage = false;
                    let total = format_bytes(report.total_reclaimable_bytes());
                    self.cache_view_state.storage_junk = Some(report);
                    self.set_status(
                        format!("Storage scan complete: {} readily reclaimable", total),
                        false,
                    );
                }
                BackgroundMessage::TrashEmptied(res) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    self.handle_storage_res(
                        res,
                        "Trash emptied: reclaimed",
                        "Failed to empty trash",
                    );
                }
                BackgroundMessage::InstallersCleaned(res) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    self.handle_storage_res(
                        res,
                        "Deleted installer files: reclaimed",
                        "Failed to delete installers",
                    );
                }
                BackgroundMessage::ApfsSnapshotsCleaned(res) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    match res {
                        Ok(c) => {
                            let m = format!("Purged {} APFS local snapshot(s) successfully", c);
                            self.set_status(m, false);
                            self.spawn_storage_junk_rescan();
                        }
                        Err(e) => {
                            self.set_status(format!("Failed to purge APFS snapshots: {}", e), true)
                        }
                    }
                }
                BackgroundMessage::SimulatorsPruned(res) => match res {
                    Ok(m) => self.set_status(m, false),
                    Err(e) => self.set_status(format!("Simulator cleanup: {}", e), true),
                },
                BackgroundMessage::LargeFilesScanned(r) => {
                    self.cache_view_state.is_scanning_large_files = false;
                    let m = format!(
                        "Large files scan complete: {} items found ({})",
                        r.files.len(),
                        format_bytes(r.total_bytes())
                    );
                    self.cache_view_state.large_files_result = Some(r);
                    self.set_status(m, false);
                }
                BackgroundMessage::LargeFilesDeleted(res) => {
                    self.cache_view_state.is_cleaning_large_files = false;
                    match res {
                        Ok(freed) => {
                            let m =
                                format!("Moved items to Trash: {} reclaimed", format_bytes(freed));
                            self.set_status(m, false);
                            self.cache_view_state.is_scanning_large_files = true;
                            let min_b = self.cache_view_state.large_files_size_filter.min_bytes();
                            self.spawn_bg(move || {
                                BackgroundMessage::LargeFilesScanned(
                                    LargeFilesService::scan_large_files(min_b, None),
                                )
                            });
                            self.spawn_storage_junk_rescan();
                        }
                        Err(e) => {
                            self.set_status(format!("Failed to move files to Trash: {}", e), true)
                        }
                    }
                }
                BackgroundMessage::AppsScanned(r) => {
                    self.cache_view_state.is_scanning_uninstaller = false;
                    let m = format!(
                        "Discovered {} installed applications ({})",
                        r.total_apps(),
                        format_bytes(r.total_reclaimable_bytes())
                    );
                    if self.cache_view_state.selected_app_index.is_none() && !r.apps.is_empty() {
                        self.cache_view_state.selected_app_index = Some(0);
                    }
                    self.cache_view_state.uninstaller_result = Some(r);
                    self.set_status(m, false);
                }
                BackgroundMessage::AppUninstalled(res) => {
                    self.cache_view_state.is_uninstalling_app = false;
                    match res {
                        Ok(freed) => {
                            let m = format!(
                                "Application uninstalled: reclaimed {}",
                                format_bytes(freed)
                            );
                            self.set_status(m, false);
                            self.cache_view_state.is_scanning_uninstaller = true;
                            self.spawn_bg(|| {
                                BackgroundMessage::AppsScanned(
                                    AppUninstallService::scan_installed_apps(),
                                )
                            });
                            self.spawn_storage_junk_rescan();
                        }
                        Err(e) => {
                            self.set_status(format!("Failed to uninstall application: {}", e), true)
                        }
                    }
                }
                BackgroundMessage::StartupItemsScanned(r) => {
                    self.cache_view_state.is_scanning_startup = false;
                    let m = format!(
                        "Startup services scan complete: {} items discovered ({} running)",
                        r.items.len(),
                        r.running_count()
                    );
                    self.cache_view_state.startup_result = Some(r);
                    self.set_status(m, false);
                }
                BackgroundMessage::StartupItemToggled(res)
                | BackgroundMessage::StartupItemTrashed(res) => match res {
                    Ok(m) => {
                        self.set_status(m, false);
                        self.cache_view_state.is_scanning_startup = true;
                        self.spawn_bg(|| {
                            BackgroundMessage::StartupItemsScanned(StartupService::scan_items())
                        });
                    }
                    Err(e) => self.set_status(format!("Service action failed: {}", e), true),
                },
                BackgroundMessage::CachesScanned(r) => {
                    self.cache_view_state.is_scanning_caches = false;
                    let m = format!(
                        "Cache scan complete: {} application caches found ({})",
                        r.items.len(),
                        format_bytes(r.total_bytes)
                    );
                    self.cache_view_state.cache_scan_result = Some(r);
                    self.set_status(m, false);
                }
                BackgroundMessage::CacheCleaned(freed) => {
                    self.cache_view_state.is_cleaning_caches = false;
                    let m = format!(
                        "Cleaned application caches: reclaimed {}",
                        format_bytes(freed)
                    );
                    self.set_status(m, false);
                    self.cache_view_state.is_scanning_caches = true;
                    self.spawn_bg(|| {
                        BackgroundMessage::CachesScanned(CacheService::scan_user_caches())
                    });
                }
                BackgroundMessage::RamPurged(res) => {
                    self.cache_view_state.is_purging_ram = false;
                    match res {
                        Ok(r) => self.set_status(r.message, false),
                        Err(e) => self.set_status(e, true),
                    }
                }
            }
        }
    }
}
