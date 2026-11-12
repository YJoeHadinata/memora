mod chart;
mod sections;
mod thermal;
mod volumes;

use egui::{vec2, RichText, ScrollArea, Ui};

use crate::models::{MemorySnapshot, ProcessInfo, StorageSnapshot, ThermalSnapshot};
use crate::services::HistoryService;
use crate::ui::components::MetricCard;
use crate::ui::theme::AppTheme;
use crate::utils::format::{format_bytes, format_percent};

use self::chart::{render_history_chart_card, render_pressure_banner};
use self::sections::{render_composition_card, render_header, render_top_consumers_card};
use self::thermal::render_thermal_card;
use self::volumes::render_storage_volumes_card;

pub enum DashboardAction {
    ExportReport,
    InspectProcess(ProcessInfo),
    NavigateToProcesses,
    NavigateToCaches,
}

#[allow(clippy::too_many_arguments)]
pub fn render_dashboard(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    storage: Option<&StorageSnapshot>,
    thermal: Option<&ThermalSnapshot>,
    history_svc: &HistoryService,
    top_processes: &[ProcessInfo],
    is_history_paused: &mut bool,
    theme: &AppTheme,
) -> Option<DashboardAction> {
    let mut action = None;

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_width = (ui.available_width() - 4.0).max(200.0);
            ui.set_max_width(avail_width);
            ui.add_space(6.0);

            let Some(snap) = snapshot else {
                ui.vertical_centered(|ui| {
                    ui.add_space(80.0);
                    ui.spinner();
                    ui.add_space(12.0);
                    ui.label(
                        RichText::new("Collecting macOS memory metrics...")
                            .size(14.0)
                            .color(theme.text_secondary),
                    );
                });
                return;
            };

            render_header(ui, snap, &mut action, theme);
            render_pressure_banner(ui, snap, &mut action, theme);
            ui.add_space(14.0);
            render_metric_cards(ui, snap, storage, avail_width, theme);
            ui.add_space(18.0);

            if let Some(th) = thermal {
                render_thermal_card(ui, th, theme);
                ui.add_space(18.0);
            }

            render_composition_card(ui, snap, &mut action, theme);
            ui.add_space(18.0);

            if let Some(st) = storage {
                render_storage_volumes_card(ui, st, theme);
                ui.add_space(18.0);
            }
            if !top_processes.is_empty() {
                render_top_consumers_card(ui, top_processes, &mut action, theme);
                ui.add_space(18.0);
            }

            render_history_chart_card(ui, snap, history_svc, is_history_paused, theme);
            ui.add_space(20.0);
        });

    action
}

fn render_metric_cards(
    ui: &mut Ui,
    snap: &MemorySnapshot,
    storage: Option<&StorageSnapshot>,
    avail_width: f32,
    theme: &AppTheme,
) {
    let (cols, spacing) = if avail_width >= 720.0 {
        (3usize, 12.0)
    } else if avail_width >= 480.0 {
        (2usize, 10.0)
    } else {
        (1usize, 8.0)
    };
    let card_w = ((avail_width - (cols as f32 - 1.0) * spacing) / cols as f32)
        .floor()
        .max(100.0);

    let used_pct = snap.used_percent();
    let used_c = if used_pct > 85.0 {
        theme.critical_color
    } else if used_pct > 70.0 {
        theme.warning_color
    } else {
        theme.accent_color
    };
    let avail_pct = snap.available_percent();
    let swap_sub = if snap.swap_total_bytes > 0 {
        format!("of {} total swap", format_bytes(snap.swap_total_bytes))
    } else {
        "No swap files allocated".to_string()
    };
    let swap_c = if snap.swap_used_bytes > 1024 * 1024 * 1024 {
        theme.warning_color
    } else {
        theme.text_secondary
    };
    let swap_prog = if snap.swap_total_bytes > 0 {
        snap.swap_used_percent() / 100.0
    } else {
        0.0
    };

    let mut cards = vec![
        MetricCard::new("PHYSICAL MEMORY", format_bytes(snap.total_bytes))
            .subtitle("Apple Silicon Unified Memory"),
        MetricCard::new("MEMORY USED", format_bytes(snap.used_bytes))
            .subtitle(format!("{} of total RAM", format_percent(used_pct)))
            .progress(used_pct / 100.0, used_c),
        MetricCard::new("AVAILABLE MEMORY", format_bytes(snap.available_bytes))
            .subtitle(format!("{} free or reclaimable", format_percent(avail_pct)))
            .progress(avail_pct / 100.0, theme.normal_color),
        MetricCard::new("COMPRESSED", format_bytes(snap.compressed_bytes))
            .subtitle("macOS memory compressor"),
        MetricCard::new("SWAP USED", format_bytes(snap.swap_used_bytes))
            .subtitle(swap_sub)
            .progress(swap_prog, swap_c),
    ];

    if let Some(p) = storage.and_then(|s| s.primary_volume.as_ref()) {
        let dp = p.used_percent();
        let dc = if dp > 88.0 {
            theme.critical_color
        } else if dp > 75.0 {
            theme.warning_color
        } else {
            theme.accent_color
        };
        let sub = format!(
            "{} used ({} free)",
            format_percent(dp),
            format_bytes(p.available_bytes)
        );
        cards.push(
            MetricCard::new("STORAGE USED", format_bytes(p.used_bytes))
                .subtitle(sub)
                .progress(dp / 100.0, dc),
        );
    }

    for (row_idx, chunk) in cards.chunks(cols).enumerate() {
        if row_idx > 0 {
            ui.add_space(spacing);
        }
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = vec2(spacing, 0.0);
            for card in chunk {
                card.clone().width(card_w).ui(ui, theme);
            }
        });
    }
}
