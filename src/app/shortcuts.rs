use std::time::{Duration, Instant};

use chrono::Local;

use crate::app::{AppTab, BackgroundMessage, MemoraApp};
use crate::config::ThemePreference;
use crate::models::{MemoryPressure, MemorySnapshot, ProcessInfo, StorageSnapshot};
use crate::services::StorageCleanService;
use crate::system::macos::send_notification;
use crate::utils::format::format_bytes;

impl MemoraApp {
    pub(crate) fn handle_storage_res(
        &mut self,
        res: Result<u64, String>,
        ok_prefix: &str,
        err_prefix: &str,
    ) {
        match res {
            Ok(freed) => {
                self.set_status(format!("{} {}", ok_prefix, format_bytes(freed)), false);
                self.spawn_storage_junk_rescan();
            }
            Err(e) => self.set_status(format!("{}: {}", err_prefix, e), true),
        }
    }

    pub(crate) fn set_status(&mut self, msg: String, is_error: bool) {
        self.status_message = Some((msg, Instant::now(), is_error));
    }

    pub(crate) fn spawn_storage_junk_rescan(&self) {
        self.spawn_bg(|| {
            BackgroundMessage::StorageJunkScanned(StorageCleanService::scan_all_junk())
        });
    }

    pub(crate) fn check_alerts(&mut self, snapshot: &MemorySnapshot, processes: &[ProcessInfo]) {
        let (now, cooldown) = (Instant::now(), Duration::from_secs(300));
        if self.config.notify_on_pressure_warning
            && matches!(
                snapshot.pressure,
                MemoryPressure::Warning | MemoryPressure::Critical
            )
            && self
                .last_pressure_alert
                .is_none_or(|(p, t)| snapshot.pressure != p || now.duration_since(t) > cooldown)
        {
            self.last_pressure_alert = Some((snapshot.pressure, now));
            let lvl = if snapshot.pressure == MemoryPressure::Critical {
                "Critical"
            } else {
                "Elevated"
            };
            send_notification(
                "Memora System Warning",
                &format!(
                    "macOS memory pressure is {}. Compressor is actively compacting RAM.",
                    lvl
                ),
            );
        }

        if self.config.notify_on_high_ram {
            let threshold = (self.config.high_ram_threshold_gb as u64) * 1024 * 1024 * 1024;
            for proc in processes {
                if proc.memory_bytes >= threshold
                    && self
                        .last_alerted_pids
                        .get(&proc.pid)
                        .is_none_or(|&t| now.duration_since(t) > cooldown)
                {
                    self.last_alerted_pids.insert(proc.pid, now);
                    send_notification(
                        "Memora High Memory Spike",
                        &format!(
                            "{} (PID {}) is consuming {:.1} GB of memory.",
                            proc.name,
                            proc.pid,
                            proc.memory_gb()
                        ),
                    );
                    break;
                }
            }
        }
        if self.last_alerted_pids.len() > 100 {
            self.last_alerted_pids
                .retain(|_, t| now.duration_since(*t) < cooldown);
        }
    }

    pub(crate) fn export_report(&mut self) {
        let Some(ref snap) = self.latest_snapshot else {
            self.set_status("No memory snapshot available to export".to_string(), true);
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

        let report = DiagnosticReport {
            app_version: env!("CARGO_PKG_VERSION"),
            timestamp: now.to_rfc3339(),
            memory: snap,
            storage: self.latest_storage.as_ref(),
            top_processes: &self.processes[..self.processes.len().min(25)],
        };

        match serde_json::to_string_pretty(&report) {
            Ok(json) => match std::fs::write(&report_path, json) {
                Ok(()) => self.set_status(
                    format!("Diagnostic snapshot saved to {}", report_path.display()),
                    false,
                ),
                Err(e) => self.set_status(format!("Failed to save report: {}", e), true),
            },
            Err(e) => self.set_status(format!("Serialization failed: {}", e), true),
        }
    }

    pub(crate) fn handle_keyboard_shortcuts(&mut self, ui: &egui::Ui) {
        ui.input(|i| {
            if i.modifiers.command && i.key_pressed(egui::Key::Num1) {
                self.active_tab = AppTab::Overview;
            }
            if i.modifiers.command
                && (i.key_pressed(egui::Key::Num2) || i.key_pressed(egui::Key::F))
            {
                self.active_tab = AppTab::Processes;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num3) {
                self.active_tab = AppTab::Cleanup;
            }
            if i.modifiers.command && i.key_pressed(egui::Key::Num4) {
                self.active_tab = AppTab::Settings;
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
