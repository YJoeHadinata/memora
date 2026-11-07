use egui::{
    pos2, vec2, Align, CornerRadius, Layout, Rect, RichText, ScrollArea, Sense, Stroke, Ui,
    UiBuilder,
};

use crate::models::{ProcessFilterCategory, ProcessInfo, ProcessSortColumn, SortDirection};
use crate::ui::processes::banner::render_sort_header;
use crate::ui::processes::row::render_row;
use crate::ui::processes::{ProcessActionRequest, ProcessViewState};
use crate::ui::theme::AppTheme;

pub(super) fn cell_ui<R>(
    ui: &mut Ui,
    rect: Rect,
    layout: Layout,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let mut child = ui.new_child(UiBuilder::new().max_rect(rect).layout(layout));
    add_contents(&mut child)
}

pub(super) fn render_process_table(
    ui: &mut Ui,
    processes: &[ProcessInfo],
    state: &mut ProcessViewState,
    action_request: &mut Option<ProcessActionRequest>,
    avail_width: f32,
    theme: &AppTheme,
) {
    let show_type = avail_width >= 620.0;
    let (pad_x, col_spacing) = (12.0, 14.0);
    let (pid_w, mem_w, cpu_w) = (68.0, 105.0, 72.0);
    let type_w = if show_type { 95.0 } else { 0.0 };
    let num_spacings = if show_type { 4.0 } else { 3.0 };
    let fixed_total = pid_w + mem_w + cpu_w + type_w + (col_spacing * num_spacings) + (pad_x * 2.0);
    let name_w = (avail_width - fixed_total).max(160.0);

    let name_x = pad_x;
    let pid_x = name_x + name_w + col_spacing;
    let mem_x = pid_x + pid_w + col_spacing;
    let cpu_x = mem_x + mem_w + col_spacing;
    let type_x = cpu_x + cpu_w + col_spacing;

    let (header_rect, _) = ui.allocate_exact_size(vec2(avail_width, 28.0), Sense::hover());
    ui.painter().rect(
        header_rect,
        CornerRadius::same(6),
        theme.card_bg,
        Stroke::new(1.0, theme.border_subtle),
        egui::StrokeKind::Inside,
    );

    let h_cell = |x: f32, w: f32| {
        Rect::from_min_size(
            pos2(header_rect.left() + x, header_rect.min.y),
            vec2(w, 28.0),
        )
    };
    let l_align = Layout::left_to_right(Align::Center);
    let r_align = Layout::right_to_left(Align::Center);

    let headers = [
        (
            h_cell(name_x, name_w),
            l_align,
            "Process Name",
            ProcessSortColumn::Name,
            SortDirection::Ascending,
        ),
        (
            h_cell(pid_x, pid_w),
            r_align,
            "PID",
            ProcessSortColumn::Pid,
            SortDirection::Ascending,
        ),
        (
            h_cell(mem_x, mem_w),
            r_align,
            "Memory",
            ProcessSortColumn::Memory,
            SortDirection::Descending,
        ),
        (
            h_cell(cpu_x, cpu_w),
            r_align,
            "CPU %",
            ProcessSortColumn::Cpu,
            SortDirection::Descending,
        ),
    ];
    for (cell, layout, label, col, dir) in headers {
        render_sort_header(ui, cell, layout, label, col, dir, state, theme);
    }

    if show_type {
        cell_ui(ui, h_cell(type_x, type_w), r_align, |ui| {
            ui.label(
                RichText::new("Type")
                    .size(11.5)
                    .strong()
                    .color(theme.text_secondary),
            );
        });
    }

    ui.add_space(4.0);

    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_min_width(avail_width);
            if processes.is_empty() {
                ui.add_space(32.0);
                theme.card_frame().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new("No matching processes found")
                                .size(14.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new("Try adjusting your search query or filter category.")
                                .size(11.5)
                                .color(theme.text_secondary),
                        );
                        ui.add_space(12.0);
                        if ui
                            .add(theme.subtle_button(RichText::new("Clear Filters").size(11.5)))
                            .clicked()
                        {
                            state.search_query.clear();
                            state.filter_category = ProcessFilterCategory::All;
                        }
                        ui.add_space(16.0);
                    });
                });
                return;
            }

            let cols = [
                (name_x, name_w),
                (pid_x, pid_w),
                (mem_x, mem_w),
                (cpu_x, cpu_w),
                (type_x, type_w),
            ];
            for (idx, proc) in processes.iter().enumerate() {
                render_row(
                    ui,
                    idx,
                    proc,
                    &cols,
                    show_type,
                    avail_width,
                    state,
                    action_request,
                    theme,
                );
            }
        });
}
