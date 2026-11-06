use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use egui::{vec2, Color32, CornerRadius, RichText, Stroke};

use crate::config::{AppConfig, ThemePreference};
use crate::models::{
    CacheScanResult, LargeFilesScanResult, MemoryPressure, MemorySnapshot, ProcessInfo,
    PurgeRamResult, StorageJunkReport, StorageSnapshot,
};
use crate::services::{
    CacheService, HistoryService, LargeFilesService, MemoryService, ProcessService,
    StorageCleanService, StorageService,
};
use crate::system::macos::{
    send_notification, MacosMemoryCollector, MacosMenuBar, MacosProcessCollector,
    MacosStorageCollector,
};
use crate::ui::components::{
    render_confirm_dialog, render_pressure_badge, render_process_inspector, ConfirmAction,
    ConfirmDialogState, DialogResult, InspectorAction,
};
use crate::ui::{
    paint_theme_icon, render_app_logo, render_caches, render_dashboard, render_processes,
    render_settings, AppTheme, CleanupActionRequest, CleanupViewState, DashboardAction,
    ProcessActionRequest, ProcessViewState,
};
use crate::utils::format::format_bytes;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppTab {
    Overview,
    Processes,
    Cleanup,
    Settings,
}

enum BackgroundMessage {
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
    CachesScanned(CacheScanResult),
    CacheCleaned(u64),
    RamPurged(Result<PurgeRamResult, String>),
}

pub struct MemoraApp {
    config: AppConfig,
    active_tab: AppTab,
    theme: AppTheme,
    #[allow(dead_code)]
    memory_service: Arc<MemoryService>,
    process_service: Arc<ProcessService>,
    cache_service: Arc<CacheService>,
    #[allow(dead_code)]
    storage_service: Arc<StorageService>,
    history_service: HistoryService,
    process_view_state: ProcessViewState,
    cache_view_state: CleanupViewState,

    // Live collected data
    latest_snapshot: Option<MemorySnapshot>,
    latest_storage: Option<StorageSnapshot>,
    processes: Vec<ProcessInfo>,
    status_message: Option<(String, Instant, bool)>, // (message, timestamp, is_error)

    // Background thread communication
    sender: std::sync::mpsc::Sender<BackgroundMessage>,
    receiver: Receiver<BackgroundMessage>,
    stop_signal: Arc<std::sync::atomic::AtomicBool>,
    refresh_interval_atomic: Arc<std::sync::atomic::AtomicU64>,

    // Controls
    is_history_paused: bool,

    // Confirmation & Inspector modals
    confirm_dialog: Option<ConfirmDialogState>,
    inspector_process: Option<ProcessInfo>,

    // Smart Alert State
    last_alerted_pids: std::collections::HashMap<i32, Instant>,
    last_pressure_alert: Option<(MemoryPressure, Instant)>,

    // macOS native Menu Bar companion
    #[cfg(target_os = "macos")]
    menu_bar: Option<MacosMenuBar>,

    // Tracks whether the window is currently displayed or running in background
    is_window_visible: Arc<std::sync::atomic::AtomicBool>,
}

