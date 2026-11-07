use egui::{vec2, Color32, CornerRadius, Rect, RichText, Ui};

use crate::models::{MemorySnapshot, ProcessInfo};
use crate::ui::components::render_pressure_badge;
use crate::ui::dashboard::DashboardAction;
use crate::ui::helpers::{process_status_color, render_section_header, render_status_dot};
use crate::ui::theme::AppTheme;
use crate::utils::format::{format_bytes, format_time_hhmmss};

pub(super) fn render_header(
    ui: &mut Ui,
    snap: &MemorySnapshot,
    action: &mut Option<DashboardAction>,
    theme: &AppTheme,
) {
    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            render_pressure_badge(ui, snap.pressure, theme);
            ui.add_space(10.0);
            let export_btn = egui::Button::new(
                RichText::new("Export Snapshot")
                    .size(11.5)
                    .color(theme.text_primary),
            )
            .fill(theme.card_bg)
            .stroke(theme.card_stroke())
            .corner_radius(theme.button_radius());
            if ui
                .add(export_btn)
                .on_hover_text("Export diagnostic JSON report (⌘E)")
                .clicked()
            {
                *action = Some(DashboardAction::ExportReport);
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("Memory Overview")
                            .size(22.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let sub = format!(
                        "Updated {} • {}",
                        format_time_hhmmss(snap.timestamp),
                        snap.pressure.description()
                    );
                    ui.label(RichText::new(sub).size(12.0).color(theme.text_secondary));
                });
            });
        });
    });
}

pub(super) fn render_composition_card(
    ui: &mut Ui,
    snap: &MemorySnapshot,
    action: &mut Option<DashboardAction>,
    theme: &AppTheme,
) {
    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "Memory Composition", theme);
        ui.add_space(10.0);

        let total_f = snap.total_bytes as f32;
        let segments = [
            (snap.app_memory_bytes, theme.accent_color, "App Memory"),
            (snap.wired_bytes, theme.purple_color, "Wired"),
            (snap.compressed_bytes, theme.warning_color, "Compressed"),
            (snap.cached_files_bytes, theme.cyan_color, "Cached Files"),
            (snap.free_bytes, theme.normal_color, "Free"),
        ];

        let bar_h = 16.0;
        let (bar_rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), bar_h), egui::Sense::hover());
        ui.painter()
            .rect_filled(bar_rect, CornerRadius::same(5), theme.border_subtle);

        let mut x = bar_rect.min.x;
        for (bytes, color, _) in &segments {
            let w = bar_rect.width() * (*bytes as f32 / total_f);
            if w > 0.5 {
                let seg_rect = Rect::from_min_size(egui::pos2(x, bar_rect.min.y), vec2(w, bar_h));
                ui.painter()
                    .rect_filled(seg_rect, CornerRadius::ZERO, *color);
                x += w;
            }
        }

        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(16.0, 6.0);
            for (bytes, color, name) in &segments {
                ui.horizontal(|ui| {
                    let (dot, _) = ui.allocate_exact_size(vec2(8.0, 8.0), egui::Sense::hover());
                    ui.painter().rect_filled(dot, CornerRadius::same(3), *color);
                    let txt = format!("{}: {}", name, format_bytes(*bytes));
                    ui.label(RichText::new(txt).size(11.5).color(theme.text_secondary));
                });
            }
        });

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = egui::Button::new(
                    RichText::new("Clear & Manage Caches →")
                        .size(11.0)
                        .color(theme.accent_color),
                )
                .fill(Color32::TRANSPARENT);
                if ui.add(btn).clicked() {
                    *action = Some(DashboardAction::NavigateToCaches);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let note = format!(
                        "Cached files in RAM: {} (reclaimable: {})",
                        format_bytes(snap.cached_files_bytes),
                        format_bytes(snap.purgeable_bytes)
                    );
                    ui.label(RichText::new(note).size(11.0).color(theme.text_secondary));
                });
            });
        });
    });
}

pub(super) fn render_top_consumers_card(
    ui: &mut Ui,
    top_processes: &[ProcessInfo],
    action: &mut Option<DashboardAction>,
    theme: &AppTheme,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let btn = egui::Button::new(
                    RichText::new("View All Processes →")
                        .size(11.5)
                        .color(theme.accent_color),
                )
                .frame(false);
                if ui.add(btn).clicked() {
                    *action = Some(DashboardAction::NavigateToProcesses);
                }
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    render_section_header(ui, "Top Memory Consumers", theme);
                });
            });
        });
        ui.add_space(10.0);

        for proc in top_processes.iter().take(5) {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(theme.subtle_button(RichText::new("Inspect").size(11.0)))
                        .clicked()
                    {
                        *action = Some(DashboardAction::InspectProcess(proc.clone()));
                    }
                    ui.add_space(12.0);
                    if let Some(cpu) = proc.cpu_usage {
                        ui.label(
                            RichText::new(format!("{:.1}% CPU", cpu))
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                        ui.add_space(12.0);
                    }
                    ui.label(
                        RichText::new(format_bytes(proc.memory_bytes))
                            .size(13.0)
                            .strong()
                            .color(theme.accent_color),
                    );
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        render_status_dot(ui, process_status_color(proc, theme), 3.0);
                        ui.label(
                            RichText::new(&proc.name)
                                .size(13.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new(format!("(PID: {})", proc.pid))
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                    });
                });
            });
            ui.add_space(6.0);
        }
    });
}
