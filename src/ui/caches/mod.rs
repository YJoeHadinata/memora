pub mod app_caches;
pub mod large_files;
pub mod large_files_filters;
pub mod maintenance;
pub mod startup;
pub mod startup_card;
pub mod storage_optimizer;
pub mod uninstaller;
pub mod uninstaller_details;

use std::path::PathBuf;
use std::time::Instant;

use egui::{vec2, Color32, CornerRadius, RichText, ScrollArea, Stroke, Ui};

use crate::models::{
    AgeFilter, AppLeftoverKind, AppUninstallScanResult, CacheScanResult, LargeFileCategory,
    LargeFilesScanResult, MemorySnapshot, SizeFilter, StartupItemKind, StartupScanResult,
    StorageJunkReport,
};
use crate::ui::theme::AppTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CleanupSubTab {
    #[default]
    Storage,
    LargeFiles,
    Uninstaller,
    Startup,
    Caches,
    Maintenance,
}

#[derive(Debug, Clone, Default)]
pub struct CleanupViewState {
    pub active_subtab: CleanupSubTab,
    pub storage_junk: Option<StorageJunkReport>,
    pub is_scanning_storage: bool,
    pub is_cleaning_storage: bool,
    pub large_files_result: Option<LargeFilesScanResult>,
    pub is_scanning_large_files: bool,
    pub is_cleaning_large_files: bool,
    pub large_files_size_filter: SizeFilter,
    pub large_files_age_filter: AgeFilter,
    pub large_files_category_filter: LargeFileCategory,
    pub large_files_search_query: String,
    pub uninstaller_result: Option<AppUninstallScanResult>,
    pub is_scanning_uninstaller: bool,
    pub is_uninstalling_app: bool,
    pub uninstaller_search_query: String,
    pub selected_app_index: Option<usize>,
    pub startup_result: Option<StartupScanResult>,
    pub is_scanning_startup: bool,
    pub startup_search_query: String,
    pub startup_filter_kind: Option<StartupItemKind>,
    pub startup_running_only: bool,
    pub cache_scan_result: Option<CacheScanResult>,
    pub is_scanning_caches: bool,
    pub is_cleaning_caches: bool,
    pub cache_search_query: String,
    pub is_purging_ram: bool,
    pub last_action_message: Option<(String, Instant, bool)>,
}

pub type CacheViewState = CleanupViewState;

#[derive(Debug, Clone)]
pub enum CleanupActionRequest {
    ScanStorageJunk,
    EmptyTrash,
    DeleteSelectedInstallers(Vec<PathBuf>),
    PurgeApfsSnapshots,
    PruneUnavailableSimulators,
    ScanLargeFiles(u64),
    RevealLargeFileInFinder(PathBuf),
    DeleteSelectedLargeFiles(Vec<PathBuf>),
    ScanInstalledApps,
    RevealAppPathInFinder(PathBuf),
    UninstallAppSelected {
        app_name: String,
        paths: Vec<PathBuf>,
    },
    ScanStartupItems,
    ToggleStartupItem(usize),
    TrashStartupItem(usize),
    RevealStartupItem(usize),
    ScanCaches,
    CleanSelectedCaches(Vec<PathBuf>),
    PurgeRamCache,
    FlushDns,
    FlushQuickLook,
}

pub type CacheActionRequest = CleanupActionRequest;

pub fn render_caches(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    state: &mut CleanupViewState,
    theme: &AppTheme,
) -> Option<CleanupActionRequest> {
    let mut requested_action = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_width = (ui.available_width() - 4.0).max(200.0);
            ui.set_max_width(avail_width);
            ui.add_space(8.0);

            ui.vertical(|ui| {
                ui.label(RichText::new("Cleanup & Storage").size(22.0).strong().color(theme.text_primary));
                let sub = "Reclaim disk storage, clear developer junk, and manage system caches with complete transparency.";
                ui.label(RichText::new(sub).size(12.0).color(theme.text_secondary));
            });
            ui.add_space(14.0);

            ui.horizontal(|ui| {
                let subtabs = [
                    (CleanupSubTab::Storage, "Storage Optimizer"),
                    (CleanupSubTab::LargeFiles, "Large & Old Files"),
                    (CleanupSubTab::Uninstaller, "App Uninstaller"),
                    (CleanupSubTab::Startup, "Startup Items"),
                    (CleanupSubTab::Caches, "Application Caches"),
                    (CleanupSubTab::Maintenance, "RAM & Diagnostics"),
                ];
                for (tab, label) in subtabs {
                    let btn = if state.active_subtab == tab {
                        theme.primary_button(RichText::new(label).size(12.0).strong())
                    } else {
                        theme.subtle_button(RichText::new(label).size(12.0))
                    };
                    if ui.add(btn).clicked() {
                        state.active_subtab = tab;
                    }
                }
            });
            ui.add_space(14.0);

            match state.active_subtab {
                CleanupSubTab::Storage => storage_optimizer::render_storage_optimizer(ui, state, theme, &mut requested_action),
                CleanupSubTab::LargeFiles => large_files::render_large_files_finder(ui, state, theme, &mut requested_action),
                CleanupSubTab::Uninstaller => uninstaller::render_app_uninstaller(ui, state, theme, &mut requested_action),
                CleanupSubTab::Startup => startup::render_startup_manager(ui, state, theme, &mut requested_action),
                CleanupSubTab::Caches => app_caches::render_application_caches(ui, state, theme, &mut requested_action),
                CleanupSubTab::Maintenance => maintenance::render_ram_and_maintenance(ui, snapshot, state, theme, &mut requested_action),
            }
            ui.add_space(20.0);
        });

    requested_action
}

pub(crate) fn render_badge_pill(ui: &mut Ui, text: &str, bg: Color32, fg: Color32) {
    let padding = vec2(6.0, 2.0);
    let font_id = egui::FontId::proportional(10.0);
    let galley = ui.painter().layout_no_wrap(text.to_string(), font_id, fg);
    let size = galley.size() + padding * 2.0;

    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(4),
        bg,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    ui.painter().galley(rect.min + padding, galley, fg);
}

pub(crate) fn render_leftover_kind_pill(ui: &mut Ui, kind: &AppLeftoverKind) {
    let (bg, text) = match kind {
        AppLeftoverKind::ApplicationBundle => ((30, 58, 138), (147, 197, 253)),
        AppLeftoverKind::ApplicationSupport => ((88, 28, 135), (216, 180, 254)),
        AppLeftoverKind::Caches => ((120, 53, 15), (253, 230, 138)),
        AppLeftoverKind::Preferences => ((19, 78, 74), (153, 246, 228)),
        AppLeftoverKind::SavedState => ((55, 48, 163), (199, 210, 254)),
        _ => ((51, 65, 85), (203, 213, 225)),
    };
    let (bg_c, tx_c) = (
        Color32::from_rgb(bg.0, bg.1, bg.2),
        Color32::from_rgb(text.0, text.1, text.2),
    );
    render_badge_pill(ui, kind.display_label(), bg_c, tx_c);
}
