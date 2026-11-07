use egui::{Color32, CornerRadius, RichText, Ui};
use egui_plot::{Legend, Line, Plot, PlotPoints};

use crate::models::{MemoryPressure, MemorySnapshot};
use crate::services::HistoryService;
use crate::ui::dashboard::DashboardAction;
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub(super) fn render_pressure_banner(
    ui: &mut Ui,
    snap: &MemorySnapshot,
    action: &mut Option<DashboardAction>,
    theme: &AppTheme,
) {
    if !matches!(
        snap.pressure,
        MemoryPressure::Warning | MemoryPressure::Critical
    ) {
        return;
    }
    ui.add_space(12.0);
    let is_crit = snap.pressure == MemoryPressure::Critical;
    let c = if is_crit {
        theme.critical_color
    } else {
        theme.warning_color
    };
    let bg = Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 26);

    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(10))
        .stroke(egui::Stroke::new(1.0, c))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let msg = if is_crit {
                    "Critical Memory Pressure: System is paging compressed memory to disk swapfile."
                } else {
                    "Elevated Memory Pressure: macOS compressor is actively compacting inactive memory."
                };
                ui.label(RichText::new(msg).size(12.0).strong().color(c));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let btn = egui::Button::new(
                        RichText::new("Manage Processes →")
                            .size(11.5)
                            .strong()
                            .color(Color32::WHITE),
                    )
                    .fill(c)
                    .corner_radius(theme.button_radius());
                    if ui.add(btn).clicked() {
                        *action = Some(DashboardAction::NavigateToProcesses);
                    }
                });
            });
        });
}

pub(super) fn render_history_chart_card(
    ui: &mut Ui,
    snap: &MemorySnapshot,
    history_svc: &HistoryService,
    is_history_paused: &mut bool,
    theme: &AppTheme,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let stats_txt = format!(
                    "Peak: {} • Avg: {}",
                    format_bytes(history_svc.peak_used_bytes()),
                    format_bytes(history_svc.average_used_bytes())
                );
                ui.label(
                    RichText::new(stats_txt)
                        .size(11.0)
                        .color(theme.text_secondary),
                );

                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    render_section_header(ui, "Memory History (GB)", theme);
                    ui.add_space(10.0);

                    let (pause_label, pause_color) = if *is_history_paused {
                        ("Resume Live", theme.warning_color)
                    } else {
                        ("Pause Live", theme.text_secondary)
                    };
                    if ui
                        .add(theme.subtle_button(
                            RichText::new(pause_label).size(11.0).color(pause_color),
                        ))
                        .clicked()
                    {
                        *is_history_paused = !*is_history_paused;
                    }

                    ui.add_space(8.0);
                    let range_id = ui.make_persistent_id("history_time_range_secs");
                    let mut range_secs: u32 =
                        ui.data_mut(|d| *d.get_temp_mut_or_default::<u32>(range_id));

                    for (r_val, r_label) in [(0u32, "All"), (60u32, "1m"), (180u32, "3m")] {
                        let is_sel = range_secs == r_val;
                        let (bg, fg) = if is_sel {
                            (theme.accent_color, Color32::WHITE)
                        } else {
                            (Color32::TRANSPARENT, theme.text_secondary)
                        };
                        let btn =
                            egui::Button::new(RichText::new(r_label).size(10.5).strong().color(fg))
                                .fill(bg)
                                .corner_radius(CornerRadius::same(5));
                        if ui.add(btn).clicked() {
                            range_secs = r_val;
                            ui.data_mut(|d| d.insert_temp(range_id, range_secs));
                        }
                    }
                });
            });
        });

        ui.add_space(8.0);

        let range_id = ui.make_persistent_id("history_time_range_secs");
        let range_secs: u32 = ui.data_mut(|d| *d.get_temp_mut_or_default::<u32>(range_id));

        let max_ts = history_svc
            .buffer()
            .points()
            .back()
            .map_or(0.0, |p| p.timestamp_secs);
        let min_ts = if range_secs > 0 {
            (max_ts - range_secs as f64).max(0.0)
        } else {
            0.0
        };

        let pts = history_svc.buffer().points();
        let used_points: PlotPoints = pts
            .iter()
            .filter(|p| p.timestamp_secs >= min_ts)
            .map(|p| [p.timestamp_secs, p.used_gb])
            .collect();
        let compressed_points: PlotPoints = pts
            .iter()
            .filter(|p| p.timestamp_secs >= min_ts)
            .map(|p| [p.timestamp_secs, p.compressed_gb])
            .collect();
        let swap_points: PlotPoints = pts
            .iter()
            .filter(|p| p.timestamp_secs >= min_ts)
            .map(|p| [p.timestamp_secs, p.swap_gb])
            .collect();

        Plot::new("memory_history_plot")
            .legend(Legend::default().position(egui_plot::Corner::RightBottom))
            .show_axes([true, true])
            .show_grid(true)
            .height(200.0)
            .include_y(0.0)
            .include_y(snap.total_bytes as f64 / (1024.0 * 1024.0 * 1024.0))
            .show(ui, |plot_ui| {
                plot_ui.line(
                    Line::new("Used Memory", used_points)
                        .color(theme.accent_color)
                        .width(2.0),
                );
                plot_ui.line(
                    Line::new("Compressed", compressed_points)
                        .color(theme.warning_color)
                        .width(1.5),
                );
                plot_ui.line(
                    Line::new("Swap", swap_points)
                        .color(theme.critical_color)
                        .width(1.5),
                );
            });
    });
}
