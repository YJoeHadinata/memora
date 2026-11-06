use egui::{pos2, vec2, Align2, RichText, ScrollArea, Window};

use crate::models::ProcessInfo;
use crate::ui::components::ConfirmAction;
use crate::ui::helpers::{process_status_color, render_section_header, render_status_dot};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub enum InspectorAction {
    Quit(ProcessInfo, ConfirmAction),
    RevealInFinder(String),
    Close,
}

pub fn render_process_inspector(
    ctx: &egui::Context,
    proc: &ProcessInfo,
    theme: &AppTheme,
) -> Option<InspectorAction> {
    let mut action = None;
    let modal_w = (ctx.viewport_rect().width() * 0.9).clamp(340.0, 520.0);

    Window::new(format!("Process Details: {}", proc.name))
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .fixed_size([modal_w, 440.0])
        .frame(theme.modal_frame())
        .show(ctx, |ui| {
            // Header: Name, PID, Type
            ui.horizontal(|ui| {
                render_status_dot(ui, process_status_color(proc, theme), 5.0);

                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&proc.name)
                            .size(17.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Process ID: {} • {} • {}",
                            proc.pid,
                            proc.display_type(),
                            proc.architecture.display_name()
                        ))
                        .size(11.5)
                        .color(theme.text_secondary),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (close_rect, close_resp) =
                        ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::click());
                    if close_resp.hovered() {
                        ui.painter()
                            .circle_filled(close_rect.center(), 10.0, theme.subtle_bg);
                    }
                    let x_color = if close_resp.hovered() {
                        theme.text_primary
                    } else {
                        theme.text_secondary
                    };
                    let r = 4.0;
                    let c = close_rect.center();
                    ui.painter().line_segment(
                        [pos2(c.x - r, c.y - r), pos2(c.x + r, c.y + r)],
                        egui::Stroke::new(1.3, x_color),
                    );
                    ui.painter().line_segment(
                        [pos2(c.x + r, c.y - r), pos2(c.x - r, c.y + r)],
                        egui::Stroke::new(1.3, x_color),
                    );
                    if close_resp.clicked() {
                        action = Some(InspectorAction::Close);
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                // Section 1: Memory Metrics
                render_section_header(ui, "MEMORY CONSUMPTION", theme);
                ui.add_space(6.0);

                egui::Grid::new("mem_details_grid")
                    .num_columns(2)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("Physical Footprint (Memory):")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format_bytes(proc.memory_bytes))
                                .size(12.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Resident Size (RSS):")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format_bytes(proc.resident_bytes))
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Lifetime Peak Footprint:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format_bytes(proc.peak_memory_bytes))
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Kernel Pageins:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format!("{}", proc.pageins))
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.end_row();
                    });

                ui.add_space(14.0);

                // Section 2: CPU & I/O
                render_section_header(ui, "ACTIVITY & I/O", theme);
                ui.add_space(6.0);

                egui::Grid::new("io_details_grid")
                    .num_columns(2)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new("CPU Usage:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        let cpu_str = proc
                            .cpu_usage
                            .map_or("0.0%".to_string(), |c| format!("{:.1}%", c));
                        ui.label(
                            RichText::new(cpu_str)
                                .size(12.0)
                                .strong()
                                .color(theme.accent_color),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Binary Architecture:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(proc.architecture.display_name())
                                .size(12.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Disk Bytes Read:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format_bytes(proc.disk_read_bytes))
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.end_row();

                        ui.label(
                            RichText::new("Disk Bytes Written:")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                        ui.label(
                            RichText::new(format_bytes(proc.disk_write_bytes))
                                .size(12.0)
                                .color(theme.text_primary),
                        );
                        ui.end_row();
                    });

                ui.add_space(14.0);

                // Section 3: Executable Path
                render_section_header(ui, "EXECUTABLE PATH", theme);
                ui.add_space(6.0);

                if let Some(ref path) = proc.path {
                    ui.label(RichText::new(path).size(11.0).color(theme.text_primary));

                    ui.add_space(6.0);
                    if ui.add(theme.subtle_button("Reveal in Finder")).clicked() {
                        action = Some(InspectorAction::RevealInFinder(path.clone()));
                    }
                } else {
                    ui.label(
                        RichText::new(
                            "Path not available (system kernel service or sandboxed entry)",
                        )
                        .size(11.0)
                        .italics()
                        .color(theme.text_secondary),
                    );
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            // Bottom Actions Bar
            ui.horizontal(|ui| {
                if proc.is_protected {
                    ui.label(
                        RichText::new("Protected macOS system process cannot be terminated.")
                            .size(11.5)
                            .color(theme.warning_color),
                    );
                } else {
                    if ui.add(theme.critical_button("Force Quit")).clicked() {
                        action = Some(InspectorAction::Quit(
                            proc.clone(),
                            ConfirmAction::ForceQuit,
                        ));
                    }

                    if ui.add(theme.subtle_button("Quit Application")).clicked() {
                        action = Some(InspectorAction::Quit(proc.clone(), ConfirmAction::SafeQuit));
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(theme.subtle_button("Done")).clicked() {
                        action = Some(InspectorAction::Close);
                    }
                });
            });
        });

    action
}
