use egui::{Align, Layout, RichText, Ui};

use crate::models::LargeFileCategory;
use crate::ui::caches::large_files_filters::{
    render_category_pill, render_filter_strip, render_header_card,
};
use crate::ui::caches::{CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_search_icon;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub fn render_large_files_finder(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    let min_bytes = state.large_files_size_filter.min_bytes();
    render_header_card(ui, state, theme, action, min_bytes);
    ui.add_space(14.0);

    if state.large_files_result.is_some() {
        theme.card_frame().show(ui, |ui| {
            let (mut select_filtered, mut clear_selection) = (false, false);
            ui.horizontal(|ui| {
                render_search_icon(ui, theme.text_secondary, 13.0);
                ui.add_space(4.0);
                let edit = egui::TextEdit::singleline(&mut state.large_files_search_query)
                    .hint_text("Filter files by name...")
                    .desired_width(220.0);
                ui.add(edit);
                ui.add_space(12.0);
                if ui.add(theme.subtle_button("Select All Filtered")).clicked() {
                    select_filtered = true;
                }
                if ui.add(theme.subtle_button("Clear Selection")).clicked() {
                    clear_selection = true;
                }
            });

            ui.add_space(10.0);
            render_filter_strip(ui, state, theme);
            ui.add_space(14.0);

            let q = state.large_files_search_query.to_lowercase();
            let cat = state.large_files_category_filter;
            let age_min = state.large_files_age_filter.min_days();
            let size_min = state.large_files_size_filter.min_bytes();

            let Some(ref mut scan) = state.large_files_result else {
                return;
            };

            if select_filtered {
                for f in &mut scan.files {
                    if (q.is_empty() || f.name.to_lowercase().contains(&q))
                        && (cat == LargeFileCategory::All || f.category == cat)
                        && f.modified_days_ago >= age_min
                        && f.size_bytes >= size_min
                    {
                        f.selected = true;
                    }
                }
            }
            if clear_selection {
                scan.files.iter_mut().for_each(|f| f.selected = false);
            }

            let home_str = std::env::var_os("HOME").and_then(|h| h.into_string().ok());
            let mut visible_count = 0;

            for file in &mut scan.files {
                let matches = (q.is_empty() || file.name.to_lowercase().contains(&q))
                    && (cat == LargeFileCategory::All || file.category == cat)
                    && file.modified_days_ago >= age_min
                    && file.size_bytes >= size_min;
                if !matches {
                    continue;
                }
                visible_count += 1;

                ui.horizontal(|ui| {
                    ui.checkbox(&mut file.selected, "");
                    render_category_pill(ui, file.category);
                    ui.add_space(4.0);

                    let parent = file
                        .path
                        .parent()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let compact_parent = home_str
                        .as_ref()
                        .map_or(parent.clone(), |h| parent.replace(h, "~"));

                    ui.vertical(|ui| {
                        let name_lbl = RichText::new(&file.name)
                            .size(12.5)
                            .strong()
                            .color(theme.text_primary);
                        ui.label(name_lbl)
                            .on_hover_text(file.path.to_string_lossy());
                        let sub = format!(
                            "{} • Modified {}d ago",
                            compact_parent, file.modified_days_ago
                        );
                        ui.label(RichText::new(sub).size(10.5).color(theme.text_secondary));
                    });

                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let t_btn = theme.subtle_button(
                            RichText::new("Trash")
                                .size(11.0)
                                .color(theme.critical_color),
                        );
                        if ui
                            .add(t_btn)
                            .on_hover_text("Move this file to Trash")
                            .clicked()
                        {
                            *action =
                                Some(CleanupActionRequest::DeleteSelectedLargeFiles(vec![file
                                    .path
                                    .clone()]));
                        }
                        ui.add_space(6.0);
                        let f_btn = theme.subtle_button(RichText::new("Finder").size(11.0));
                        if ui
                            .add(f_btn)
                            .on_hover_text("Reveal file in macOS Finder")
                            .clicked()
                        {
                            *action = Some(CleanupActionRequest::RevealLargeFileInFinder(
                                file.path.clone(),
                            ));
                        }
                        ui.add_space(10.0);
                        let sz = format_bytes(file.size_bytes);
                        ui.label(
                            RichText::new(sz)
                                .size(12.5)
                                .strong()
                                .color(theme.text_primary),
                        );
                    });
                });
                ui.add_space(6.0);
            }

            if visible_count == 0 {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No files match the active filters.")
                            .size(12.0)
                            .color(theme.text_secondary),
                    );
                });
                ui.add_space(10.0);
            }
        });
    } else if !state.is_scanning_large_files {
        theme.card_frame().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(RichText::new("No Scan Results Yet").size(16.0).strong().color(theme.text_primary));
                ui.add_space(6.0);
                let msg = "Click 'Scan for Large Files' to scan Downloads, Documents, Movies, and Desktop for files over 50 MB.";
                ui.label(RichText::new(msg).size(12.0).color(theme.text_secondary));
                ui.add_space(12.0);
                let btn = theme.primary_button(RichText::new("Scan for Large Files").size(12.0));
                if ui.add(btn).clicked() {
                    *action = Some(CleanupActionRequest::ScanLargeFiles(min_bytes));
                }
                ui.add_space(20.0);
            });
        });
    }
}
