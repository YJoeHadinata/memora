use egui::{CornerRadius, Layout, Rect, RichText, Stroke, Ui};

use crate::models::{ProcessInfo, ProcessSortColumn, SortDirection};
use crate::ui::components::ConfirmAction;
use crate::ui::helpers::{
    process_status_color, render_status_dot, render_subtle_pill, render_tinted_pill,
};
use crate::ui::processes::table::cell_ui;
use crate::ui::processes::{ProcessActionRequest, ProcessViewState};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_sort_header(
    ui: &mut Ui,
    cell: Rect,
    layout: Layout,
    label: &str,
    col: ProcessSortColumn,
    default_dir: SortDirection,
    state: &mut ProcessViewState,
    theme: &AppTheme,
) {
    cell_ui(ui, cell, layout, |ui| {
        let is_active = state.sort_column == col;
        let arrow = if is_active {
            match state.sort_direction {
                SortDirection::Ascending => " ▲",
                SortDirection::Descending => " ▼",
            }
        } else {
            ""
        };
        let color = if is_active {
            theme.accent_color
        } else {
            theme.text_secondary
        };
        let btn = egui::Button::new(
            RichText::new(format!("{}{}", label, arrow))
                .size(11.5)
                .strong()
                .color(color),
        )
        .frame(false);
        if ui.add(btn).clicked() {
            if is_active {
                state.sort_direction = match state.sort_direction {
                    SortDirection::Ascending => SortDirection::Descending,
                    SortDirection::Descending => SortDirection::Ascending,
                };
            } else {
                state.sort_column = col;
                state.sort_direction = default_dir;
            }
        }
    });
}

pub(super) fn render_selected_banner(
    ui: &mut Ui,
    processes: &[ProcessInfo],
    state: &mut ProcessViewState,
    action_request: &mut Option<ProcessActionRequest>,
    theme: &AppTheme,
) {
    let Some(proc) = state
        .selected_pid
        .and_then(|pid| processes.iter().find(|p| p.pid == pid))
    else {
        return;
    };

    egui::Frame::new()
        .fill(theme.card_bg)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0, theme.accent_color.linear_multiply(0.6)))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        render_status_dot(ui, process_status_color(proc, theme), 4.0);
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(&proc.name)
                                .size(14.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                        render_subtle_pill(
                            ui,
                            &format!("PID {}", proc.pid),
                            theme.text_secondary,
                            theme,
                            11.0,
                        );
                        render_tinted_pill(
                            ui,
                            &format_bytes(proc.memory_bytes),
                            theme.accent_color,
                            11.0,
                        );
                        if let Some(cpu) = proc.cpu_usage {
                            render_subtle_pill(
                                ui,
                                &format!("{:.1}% CPU", cpu),
                                theme.text_primary,
                                theme,
                                11.0,
                            );
                        }
                        render_subtle_pill(
                            ui,
                            proc.display_type(),
                            theme.text_secondary,
                            theme,
                            10.5,
                        );
                        render_subtle_pill(
                            ui,
                            proc.architecture.display_name(),
                            theme.text_secondary,
                            theme,
                            10.5,
                        );
                    });
                    if let Some(ref path) = proc.path {
                        ui.add_space(3.0);
                        ui.label(
                            RichText::new(format!("Path: {}", path))
                                .size(10.5)
                                .color(theme.text_secondary),
                        );
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close_btn = egui::Button::new(
                        RichText::new("✕").size(11.0).color(theme.text_secondary),
                    )
                    .frame(false);
                    if ui.add(close_btn).clicked() {
                        state.selected_pid = None;
                    }
                    ui.add_space(8.0);
                    if proc.is_protected {
                        render_tinted_pill(
                            ui,
                            "Protected System Process",
                            theme.warning_color,
                            11.0,
                        );
                    } else {
                        if ui
                            .add(theme.critical_button(RichText::new("Force Quit").size(11.5)))
                            .clicked()
                        {
                            *action_request = Some(ProcessActionRequest::RequestQuit(
                                proc.clone(),
                                ConfirmAction::ForceQuit,
                            ));
                        }
                        ui.add_space(4.0);
                        if ui
                            .add(theme.subtle_button(RichText::new("Quit").size(11.5)))
                            .clicked()
                        {
                            *action_request = Some(ProcessActionRequest::RequestQuit(
                                proc.clone(),
                                ConfirmAction::SafeQuit,
                            ));
                        }
                    }
                    ui.add_space(4.0);
                    if ui
                        .add(theme.subtle_button(RichText::new("Inspect Details").size(11.5)))
                        .clicked()
                    {
                        *action_request = Some(ProcessActionRequest::Inspect(proc.clone()));
                    }
                });
            });
        });
    ui.add_space(6.0);
}