impl MemoraApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let memory_collector = Arc::new(MacosMemoryCollector::new());
        let process_collector = Arc::new(MacosProcessCollector::new());
        let storage_collector = Arc::new(MacosStorageCollector::new());

        let memory_service = Arc::new(MemoryService::new(memory_collector));
        let process_service = Arc::new(ProcessService::new(
            process_collector.clone(),
            process_collector,
        ));
        let storage_service = Arc::new(StorageService::new(storage_collector));

        let config = AppConfig::load();
        let history_service = HistoryService::new(config.history_capacity);
        let cache_service = Arc::new(CacheService::new(memory_service.clone()));

        let (tx, rx) = channel();
        let stop_signal = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let refresh_interval_atomic = Arc::new(std::sync::atomic::AtomicU64::new(
            config.refresh_interval_secs.max(1),
        ));
        let is_window_visible = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Spawn non-blocking background collector thread
        let mem_svc = memory_service.clone();
        let proc_svc = process_service.clone();
        let stor_svc = storage_service.clone();
        let stop_clone = stop_signal.clone();
        let interval_clone = refresh_interval_atomic.clone();
        let tx_collector = tx.clone();
        let win_vis = is_window_visible.clone();

        thread::spawn(move || {
            while !stop_clone.load(std::sync::atomic::Ordering::Relaxed) {
                let start = Instant::now();

                let is_vis = win_vis.load(std::sync::atomic::Ordering::Relaxed);
                let processes = if is_vis {
                    proc_svc.list_processes().unwrap_or_default()
                } else {
                    Vec::new() // Zero heap overhead while running in background
                };

                match (mem_svc.snapshot(), stor_svc.snapshot()) {
                    (Ok(snapshot), Ok(storage)) => {
                        let _ = tx_collector.send(BackgroundMessage::MetricsUpdated {
                            snapshot,
                            storage: Box::new(storage),
                            processes,
                        });
                    }
                    (Err(e), _) => {
                        let _ =
                            tx_collector.send(BackgroundMessage::CollectionError(e.to_string()));
                    }
                    (_, Err(e)) => {
                        let _ =
                            tx_collector.send(BackgroundMessage::CollectionError(e.to_string()));
                    }
                }

                let interval_secs = interval_clone
                    .load(std::sync::atomic::Ordering::Relaxed)
                    .max(1);
                let elapsed = start.elapsed();
                let sleep_duration = Duration::from_secs(interval_secs).saturating_sub(elapsed);
                if !sleep_duration.is_zero() {
                    thread::sleep(sleep_duration);
                }
            }
        });

        #[cfg(target_os = "macos")]
        let menu_bar = if config.show_menu_bar {
            MacosMenuBar::new()
        } else {
            None
        };

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
            last_alerted_pids: std::collections::HashMap::new(),
            last_pressure_alert: None,
            #[cfg(target_os = "macos")]
            menu_bar,
            is_window_visible,
        }
    }

    fn update_theme(&mut self, is_system_dark: bool) {
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

    fn check_background_messages(&mut self) {
        while let Ok(msg) = self.receiver.try_recv() {
            match msg {
                BackgroundMessage::MetricsUpdated {
                    snapshot,
                    storage,
                    processes,
                } => {
                    if !self.is_history_paused {
                        self.history_service.record(&snapshot);
                    }
                    #[cfg(target_os = "macos")]
                    if let Some(ref mut bar) = self.menu_bar {
                        bar.update(&snapshot, self.config.menu_bar_display_mode);
                    }
                    self.check_alerts(&snapshot, &processes);
                    self.latest_snapshot = Some(snapshot);
                    self.latest_storage = Some(*storage);
                    self.processes = processes;
                }
                BackgroundMessage::CollectionError(err) => {
                    self.status_message = Some((err, Instant::now(), true));
                }
                BackgroundMessage::StorageJunkScanned(report) => {
                    self.cache_view_state.is_scanning_storage = false;
                    let total = format_bytes(report.total_reclaimable_bytes());
                    self.cache_view_state.storage_junk = Some(report);
                    self.status_message = Some((
                        format!("Storage scan complete: {} readily reclaimable", total),
                        Instant::now(),
                        false,
                    ));
                }
                BackgroundMessage::TrashEmptied(result) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    match result {
                        Ok(freed) => {
                            self.status_message = Some((
                                format!("Trash emptied: reclaimed {}", format_bytes(freed)),
                                Instant::now(),
                                false,
                            ));
                            // Trigger rescan of storage junk
                            let tx = self.sender.clone();
                            thread::spawn(move || {
                                let junk = StorageCleanService::scan_all_junk();
                                let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                            });
                        }
                        Err(err) => {
                            self.status_message = Some((
                                format!("Failed to empty trash: {}", err),
                                Instant::now(),
                                true,
                            ));
                        }
                    }
                }
                BackgroundMessage::InstallersCleaned(result) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    match result {
                        Ok(freed) => {
                            self.status_message = Some((
                                format!(
                                    "Deleted installer files: reclaimed {}",
                                    format_bytes(freed)
                                ),
                                Instant::now(),
                                false,
                            ));
                            // Trigger rescan of storage junk
                            let tx = self.sender.clone();
                            thread::spawn(move || {
                                let junk = StorageCleanService::scan_all_junk();
                                let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                            });
                        }
                        Err(err) => {
                            self.status_message = Some((
                                format!("Failed to delete installers: {}", err),
                                Instant::now(),
                                true,
                            ));
                        }
                    }
                }
                BackgroundMessage::ApfsSnapshotsCleaned(result) => {
                    self.cache_view_state.is_cleaning_storage = false;
                    match result {
                        Ok(count) => {
                            self.status_message = Some((
                                format!("Purged {} APFS local snapshot(s) successfully", count),
                                Instant::now(),
                                false,
                            ));
                            // Trigger rescan of storage junk
                            let tx = self.sender.clone();
                            thread::spawn(move || {
                                let junk = StorageCleanService::scan_all_junk();
                                let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                            });
                        }
                        Err(err) => {
                            self.status_message = Some((
                                format!("Failed to purge APFS snapshots: {}", err),
                                Instant::now(),
                                true,
                            ));
                        }
                    }
                }
                BackgroundMessage::SimulatorsPruned(result) => match result {
                    Ok(msg) => {
                        self.status_message = Some((msg, Instant::now(), false));
                    }
                    Err(err) => {
                        self.status_message =
                            Some((format!("Simulator cleanup: {}", err), Instant::now(), true));
                    }
                },
                BackgroundMessage::LargeFilesScanned(result) => {
                    self.cache_view_state.is_scanning_large_files = false;
                    let count = result.files.len();
                    let total = format_bytes(result.total_bytes());
                    self.cache_view_state.large_files_result = Some(result);
                    self.status_message = Some((
                        format!(
                            "Large files scan complete: {} items found ({})",
                            count, total
                        ),
                        Instant::now(),
                        false,
                    ));
                }
                BackgroundMessage::LargeFilesDeleted(result) => {
                    self.cache_view_state.is_cleaning_large_files = false;
                    match result {
                        Ok(freed) => {
                            self.status_message = Some((
                                format!("Moved items to Trash: {} reclaimed", format_bytes(freed)),
                                Instant::now(),
                                false,
                            ));
                            // Trigger background rescan of large files and storage junk
                            self.cache_view_state.is_scanning_large_files = true;
                            let min_bytes =
                                self.cache_view_state.large_files_size_filter.min_bytes();
                            let tx = self.sender.clone();
                            thread::spawn(move || {
                                let res = LargeFilesService::scan_large_files(min_bytes, None);
                                let _ = tx.send(BackgroundMessage::LargeFilesScanned(res));
                                let junk = StorageCleanService::scan_all_junk();
                                let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                            });
                        }
                        Err(err) => {
                            self.status_message = Some((
                                format!("Failed to move files to Trash: {}", err),
                                Instant::now(),
                                true,
                            ));
                        }
                    }
                }
                BackgroundMessage::CachesScanned(result) => {
                    self.cache_view_state.is_scanning_caches = false;
                    let count = result.items.len();
                    let total = format_bytes(result.total_bytes);
                    self.cache_view_state.cache_scan_result = Some(result);
                    self.status_message = Some((
                        format!(
                            "Cache scan complete: {} application caches found ({})",
                            count, total
                        ),
                        Instant::now(),
                        false,
                    ));
                }
                BackgroundMessage::CacheCleaned(freed_bytes) => {
                    self.cache_view_state.is_cleaning_caches = false;
                    self.status_message = Some((
                        format!(
                            "Cleaned application caches: reclaimed {}",
                            format_bytes(freed_bytes)
                        ),
                        Instant::now(),
                        false,
                    ));
                    // Re-scan user caches
                    self.cache_view_state.is_scanning_caches = true;
                    let tx = self.sender.clone();
                    thread::spawn(move || {
                        let res = CacheService::scan_user_caches();
                        let _ = tx.send(BackgroundMessage::CachesScanned(res));
                    });
                }
                BackgroundMessage::RamPurged(result) => {
                    self.cache_view_state.is_purging_ram = false;
                    match result {
                        Ok(res) => {
                            self.status_message = Some((res.message, Instant::now(), false));
                        }
                        Err(err) => {
                            self.status_message = Some((err, Instant::now(), true));
                        }
                    }
                }
            }
        }
    }

    fn check_alerts(&mut self, snapshot: &MemorySnapshot, processes: &[ProcessInfo]) {
        let now = Instant::now();
        let cooldown = Duration::from_secs(300); // 5-minute cooldown per alert

        // 1. Overall macOS Memory Pressure Warning/Critical Alert
        if self.config.notify_on_pressure_warning
            && (snapshot.pressure == MemoryPressure::Warning
                || snapshot.pressure == MemoryPressure::Critical)
        {
            let should_alert = match self.last_pressure_alert {
                Some((prev_pressure, prev_time)) => {
                    snapshot.pressure != prev_pressure || now.duration_since(prev_time) > cooldown
                }
                None => true,
            };

            if should_alert {
                self.last_pressure_alert = Some((snapshot.pressure, now));
                let pressure_name = match snapshot.pressure {
                    MemoryPressure::Critical => "Critical",
                    _ => "Elevated",
                };
                let body = format!(
                    "macOS memory pressure is {}. Compressor is actively compacting RAM.",
                    pressure_name
                );
                send_notification("Memora System Warning", &body);
            }
        }

        // 2. High RAM Process Spikes
        if self.config.notify_on_high_ram {
            let threshold_bytes = (self.config.high_ram_threshold_gb as u64) * 1024 * 1024 * 1024;
            for proc in processes {
                if proc.memory_bytes >= threshold_bytes {
                    let should_alert = match self.last_alerted_pids.get(&proc.pid) {
                        Some(&prev_time) => now.duration_since(prev_time) > cooldown,
                        None => true,
                    };

                    if should_alert {
                        self.last_alerted_pids.insert(proc.pid, now);
                        let body = format!(
                            "{} (PID {}) is consuming {:.1} GB of memory.",
                            proc.name,
                            proc.pid,
                            proc.memory_gb()
                        );
                        send_notification("Memora High Memory Spike", &body);
                        break; // Limit to one process notification per sample tick
                    }
                }
            }
        }

        // Periodically purge expired PIDs from tracker
        if self.last_alerted_pids.len() > 100 {
            self.last_alerted_pids
                .retain(|_, time| now.duration_since(*time) < cooldown);
        }
    }

    fn export_report(&mut self) {
        let Some(ref snap) = self.latest_snapshot else {
            self.status_message = Some((
                "No memory snapshot available to export".to_string(),
                Instant::now(),
                true,
            ));
            return;
        };

        let now = Local::now();
        let filename = format!("Memora-Report-{}.json", now.format("%Y%m%d-%H%M%S"));

        let base_dir = std::env::var_os("HOME")
            .map(|h| std::path::PathBuf::from(h).join("Downloads"))
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));

        let report_path = base_dir.join(filename);

        #[derive(serde::Serialize)]
        struct DiagnosticReport<'a> {
            app_version: &'static str,
            timestamp: String,
            memory: &'a MemorySnapshot,
            storage: Option<&'a StorageSnapshot>,
            top_processes: &'a [ProcessInfo],
        }

        let top_count = self.processes.len().min(25);
        let report = DiagnosticReport {
            app_version: "0.1.0",
            timestamp: now.to_rfc3339(),
            memory: snap,
            storage: self.latest_storage.as_ref(),
            top_processes: &self.processes[..top_count],
        };

        match serde_json::to_string_pretty(&report) {
            Ok(json_str) => {
                if let Err(e) = std::fs::write(&report_path, json_str) {
                    self.status_message = Some((
                        format!("Failed to save report: {}", e),
                        Instant::now(),
                        true,
                    ));
                } else {
                    self.status_message = Some((
                        format!("Diagnostic snapshot saved to {}", report_path.display()),
                        Instant::now(),
                        false,
                    ));
                }
            }
            Err(e) => {
                self.status_message =
                    Some((format!("Serialization failed: {}", e), Instant::now(), true));
            }
        }
    }

    fn handle_keyboard_shortcuts(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Num1) {
                self.active_tab = AppTab::Overview;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num2) {
                self.active_tab = AppTab::Processes;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num3) {
                self.active_tab = AppTab::Cleanup;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num4) {
                self.active_tab = AppTab::Settings;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::F) {
                self.active_tab = AppTab::Processes;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::E) {
                self.export_report();
            }
            if i.modifiers.command && i.key_pressed(egui::Key::T) {
                self.config.theme = match self.config.theme {
                    ThemePreference::Dark => ThemePreference::Light,
                    _ => ThemePreference::Dark,
                };
                let _ = self.config.save();
            }
            if i.key_pressed(egui::Key::Escape) {
                self.inspector_process = None;
                self.confirm_dialog = None;
            }
        });
    }
}

