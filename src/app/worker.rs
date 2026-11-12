use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use crate::services::{MemoryService, ProcessService, StorageService, ThermalService};

use super::BackgroundMessage;

#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_background_worker(
    mem_s: Arc<MemoryService>,
    proc_s: Arc<ProcessService>,
    stor_s: Arc<StorageService>,
    therm_s: Arc<ThermalService>,
    stop: Arc<AtomicBool>,
    interval: Arc<AtomicU64>,
    tx: Sender<BackgroundMessage>,
    vis: Arc<AtomicBool>,
    egui_ctx: egui::Context,
) {
    thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            let start = Instant::now();
            let is_vis = vis.load(Ordering::Relaxed);
            let processes = if is_vis {
                proc_s.list_processes().unwrap_or_default()
            } else {
                Vec::new()
            };

            let thermal = therm_s.snapshot().unwrap_or_default();

            match (mem_s.snapshot(), stor_s.snapshot()) {
                (Ok(snapshot), Ok(storage)) => {
                    #[cfg(target_os = "macos")]
                    crate::system::macos::menu_bar::dispatch_menu_bar_telemetry(
                        snapshot.clone(),
                        Some(thermal.clone()),
                    );

                    let _ = tx.send(BackgroundMessage::MetricsUpdated {
                        snapshot,
                        storage: Box::new(storage),
                        thermal: Box::new(thermal),
                        processes,
                    });
                    if is_vis {
                        egui_ctx.request_repaint();
                    }
                }
                (Err(e), _) => {
                    let _ = tx.send(BackgroundMessage::CollectionError(e.to_string()));
                }
                (_, Err(e)) => {
                    let _ = tx.send(BackgroundMessage::CollectionError(e));
                }
            }

            let target_dur = Duration::from_secs(interval.load(Ordering::Relaxed).max(1));
            while start.elapsed() < target_dur && !stop.load(Ordering::Relaxed) {
                if !is_vis && vis.load(Ordering::Relaxed) {
                    break;
                }
                thread::sleep(Duration::from_millis(100));
            }
        }
    });
}
