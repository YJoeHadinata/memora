mod banner;
mod row;
mod table;

use egui::{vec2, Color32, CornerRadius, Frame, RichText, Stroke, Ui};

use crate::models::{ProcessFilterCategory, ProcessInfo, ProcessSortColumn, SortDirection};
use crate::ui::components::ConfirmAction;
use crate::ui::helpers::{render_search_icon, render_subtle_pill};
use crate::ui::theme::AppTheme;

use self::banner::render_selected_banner;
use self::table::render_process_table;

pub struct ProcessViewState {
    pub search_query: String,
    pub filter_category: ProcessFilterCategory,
    pub sort_column: ProcessSortColumn,
    pub sort_direction: SortDirection,
    pub selected_pid: Option<i32>,
}

impl Default for ProcessViewState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            filter_category: ProcessFilterCategory::All,
            sort_column: ProcessSortColumn::Memory,
            sort_direction: SortDirection::Descending,
            selected_pid: None,
        }
    }
}

pub enum ProcessActionRequest {
    RequestQuit(ProcessInfo, ConfirmAction),
    Inspect(ProcessInfo),
}

pub fn render_processes(
    ui: &mut Ui,
    processes: &[ProcessInfo],
    state: &mut ProcessViewState,
    theme: &AppTheme,
) -> Option<ProcessActionRequest> {
    let mut action_request = None;
    let avail_width = (ui.available_width() - 4.0).max(280.0);

    ui.add_space(4.0);
    render_toolbar(ui, processes.len(), state, avail_width, theme);
    ui.add_space(8.0);
    render_category_chips(ui, state, theme);
    ui.add_space(8.0);
    render_selected_banner(ui, processes, state, &mut action_request, theme);
    render_process_table(
        ui,
        processes,
        state,
        &mut action_request,
        avail_width,
        theme,
    );

    action_request
}

fn render_toolbar(
    ui: &mut Ui,
    total_count: usize,
    state: &mut ProcessViewState,
    avail_width: f32,
    theme: &AppTheme,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Processes")
                .size(20.0)
                .strong()
                .color(theme.text_primary),
        );
        ui.add_space(8.0);
        render_subtle_pill(
            ui,
            &format!("{} total", total_count),
            theme.text_secondary,
            theme,
            11.0,
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let search_w = (avail_width * 0.32).clamp(180.0, 320.0);
            egui::Frame::new()
                .fill(theme.subtle_bg)
                .corner_radius(CornerRadius::same(7))
                .stroke(Stroke::new(1.0, theme.border_subtle))
                .inner_margin(egui::Margin::symmetric(8, 4))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        render_search_icon(ui, theme.text_secondary, 12.0);
                        ui.add_space(4.0);
                        let search_edit = egui::TextEdit::singleline(&mut state.search_query)
                            .id_salt("process_search_input")
                            .hint_text("Search name, PID, path... (⌘F)")
                            .desired_width(search_w - 48.0)
                            .frame(Frame::NONE);
                        ui.add(search_edit);

                        let clr = egui::Button::new(
                            RichText::new("✕").size(10.5).color(theme.text_secondary),
                        )
                        .frame(false);
                        if !state.search_query.is_empty() && ui.add(clr).clicked() {
                            state.search_query.clear();
                        }
                    });
                });
        });
    });
}

fn render_category_chips(ui: &mut Ui, state: &mut ProcessViewState, theme: &AppTheme) {
    let categories = [
        (ProcessFilterCategory::All, "All"),
        (ProcessFilterCategory::Applications, "Applications"),
        (ProcessFilterCategory::Background, "Background"),
        (ProcessFilterCategory::System, "System"),
        (ProcessFilterCategory::HighMemory, "High Memory (>500MB)"),
        (ProcessFilterCategory::HighEnergy, "Energy Hogs (>10% CPU)"),
    ];

    egui::Frame::new()
        .fill(theme.subtle_bg)
        .corner_radius(CornerRadius::same(8))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .inner_margin(egui::Margin::symmetric(3, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                for (cat, label) in categories {
                    let (bg, fg) = if state.filter_category == cat {
                        (theme.accent_color, Color32::WHITE)
                    } else {
                        (Color32::TRANSPARENT, theme.text_secondary)
                    };
                    let btn = egui::Button::new(RichText::new(label).size(11.5).strong().color(fg))
                        .fill(bg)
                        .corner_radius(CornerRadius::same(6));
                    if ui.add(btn).clicked() {
                        state.filter_category = cat;
                    }
                }
            });
        });
}
