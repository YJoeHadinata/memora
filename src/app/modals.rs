use std::thread;

use crate::app::{BackgroundMessage, MemoraApp};
use crate::services::{
    AppUninstallService, CacheService, LargeFilesService, StartupService, StorageCleanService,
};
use crate::ui::components::{
    render_confirm_dialog, render_process_inspector, ConfirmAction, ConfirmDialogState,
    DialogResult, InspectorAction,
};
use crate::ui::AppTheme;

impl MemoraApp {
    pub(crate) fn render_modals(&mut self, ui: &mut egui::Ui, theme: &AppTheme) {
        if let Some(proc) = self.inspector_process.clone() {
            let ctx = ui.ctx().clone();
            if let Some(inspector_action) = render_process_inspector(&ctx, &proc, theme) {
                match inspector_action {
                    InspectorAction::Quit(p, action) => {
                        self.inspector_process = None;
                        self.confirm_dialog = Some(ConfirmDialogState {
                            process: Some(p),
                            action,
                        });
                    }
                    InspectorAction::RevealInFinder(path) => {
                        let _ = std::process::Command::new("open")
                            .args(["-R", &path])
                            .spawn();
                    }
                    InspectorAction::Close => self.inspector_process = None,
                }
            }
        }

        if let Some(dialog_state) = self.confirm_dialog.clone() {
            let ctx = ui.ctx().clone();
            if let Some(result) = render_confirm_dialog(&ctx, &dialog_state, theme) {
                if let DialogResult::Confirmed(action) = result {
                    self.execute_confirmed_action(action, &dialog_state);
                }
                self.confirm_dialog = None;
            }
        }
    }

    fn execute_confirmed_action(
        &mut self,
        action: ConfirmAction,
        dialog_state: &ConfirmDialogState,
    ) {
        match action {
            ConfirmAction::SafeQuit => {
                if let Some(ref p) = dialog_state.process {
                    match self.process_service.terminate(p.pid) {
                        Ok(()) => self.set_status(
                            format!("Termination request sent to {} (PID {})", p.name, p.pid),
                            false,
                        ),
                        Err(e) => {
                            self.set_status(format!("Failed to quit {}: {}", p.name, e), true)
                        }
                    }
                }
            }
            ConfirmAction::ForceQuit => {
                if let Some(ref p) = dialog_state.process {
                    match self.process_service.force_kill(p.pid) {
                        Ok(()) => {
                            self.set_status(format!("Force quit {} (PID {})", p.name, p.pid), false)
                        }
                        Err(e) => {
                            self.set_status(format!("Failed to force quit {}: {}", p.name, e), true)
                        }
                    }
                }
            }
            ConfirmAction::EmptyTrash { .. } => {
                self.cache_view_state.is_cleaning_storage = true;
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let _ = tx.send(BackgroundMessage::TrashEmptied(
                        StorageCleanService::empty_trash(),
                    ));
                });
            }
            ConfirmAction::DeleteInstallers { paths, .. } => {
                self.cache_view_state.is_cleaning_storage = true;
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let _ = tx.send(BackgroundMessage::InstallersCleaned(
                        StorageCleanService::clean_installers(&paths),
                    ));
                });
            }
            ConfirmAction::DeleteLargeFiles { paths, .. } => {
                self.cache_view_state.is_cleaning_large_files = true;
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let _ = tx.send(BackgroundMessage::LargeFilesDeleted(
                        LargeFilesService::move_multiple_to_trash(&paths),
                    ));
                });
            }
            ConfirmAction::CleanCaches { paths, .. } => {
                self.cache_view_state.is_cleaning_caches = true;
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let freed = CacheService::clean_cache_items(&paths).unwrap_or(0);
                    let _ = tx.send(BackgroundMessage::CacheCleaned(freed));
                });
            }
            ConfirmAction::UninstallApp { paths, .. } => {
                self.cache_view_state.is_uninstalling_app = true;
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let _ = tx.send(BackgroundMessage::AppUninstalled(
                        AppUninstallService::move_paths_to_trash(&paths),
                    ));
                });
            }
            ConfirmAction::TrashStartupItem { label, path } => {
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let item = crate::models::StartupItem {
                        id: label.clone(),
                        label: label.clone(),
                        display_name: label.clone(),
                        program_path: None,
                        plist_path: path,
                        kind: crate::models::StartupItemKind::UserAgent,
                        status: crate::models::StartupItemStatus::Disabled,
                        run_at_load: false,
                        keep_alive: false,
                        is_disabled_file: false,
                        file_size_bytes: 0,
                    };
                    let res = StartupService::move_item_to_trash(&item)
                        .map(|()| format!("Moved \"{}\" service plist to Trash", label));
                    let _ = tx.send(BackgroundMessage::StartupItemTrashed(res));
                });
            }
            ConfirmAction::PurgeRam => {
                self.cache_view_state.is_purging_ram = true;
                let cache_svc = self.cache_service.clone();
                let tx = self.sender.clone();
                thread::spawn(move || {
                    let _ = tx.send(BackgroundMessage::RamPurged(
                        cache_svc.purge_system_ram_cache(),
                    ));
                });
            }
        }
    }
}
