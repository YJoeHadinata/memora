use egui::{pos2, vec2, Align2, Color32, RichText, ScrollArea, Ui, Window};

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
            ui.horizontal(|ui| {
                render_status_dot(ui, process_status_color(proc, theme), 5.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&proc.name)
                            .size(17.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let sub = format!(
                        "Process ID: {} • {} • {}",
                        proc.pid,
                        proc.display_type(),
                        proc.architecture.display_name()
                    );
                    ui.label(RichText::new(sub).size(11.5).color(theme.text_secondary));
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (close_rect, close_resp) =
                        ui.allocate_exact_size(vec2(22.0, 22.0), egui::Sense::click());
                    if close_resp.hovered() {
                        ui.painter()
                            .circle_filled(close_rect.center(), 10.0, theme.subtle_bg);
                    }
                    let x_col = if close_resp.hovered() {
                        theme.text_primary
                    } else {
                        theme.text_secondary
                    };
                    let (r, c, s) = (4.0, close_rect.center(), egui::Stroke::new(1.3, x_col));
                    ui.painter()
                        .line_segment([pos2(c.x - r, c.y - r), pos2(c.x + r, c.y + r)], s);
                    ui.painter()
                        .line_segment([pos2(c.x + r, c.y - r), pos2(c.x - r, c.y + r)], s);
                    if close_resp.clicked() {
                        action = Some(InspectorAction::Close);
                    }
                });
            });

            ui.add_space(12.0);
            ui.separator();
            ui.add_space(10.0);

            ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                render_section_header(ui, "MEMORY CONSUMPTION", theme);
                ui.add_space(6.0);
                let mem_rows = [
                    (
                        "Physical Footprint (Memory):",
                        format_bytes(proc.memory_bytes),
                        true,
                    ),
                    (
                        "Resident Size (RSS):",
                        format_bytes(proc.resident_bytes),
                        false,
                    ),
                    (
                        "Lifetime Peak Footprint:",
                        format_bytes(proc.peak_memory_bytes),
                        false,
                    ),
                    ("Kernel Pageins:", format!("{}", proc.pageins), false),
                ];
                egui::Grid::new("mem_details_grid")
                    .num_columns(2)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        for (lbl, val, bold) in &mem_rows {
                            grid_row(ui, lbl, val, theme.text_primary, *bold, theme);
                        }
                    });

                ui.add_space(14.0);
                render_section_header(ui, "ACTIVITY & I/O", theme);
                ui.add_space(6.0);
                let cpu_s = proc
                    .cpu_usage
                    .map_or("0.0%".into(), |c| format!("{:.1}%", c));
                let arch_s = proc.architecture.display_name().to_string();
                let io_rows = [
                    ("CPU Usage:", cpu_s, theme.accent_color, true),
                    ("Binary Architecture:", arch_s, theme.text_primary, true),
                    (
                        "Disk Bytes Read:",
                        format_bytes(proc.disk_read_bytes),
                        theme.text_primary,
                        false,
                    ),
                    (
                        "Disk Bytes Written:",
                        format_bytes(proc.disk_write_bytes),
                        theme.text_primary,
                        false,
                    ),
                ];
                egui::Grid::new("io_details_grid")
                    .num_columns(2)
                    .spacing([24.0, 8.0])
                    .show(ui, |ui| {
                        for (lbl, val, col, bold) in &io_rows {
                            grid_row(ui, lbl, val, *col, *bold, theme);
                        }
                    });

                ui.add_space(14.0);
                render_section_header(ui, "EXECUTABLE PATH", theme);
                ui.add_space(6.0);

                if let Some(ref path) = proc.path {
                    ui.label(RichText::new(path).size(11.0).color(theme.text_primary));
                    ui.add_space(6.0);
                    if ui.add(theme.subtle_button("Reveal in Finder")).clicked() {
                        action = Some(InspectorAction::RevealInFinder(path.clone()));
                    }
                } else {
                    let msg = "Path not available (system kernel service or sandboxed entry)";
                    ui.label(
                        RichText::new(msg)
                            .size(11.0)
                            .italics()
                            .color(theme.text_secondary),
                    );
                }
            });

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if proc.is_protected {
                    let msg = "Protected macOS system process cannot be terminated.";
                    ui.label(RichText::new(msg).size(11.5).color(theme.warning_color));
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

fn grid_row(
    ui: &mut Ui,
    label: &str,
    value: &str,
    val_color: Color32,
    bold: bool,
    theme: &AppTheme,
) {
    ui.label(RichText::new(label).size(12.0).color(theme.text_secondary));
    let rt = RichText::new(value).size(12.0).color(val_color);
    ui.label(if bold { rt.strong() } else { rt });
    ui.end_row();
}