impl Drop for MemoraApp {
    fn drop(&mut self) {
        self.stop_signal
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl eframe::App for MemoraApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // macOS background menu bar mode: hide window instead of quitting when close button is pressed
        #[cfg(target_os = "macos")]
        if self.config.show_menu_bar && ui.ctx().input(|i| i.viewport().close_requested()) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.is_window_visible
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.processes.clear();
            self.processes.shrink_to_fit();
            crate::system::macos::purge_heap_memory();
        }

        // Handle restore/unhide request from macOS Menu Bar
        #[cfg(target_os = "macos")]
        if crate::system::macos::menu_bar::take_show_window_request() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            self.is_window_visible
                .store(true, std::sync::atomic::Ordering::Relaxed);
        }

        // Lazily initialize native macOS Menu Bar once NSApp is active
        #[cfg(target_os = "macos")]
        if self.config.show_menu_bar && self.menu_bar.is_none() {
            self.menu_bar = MacosMenuBar::new();
            if let (Some(ref mut bar), Some(ref snap)) = (&mut self.menu_bar, &self.latest_snapshot)
            {
                bar.update(snap, self.config.menu_bar_display_mode);
            }
        }

        self.check_background_messages();

        // When running in background mode, skip rendering entire UI to minimize memory & GPU overhead
        #[cfg(target_os = "macos")]
        if !self
            .is_window_visible
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            ui.ctx().request_repaint_after(Duration::from_secs(2));
            return;
        }

        let is_system_dark = ui.visuals().dark_mode;
        self.update_theme(is_system_dark);
        self.handle_keyboard_shortcuts(ui);

        // Request frame repaint for smooth UI updates
        ui.ctx().request_repaint_after(Duration::from_millis(500));

        let theme = self.theme.clone();

        // Top Navigation & Header Panel (macOS Toolbar style)
        egui::Panel::top("top_header_panel")
            .frame(
                egui::Frame::new()
                    .fill(theme.card_bg)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(egui::Margin::symmetric(16, 10)),
            )
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let is_compact = avail_w < 720.0;
                let is_very_compact = avail_w < 580.0;

                ui.horizontal(|ui| {
                    // App Logo & Name (Apple Squircle Icon)
                    ui.horizontal(|ui| {
                        render_app_logo(ui, 22.0, &theme);
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Memora")
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });

                    ui.add_space(if is_compact { 10.0 } else { 20.0 });

                    // macOS Segmented Control Tabs Capsule
                    let tabs = [
                        (AppTab::Overview, "Overview", "⌘1"),
                        (AppTab::Processes, "Processes", "⌘2"),
                        (AppTab::Cleanup, "Cleanup", "⌘3"),
                        (AppTab::Settings, "Settings", "⌘4"),
                    ];

                    egui::Frame::new()
                        .fill(theme.subtle_bg)
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, theme.border_subtle))
                        .inner_margin(egui::Margin::symmetric(3, 3))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                            for (tab, label, shortcut) in tabs {
                                let is_active = self.active_tab == tab;
                                let tab_label = if is_compact {
                                    label.to_string()
                                } else {
                                    format!("{} ({})", label, shortcut)
                                };

                                let (bg_color, text_color) = if is_active {
                                    (theme.accent_color, Color32::WHITE)
                                } else {
                                    (Color32::TRANSPARENT, theme.text_secondary)
                                };

                                let btn = egui::Button::new(
                                    RichText::new(tab_label)
                                        .size(11.5)
                                        .strong()
                                        .color(text_color),
                                )
                                .fill(bg_color)
                                .corner_radius(CornerRadius::same(6));

                                if ui.add(btn).clicked() {
                                    self.active_tab = tab;
                                }
                            }
                        });

                    // Quick Theme Toggle Button (Apple native vector appearance icon)
                    ui.add_space(8.0);
                    let (theme_rect, theme_resp) =
                        ui.allocate_exact_size(vec2(28.0, 24.0), egui::Sense::click());
                    let theme_bg = if theme_resp.hovered() {
                        theme.card_bg
                    } else {
                        theme.subtle_bg
                    };
                    ui.painter().rect(
                        theme_rect,
                        CornerRadius::same(6),
                        theme_bg,
                        Stroke::new(1.0, theme.border_subtle),
                        egui::StrokeKind::Inside,
                    );
                    let icon_color = if theme_resp.hovered() {
                        theme.accent_color
                    } else {
                        theme.text_secondary
                    };
                    paint_theme_icon(
                        ui.painter(),
                        theme_rect.center(),
                        7.0,
                        self.config.theme,
                        icon_color,
                    );

                    if theme_resp.on_hover_text("Toggle Theme (⌘T)").clicked() {
                        self.config.theme = match self.config.theme {
                            ThemePreference::Dark => ThemePreference::Light,
                            _ => ThemePreference::Dark,
                        };
                        let _ = self.config.save();
                    }

                    // Top right status summary (Responsive: hides RAM text on very compact window)
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(ref snap) = self.latest_snapshot {
                            render_pressure_badge(ui, snap.pressure, &theme);

                            if !is_very_compact {
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} / {}",
                                        format_bytes(snap.used_bytes),
                                        format_bytes(snap.total_bytes)
                                    ))
                                    .size(11.5)
                                    .color(theme.text_secondary),
                                );
                            }
                        }
                    });
                });
            });

        // Bottom Status Bar (macOS subtle footer)
        egui::Panel::bottom("bottom_status_panel")
            .frame(
                egui::Frame::new()
                    .fill(theme.card_bg)
                    .stroke(Stroke::new(1.0, theme.border_subtle))
                    .inner_margin(egui::Margin::symmetric(16, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some((ref msg, time, is_error)) = self.status_message {
                        if time.elapsed() < Duration::from_secs(6) {
                            let color = if is_error {
                                theme.critical_color
                            } else {
                                theme.normal_color
                            };
                            ui.label(RichText::new(msg).size(11.0).strong().color(color));
                        } else {
                            self.status_message = None;
                        }
                    } else {
                        ui.label(
                            RichText::new(
                                "Memora • Native macOS System Monitor • ⌘E Export Report",
                            )
                            .size(11.0)
                            .color(theme.text_secondary),
                        );
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} processes • Sampling every {}s",
                                self.processes.len(),
                                self.config.refresh_interval_secs,
                            ))
                            .size(11.0)
                            .color(theme.text_secondary),
                        );
                    });
                });
            });

        // Central Content Area
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme.bg_fill)
                    .inner_margin(egui::Margin::symmetric(16, 12)),
            )
            .show(ui, |ui| match self.active_tab {
                AppTab::Overview => {
                    if let Some(action) = render_dashboard(
                        ui,
                        self.latest_snapshot.as_ref(),
                        self.latest_storage.as_ref(),
                        &self.history_service,
                        &self.processes,
                        &mut self.is_history_paused,
                        &theme,
                    ) {
                        match action {
                            DashboardAction::ExportReport => {
                                self.export_report();
                            }
                            DashboardAction::InspectProcess(proc) => {
                                self.inspector_process = Some(proc);
                            }
                            DashboardAction::NavigateToProcesses => {
                                self.active_tab = AppTab::Processes;
                            }
                            DashboardAction::NavigateToCaches => {
                                self.active_tab = AppTab::Cleanup;
                            }
                        }
                    }
                }
                AppTab::Processes => {
                    let filtered_procs = ProcessService::filter_and_sort(
                        &self.processes,
                        &self.process_view_state.search_query,
                        self.process_view_state.filter_category,
                        self.process_view_state.sort_column,
                        self.process_view_state.sort_direction,
                    );

                    if let Some(req) =
                        render_processes(ui, &filtered_procs, &mut self.process_view_state, &theme)
                    {
                        match req {
                            ProcessActionRequest::RequestQuit(proc, action) => {
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: Some(proc),
                                    action,
                                });
                            }
                            ProcessActionRequest::Inspect(proc) => {
                                self.inspector_process = Some(proc);
                            }
                        }
                    }
                }
                AppTab::Cleanup => {
                    // Lazy-load user caches & storage junk upon first visiting Cleanup tab
                    if self.cache_view_state.cache_scan_result.is_none()
                        && !self.cache_view_state.is_scanning_caches
                    {
                        self.cache_view_state.is_scanning_caches = true;
                        let tx = self.sender.clone();
                        thread::spawn(move || {
                            let scan_res = CacheService::scan_user_caches();
                            let _ = tx.send(BackgroundMessage::CachesScanned(scan_res));
                        });
                    }
                    if self.cache_view_state.storage_junk.is_none()
                        && !self.cache_view_state.is_scanning_storage
                    {
                        self.cache_view_state.is_scanning_storage = true;
                        let tx = self.sender.clone();
                        thread::spawn(move || {
                            let junk = StorageCleanService::scan_all_junk();
                            let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                        });
                    }

                    if let Some(req) = render_caches(
                        ui,
                        self.latest_snapshot.as_ref(),
                        &mut self.cache_view_state,
                        &theme,
                    ) {
                        match req {
                            CleanupActionRequest::ScanStorageJunk => {
                                self.cache_view_state.is_scanning_storage = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let junk = StorageCleanService::scan_all_junk();
                                    let _ = tx.send(BackgroundMessage::StorageJunkScanned(junk));
                                });
                            }
                            CleanupActionRequest::EmptyTrash => {
                                if let Some(ref junk) = self.cache_view_state.storage_junk {
                                    self.confirm_dialog = Some(ConfirmDialogState {
                                        process: None,
                                        action: ConfirmAction::EmptyTrash {
                                            item_count: junk.trash.item_count,
                                            total_bytes: junk.trash.total_bytes,
                                        },
                                    });
                                }
                            }
                            CleanupActionRequest::DeleteSelectedInstallers(paths) => {
                                let total_bytes: u64 = paths
                                    .iter()
                                    .map(|p| p.metadata().map(|m| m.len()).unwrap_or(0))
                                    .sum();
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: None,
                                    action: ConfirmAction::DeleteInstallers {
                                        count: paths.len(),
                                        total_bytes,
                                        paths,
                                    },
                                });
                            }
                            CleanupActionRequest::PurgeApfsSnapshots => {
                                self.cache_view_state.is_cleaning_storage = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = StorageCleanService::clean_apfs_snapshots();
                                    let _ = tx.send(BackgroundMessage::ApfsSnapshotsCleaned(res));
                                });
                            }
                            CleanupActionRequest::PruneUnavailableSimulators => {
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = StorageCleanService::clean_unavailable_simulators();
                                    let _ = tx.send(BackgroundMessage::SimulatorsPruned(res));
                                });
                            }
                            CleanupActionRequest::ScanLargeFiles(min_bytes) => {
                                self.cache_view_state.is_scanning_large_files = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = LargeFilesService::scan_large_files(min_bytes, None);
                                    let _ = tx.send(BackgroundMessage::LargeFilesScanned(res));
                                });
                            }
                            CleanupActionRequest::RevealLargeFileInFinder(path) => {
                                let _ = LargeFilesService::reveal_in_finder(&path);
                            }
                            CleanupActionRequest::DeleteSelectedLargeFiles(paths) => {
                                let total_bytes: u64 = paths
                                    .iter()
                                    .map(|p| p.metadata().map(|m| m.len()).unwrap_or(0))
                                    .sum();
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: None,
                                    action: ConfirmAction::DeleteLargeFiles {
                                        count: paths.len(),
                                        total_bytes,
                                        paths,
                                    },
                                });
                            }
                            CleanupActionRequest::ScanCaches => {
                                self.cache_view_state.is_scanning_caches = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = CacheService::scan_user_caches();
                                    let _ = tx.send(BackgroundMessage::CachesScanned(res));
                                });
                            }
                            CleanupActionRequest::CleanSelectedCaches(paths) => {
                                let total_bytes: u64 = if let Some(ref scan) =
                                    self.cache_view_state.cache_scan_result
                                {
                                    scan.selected_total_bytes()
                                } else {
                                    0
                                };
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: None,
                                    action: ConfirmAction::CleanCaches {
                                        count: paths.len(),
                                        total_bytes,
                                        paths,
                                    },
                                });
                            }
                            CleanupActionRequest::PurgeRamCache => {
                                self.confirm_dialog = Some(ConfirmDialogState {
                                    process: None,
                                    action: ConfirmAction::PurgeRam,
                                });
                            }
                            CleanupActionRequest::FlushDns => {
                                match CacheService::flush_dns_cache() {
                                    Ok(_) => {
                                        self.status_message = Some((
                                            "DNS cache flushed successfully".to_string(),
                                            Instant::now(),
                                            false,
                                        ));
                                    }
                                    Err(e) => {
                                        self.status_message = Some((
                                            format!("Failed to flush DNS cache: {}", e),
                                            Instant::now(),
                                            true,
                                        ));
                                    }
                                }
                            }
                            CleanupActionRequest::FlushQuickLook => {
                                match CacheService::flush_quicklook_cache() {
                                    Ok(_) => {
                                        self.status_message = Some((
                                            "QuickLook thumbnail cache reset successfully"
                                                .to_string(),
                                            Instant::now(),
                                            false,
                                        ));
                                    }
                                    Err(e) => {
                                        self.status_message = Some((
                                            format!("Failed to reset QuickLook cache: {}", e),
                                            Instant::now(),
                                            true,
                                        ));
                                    }
                                }
                            }
                        }
                    }
                }
                AppTab::Settings => {
                    let prev_config = self.config.clone();
                    render_settings(ui, &mut self.config, &theme);
                    self.history_service
                        .buffer_mut()
                        .set_capacity(self.config.history_capacity);
                    self.refresh_interval_atomic.store(
                        self.config.refresh_interval_secs.max(1),
                        std::sync::atomic::Ordering::Relaxed,
                    );
                    #[cfg(target_os = "macos")]
                    if self.config.show_menu_bar != prev_config.show_menu_bar
                        || self.config.menu_bar_display_mode != prev_config.menu_bar_display_mode
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

                    if self.config != prev_config {
                        let _ = self.config.save();
                    }
                }
            });

        // Render Process Inspector Modal if open
        if let Some(proc) = self.inspector_process.clone() {
            let ctx = ui.ctx().clone();
            if let Some(inspector_action) = render_process_inspector(&ctx, &proc, &theme) {
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
                    InspectorAction::Close => {
                        self.inspector_process = None;
                    }
                }
            }
        }

        // Render Confirmation Dialog if open
        if let Some(dialog_state) = self.confirm_dialog.clone() {
            let ctx = ui.ctx().clone();
            if let Some(result) = render_confirm_dialog(&ctx, &dialog_state, &theme) {
                match result {
                    DialogResult::Confirmed(action) => {
                        match action {
                            ConfirmAction::SafeQuit => {
                                if let Some(ref proc) = dialog_state.process {
                                    let pid = proc.pid;
                                    let proc_name = proc.name.clone();
                                    match self.process_service.terminate(pid) {
                                        Ok(_) => {
                                            self.status_message = Some((
                                                format!(
                                                    "Termination request sent to {} (PID {})",
                                                    proc_name, pid
                                                ),
                                                Instant::now(),
                                                false,
                                            ));
                                        }
                                        Err(e) => {
                                            self.status_message = Some((
                                                format!("Failed to quit {}: {}", proc_name, e),
                                                Instant::now(),
                                                true,
                                            ));
                                        }
                                    }
                                }
                            }
                            ConfirmAction::ForceQuit => {
                                if let Some(ref proc) = dialog_state.process {
                                    let pid = proc.pid;
                                    let proc_name = proc.name.clone();
                                    match self.process_service.force_kill(pid) {
                                        Ok(_) => {
                                            self.status_message = Some((
                                                format!("Force quit {} (PID {})", proc_name, pid),
                                                Instant::now(),
                                                false,
                                            ));
                                        }
                                        Err(e) => {
                                            self.status_message = Some((
                                                format!(
                                                    "Failed to force quit {}: {}",
                                                    proc_name, e
                                                ),
                                                Instant::now(),
                                                true,
                                            ));
                                        }
                                    }
                                }
                            }
                            ConfirmAction::EmptyTrash { .. } => {
                                self.cache_view_state.is_cleaning_storage = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = StorageCleanService::empty_trash();
                                    let _ = tx.send(BackgroundMessage::TrashEmptied(res));
                                });
                            }
                            ConfirmAction::DeleteInstallers { paths, .. } => {
                                self.cache_view_state.is_cleaning_storage = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = StorageCleanService::clean_installers(&paths);
                                    let _ = tx.send(BackgroundMessage::InstallersCleaned(res));
                                });
                            }
                            ConfirmAction::DeleteLargeFiles { paths, .. } => {
                                self.cache_view_state.is_cleaning_large_files = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = LargeFilesService::move_multiple_to_trash(&paths);
                                    let _ = tx.send(BackgroundMessage::LargeFilesDeleted(res));
                                });
                            }
                            ConfirmAction::CleanCaches { paths, .. } => {
                                self.cache_view_state.is_cleaning_caches = true;
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let freed =
                                        CacheService::clean_cache_items(&paths).unwrap_or(0);
                                    let _ = tx.send(BackgroundMessage::CacheCleaned(freed));
                                });
                            }
                            ConfirmAction::PurgeRam => {
                                self.cache_view_state.is_purging_ram = true;
                                let cache_svc = self.cache_service.clone();
                                let tx = self.sender.clone();
                                thread::spawn(move || {
                                    let res = cache_svc.purge_system_ram_cache();
                                    let _ = tx.send(BackgroundMessage::RamPurged(res));
                                });
                            }
                        }
                        self.confirm_dialog = None;
                    }
                    DialogResult::Cancelled => {
                        self.confirm_dialog = None;
                    }
                }
            }
        }
    }
}
