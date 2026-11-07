use std::sync::atomic::Ordering;
use std::time::Duration;

use egui::{RichText, Stroke};

use crate::app::{AppTab, BackgroundMessage, MemoraApp};
use crate::services::{CacheService, ProcessService, StartupService, StorageCleanService};
use crate::system::macos::MacosMenuBar;
use crate::ui::components::ConfirmDialogState;
use crate::ui::{
    render_caches, render_dashboard, render_processes, render_settings, AppTheme, DashboardAction,
    ProcessActionRequest,
};

impl MemoraApp {
    pub(crate) fn render_bottom_panel(&mut self, ui: &mut egui::Ui, theme: &AppTheme) {
        let frame = egui::Frame::new()
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(egui::Margin::symmetric(16, 6));

        egui::Panel::bottom("bottom_status_panel")
            .frame(frame)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((ref msg, time, is_error)) = self.status_message {
                        if time.elapsed() < Duration::from_secs(6) {
                            let c = if is_error {
                                theme.critical_color
                            } else {
                                theme.normal_color
                            };
                            ui.label(RichText::new(msg).size(11.0).strong().color(c));
                        } else {
                            self.status_message = None;
                        }
                    } else {
                        let def = "Memora • Native macOS System Monitor • ⌘E Export Report";
                        ui.label(RichText::new(def).size(11.0).color(theme.text_secondary));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let info = format!(
                            "{} processes • Sampling every {}s",
                            self.processes.len(),
                            self.config.refresh_interval_secs
                        );
                        ui.label(RichText::new(info).size(11.0).color(theme.text_secondary));
                    });
                });
            });
    }

    pub(crate) fn render_central_panel(&mut self, ui: &mut egui::Ui, theme: &AppTheme) {
        let frame = egui::Frame::new()
            .fill(theme.bg_fill)
            .inner_margin(egui::Margin::symmetric(16, 12));

        egui::CentralPanel::default()
            .frame(frame)
            .show(ui, |ui| match self.active_tab {
                AppTab::Overview => {
                    if let Some(action) = render_dashboard(
                        ui,
                        self.latest_snapshot.as_ref(),
                        self.latest_storage.as_ref(),
                        &self.history_service,
                        &self.processes,
                        &mut self.is_history_paused,
                        theme,
                    ) {
                        match action {
                            DashboardAction::ExportReport => self.export_report(),
                            DashboardAction::InspectProcess(p) => self.inspector_process = Some(p),
                            DashboardAction::NavigateToProcesses => {
                                self.active_tab = AppTab::Processes
                            }
                            DashboardAction::NavigateToCaches => self.active_tab = AppTab::Cleanup,
                        }
                    }
                }
                AppTab::Processes => {
                    let filtered = ProcessService::filter_and_sort(
                        &self.processes,
                        &self.process_view_state.search_query,
                        self.process_view_state.filter_category,
                        self.process_view_state.sort_column,
                        self.process_view_state.sort_direction,
                    );
                    if let Some(req) =
                        render_processes(ui, &filtered, &mut self.process_view_state, theme)
                    {
                        match req {
                            ProcessActionRequest::RequestQuit(proc, action) => {
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: Some(proc),
                                    action,
                                });
                            }
                            ProcessActionRequest::Inspect(p) => self.inspector_process = Some(p),
                        }
                    }
                }
                AppTab::Cleanup => {
                    self.lazy_load_cleanup_data();
                    if let Some(req) = render_caches(
                        ui,
                        self.latest_snapshot.as_ref(),
                        &mut self.cache_view_state,
                        theme,
                    ) {
                        self.handle_cleanup_action(req);
                    }
                }
                AppTab::Settings => {
                    let prev = self.config.clone();
                    render_settings(ui, &mut self.config, theme);
                    self.history_service
                        .buffer_mut()
                        .set_capacity(self.config.history_capacity);
                    self.refresh_interval_atomic
                        .store(self.config.refresh_interval_secs.max(1), Ordering::Relaxed);
                    #[cfg(target_os = "macos")]
                    if self.config.show_menu_bar != prev.show_menu_bar
                        || self.config.menu_bar_display_mode != prev.menu_bar_display_mode
                    {
                        if self.config.show_menu_bar {
                            if self.menu_bar.is_none() {
                                self.menu_bar = MacosMenuBar::new();
                            }
                            if let (Some(ref mut bar), Some(ref snap)) =
                                (&mut self.menu_bar, &self.latest_snapshot)
                            {
                                bar.update(snap, self.config.menu_bar_display_mode);
                            }
                        } else {
                            self.menu_bar = None;
                        }
                    }
                    if self.config != prev {
                        let _ = self.config.save();
                    }
                }
            });
    }

    fn lazy_load_cleanup_data(&mut self) {
        if self.cache_view_state.cache_scan_result.is_none()
            && !self.cache_view_state.is_scanning_caches
        {
            self.cache_view_state.is_scanning_caches = true;
            self.spawn_bg(|| BackgroundMessage::CachesScanned(CacheService::scan_user_caches()));
        }
        if self.cache_view_state.storage_junk.is_none()
            && !self.cache_view_state.is_scanning_storage
        {
            self.cache_view_state.is_scanning_storage = true;
            self.spawn_bg(|| {
                BackgroundMessage::StorageJunkScanned(StorageCleanService::scan_all_junk())
            });
        }
        if self.cache_view_state.active_subtab == crate::ui::CleanupSubTab::Startup
            && self.cache_view_state.startup_result.is_none()
            && !self.cache_view_state.is_scanning_startup
        {
            self.cache_view_state.is_scanning_startup = true;
            self.spawn_bg(|| BackgroundMessage::StartupItemsScanned(StartupService::scan_items()));
        }
    }
}
