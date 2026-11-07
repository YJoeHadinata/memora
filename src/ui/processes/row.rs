use egui::{pos2, vec2, Align, Color32, CornerRadius, Layout, Rect, RichText, Sense, Stroke, Ui};

use crate::models::{ProcessArchitecture, ProcessInfo};
use crate::ui::helpers::{process_status_color, render_status_dot};
use crate::ui::processes::table::cell_ui;
use crate::ui::processes::{ProcessActionRequest, ProcessViewState};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

#[allow(clippy::too_many_arguments)]
pub(super) fn render_row(
    ui: &mut Ui,
    idx: usize,
    proc: &ProcessInfo,
    cols: &[(f32, f32); 5],
    show_type: bool,
    avail_width: f32,
    state: &mut ProcessViewState,
    action_request: &mut Option<ProcessActionRequest>,
    theme: &AppTheme,
) {
    let is_selected = state.selected_pid == Some(proc.pid);
    let (row_rect, row_resp) = ui.allocate_exact_size(vec2(avail_width, 27.0), Sense::click());
    let ac = theme.accent_color;
    let sb = theme.subtle_bg;

    let row_bg = if is_selected {
        Color32::from_rgba_unmultiplied(ac.r(), ac.g(), ac.b(), 48)
    } else if row_resp.hovered() {
        sb
    } else if idx % 2 == 1 {
        Color32::from_rgba_unmultiplied(sb.r(), sb.g(), sb.b(), 70)
    } else {
        Color32::TRANSPARENT
    };

    if row_bg != Color32::TRANSPARENT {
        ui.painter()
            .rect_filled(row_rect, CornerRadius::same(5), row_bg);
    }
    if is_selected {
        let sc = Color32::from_rgba_unmultiplied(ac.r(), ac.g(), ac.b(), 140);
        ui.painter().rect_stroke(
            row_rect,
            CornerRadius::same(5),
            Stroke::new(1.0, sc),
            egui::StrokeKind::Inside,
        );
    }

    if row_resp.clicked() {
        state.selected_pid = if is_selected { None } else { Some(proc.pid) };
    }
    if row_resp.double_clicked() {
        *action_request = Some(ProcessActionRequest::Inspect(proc.clone()));
    }

    let c_rect = |i: usize| {
        Rect::from_min_size(
            pos2(row_rect.left() + cols[i].0, row_rect.min.y),
            vec2(cols[i].1, 27.0),
        )
    };

    cell_ui(ui, c_rect(0), Layout::left_to_right(Align::Center), |ui| {
        render_status_dot(ui, process_status_color(proc, theme), 3.0);
        ui.add_space(6.0);
        let fg = if is_selected {
            Color32::WHITE
        } else {
            theme.text_primary
        };
        ui.label(RichText::new(&proc.name).size(12.5).strong().color(fg));
    });

    cell_ui(ui, c_rect(1), Layout::right_to_left(Align::Center), |ui| {
        let fg = if is_selected {
            Color32::WHITE
        } else {
            theme.text_secondary
        };
        ui.label(RichText::new(format!("{}", proc.pid)).size(11.5).color(fg));
    });

    cell_ui(ui, c_rect(2), Layout::right_to_left(Align::Center), |ui| {
        let fg = if is_selected { Color32::WHITE } else { ac };
        ui.label(
            RichText::new(format_bytes(proc.memory_bytes))
                .size(12.0)
                .strong()
                .color(fg),
        );
    });

    cell_ui(ui, c_rect(3), Layout::right_to_left(Align::Center), |ui| {
        let cpu_str = proc
            .cpu_usage
            .map_or("-".to_string(), |c| format!("{:.1}%", c));
        let (fg, bold) = match (is_selected, proc.cpu_usage) {
            (true, _) => (Color32::WHITE, false),
            (false, Some(c)) if c >= 50.0 => (theme.critical_color, true),
            (false, Some(c)) if c >= 15.0 => (theme.warning_color, true),
            (false, Some(_)) => (theme.text_primary, false),
            (false, None) => (theme.text_secondary, false),
        };
        let rt = RichText::new(cpu_str).size(11.5).color(fg);
        ui.label(if bold { rt.strong() } else { rt });
    });

    if show_type {
        cell_ui(ui, c_rect(4), Layout::right_to_left(Align::Center), |ui| {
            let fg = if is_selected {
                Color32::WHITE
            } else {
                theme.text_secondary
            };
            let arch = if proc.architecture == ProcessArchitecture::Intel {
                " • Intel"
            } else {
                ""
            };
            ui.label(
                RichText::new(format!("{}{}", proc.display_type(), arch))
                    .size(11.0)
                    .color(fg),
            );
        });
    }
    ui.add_space(1.0);
}
