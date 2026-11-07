pub mod actions;
pub mod layout;
pub mod messages;
pub mod modals;
pub mod shortcuts;
pub mod tabs;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

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
    pub(crate) history_service: HistoryService,
    pub(crate) process_view_state: ProcessViewState,
    pub(crate) cache_view_state: CleanupViewState,
    pub(crate) latest_snapshot: Option<MemorySnapshot>,
    pub(crate) latest_storage: Option<StorageSnapshot>,
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
    #[cfg(target_os = "macos")]
    pub(crate) menu_bar: Option<MacosMenuBar>,
    pub(crate) is_window_visible: Arc<AtomicBool>,
}

impl MemoraApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let proc_col = Arc::new(MacosProcessCollector::new());
        let memory_service = Arc::new(MemoryService::new(Arc::new(MacosMemoryCollector::new())));
        let process_service = Arc::new(ProcessService::new(proc_col.clone(), proc_col));
        let storage_service = Arc::new(StorageService::new(Arc::new(MacosStorageCollector::new())));

        let config = AppConfig::load();
        let history_service = HistoryService::new(config.history_capacity);
        let cache_service = Arc::new(CacheService::new(memory_service.clone()));

        let (tx, rx) = channel();
        let stop_signal = Arc::new(AtomicBool::new(false));
        let refresh_interval_atomic = Arc::new(AtomicU64::new(config.refresh_interval_secs.max(1)));
        let is_window_visible = Arc::new(AtomicBool::new(true));

        let (mem_s, proc_s, stor_s) = (
            memory_service.clone(),
            process_service.clone(),
            storage_service.clone(),
        );
        let (stop_c, int_c, tx_c, vis_c) = (
            stop_signal.clone(),
            refresh_interval_atomic.clone(),
            tx.clone(),
            is_window_visible.clone(),
        );

        thread::spawn(move || {
            while !stop_c.load(Ordering::Relaxed) {
                let start = Instant::now();
                let processes = if vis_c.load(Ordering::Relaxed) {
                    proc_s.list_processes().unwrap_or_default()
                } else {
                    Vec::new()
                };
                match (mem_s.snapshot(), stor_s.snapshot()) {
                    (Ok(snapshot), Ok(storage)) => {
                        let _ = tx_c.send(BackgroundMessage::MetricsUpdated {
                            snapshot,
                            storage: Box::new(storage),
                            processes,
                        });
                    }
                    (Err(e), _) => {
                        let _ = tx_c.send(BackgroundMessage::CollectionError(e.to_string()));
                    }
                    (_, Err(e)) => {
                        let _ = tx_c.send(BackgroundMessage::CollectionError(e));
                    }
                }
                let secs = int_c.load(Ordering::Relaxed).max(1);
                let sleep_dur = Duration::from_secs(secs).saturating_sub(start.elapsed());
                if !sleep_dur.is_zero() {
                    thread::sleep(sleep_dur);
                }
            }
        });

        #[cfg(target_os = "macos")]
        let menu_bar = config.show_menu_bar.then(MacosMenuBar::new).flatten();

        Self {
            config,
            active_tab: AppTab::Overview,
            theme: AppTheme::dark(),
            memory_service,
            process_service,
            cache_service,
            storage_service,
            history_service,
            process_view_state: ProcessViewState::default(),
            cache_view_state: CleanupViewState::default(),
            latest_snapshot: None,
            latest_storage: None,
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
            #[cfg(target_os = "macos")]
            menu_bar,
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
