pub mod actions;
pub mod layout;
pub mod messages;
pub mod modals;
pub mod shortcuts;
pub mod tabs;
pub mod worker;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use crate::config::{AppConfig, ThemePreference};
use crate::models::*;
use crate::services::*;
use crate::system::macos::*;
use crate::ui::components::ConfirmDialogState;
use crate::ui::{AppTheme, CleanupViewState, ProcessViewState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Overview,
    Processes,
    Cleanup,
    Settings,
}

pub(crate) enum BackgroundMessage {
    MetricsUpdated {
        snapshot: MemorySnapshot,
        storage: Box<StorageSnapshot>,
        thermal: Box<ThermalSnapshot>,
        processes: Vec<ProcessInfo>,
    },
    CollectionError(String),
    StorageJunkScanned(StorageJunkReport),
    TrashEmptied(Result<u64, String>),
    InstallersCleaned(Result<u64, String>),
    ApfsSnapshotsCleaned(Result<usize, String>),
    SimulatorsPruned(Result<String, String>),
    LargeFilesScanned(LargeFilesScanResult),
    LargeFilesDeleted(Result<u64, String>),
    AppsScanned(AppUninstallScanResult),
    AppUninstalled(Result<u64, String>),
    StartupItemsScanned(StartupScanResult),
    StartupItemToggled(Result<String, String>),
    StartupItemTrashed(Result<String, String>),
    CachesScanned(CacheScanResult),
    CacheCleaned(u64),
    RamPurged(Result<PurgeRamResult, String>),
}

pub struct MemoraApp {
    pub(crate) config: AppConfig,
    pub(crate) active_tab: AppTab,
    pub(crate) theme: AppTheme,
    #[allow(dead_code)]
    pub(crate) memory_service: Arc<MemoryService>,
    pub(crate) process_service: Arc<ProcessService>,
    pub(crate) cache_service: Arc<CacheService>,
    #[allow(dead_code)]
    pub(crate) storage_service: Arc<StorageService>,
    #[allow(dead_code)]
    pub(crate) thermal_service: Arc<ThermalService>,
    pub(crate) history_service: HistoryService,
    pub(crate) process_view_state: ProcessViewState,
    pub(crate) cache_view_state: CleanupViewState,
    pub(crate) latest_snapshot: Option<MemorySnapshot>,
    pub(crate) latest_storage: Option<StorageSnapshot>,
    pub(crate) latest_thermal: Option<ThermalSnapshot>,
    pub(crate) processes: Vec<ProcessInfo>,
    pub(crate) status_message: Option<(String, Instant, bool)>,
    pub(crate) sender: Sender<BackgroundMessage>,
    pub(crate) receiver: Receiver<BackgroundMessage>,
    pub(crate) stop_signal: Arc<AtomicBool>,
    pub(crate) refresh_interval_atomic: Arc<AtomicU64>,
    pub(crate) is_history_paused: bool,
    pub(crate) confirm_dialog: Option<ConfirmDialogState>,
    pub(crate) inspector_process: Option<ProcessInfo>,
    pub(crate) last_alerted_pids: HashMap<i32, Instant>,
    pub(crate) last_pressure_alert: Option<(MemoryPressure, Instant)>,
    pub(crate) is_window_visible: Arc<AtomicBool>,
}

impl MemoraApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let proc_col = Arc::new(MacosProcessCollector::new());
        let memory_service = Arc::new(MemoryService::new(Arc::new(MacosMemoryCollector::new())));
        let process_service = Arc::new(ProcessService::new(proc_col.clone(), proc_col));
        let storage_service = Arc::new(StorageService::new(Arc::new(MacosStorageCollector::new())));
        let thermal_service = Arc::new(ThermalService::new(Arc::new(MacosThermalCollector::new())));

        let config = AppConfig::load();
        let history_service = HistoryService::new(config.history_capacity);
        let cache_service = Arc::new(CacheService::new(memory_service.clone()));

        #[cfg(target_os = "macos")]
        menu_bar::sync_menu_bar_config(config.show_menu_bar, config.menu_bar_display_mode);

        let (tx, rx) = channel();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let refresh_interval_atomic = Arc::new(AtomicU64::new(config.refresh_interval_secs.max(1)));
        let is_window_visible = Arc::new(AtomicBool::new(true));

        worker::spawn_background_worker(
            memory_service.clone(),
            process_service.clone(),
            storage_service.clone(),
            thermal_service.clone(),
            stop_signal.clone(),
            refresh_interval_atomic.clone(),
            tx.clone(),
            is_window_visible.clone(),
            cc.egui_ctx.clone(),
        );

        Self {
            config,
            active_tab: AppTab::Overview,
            theme: AppTheme::dark(),
            memory_service,
            process_service,
            cache_service,
            storage_service,
            thermal_service,
            history_service,
            process_view_state: ProcessViewState::default(),
            cache_view_state: CleanupViewState::default(),
            latest_snapshot: None,
            latest_storage: None,
            latest_thermal: None,
            processes: Vec::new(),
            status_message: None,
            sender: tx,
            receiver: rx,
            stop_signal,
            refresh_interval_atomic,
            is_history_paused: false,
            confirm_dialog: None,
            inspector_process: None,
            last_alerted_pids: HashMap::new(),
            last_pressure_alert: None,
            is_window_visible,
        }
    }

    pub(crate) fn update_theme(&mut self, is_system_dark: bool) {
        let is_dark = match self.config.theme {
            ThemePreference::System => is_system_dark,
            ThemePreference::Dark => true,
            ThemePreference::Light => false,
        };
        self.theme = if is_dark {
            AppTheme::dark()
        } else {
            AppTheme::light()
        };
    }
}

impl Drop for MemoraApp {
    fn drop(&mut self) {
        self.stop_signal.store(true, Ordering::Relaxed);
    }
}
