use std::path::PathBuf;

use egui::{Align, Layout, RichText, Ui};

use crate::ui::caches::{CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::{render_search_icon, render_section_header};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub fn render_application_caches(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    let scan_result = state.cache_scan_result.as_mut();

    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Application & Developer Caches on Disk", theme);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan_txt = if state.is_scanning_caches {
                    "Scanning..."
                } else {
                    "Scan Caches"
                };
                if ui
                    .add_enabled(
                        !state.is_scanning_caches,
                        theme.subtle_button(RichText::new(scan_txt).size(11.5)),
                    )
                    .clicked()
                {
                    *action = Some(CleanupActionRequest::ScanCaches);
                }

                if let Some(ref scan) = scan_result {
                    let sel_cnt = scan.selected_count();
                    let sel_bytes = scan.selected_total_bytes();
                    let clean_txt = if state.is_cleaning_caches {
                        "Cleaning...".to_string()
                    } else if sel_cnt > 0 {
                        format!("Clean {} ({})", sel_cnt, format_bytes(sel_bytes))
                    } else {
                        "Clean Selected".to_string()
                    };
                    let btn = theme.critical_button(RichText::new(clean_txt).size(11.5));
                    if ui
                        .add_enabled(sel_cnt > 0 && !state.is_cleaning_caches, btn)
                        .clicked()
                    {
                        let paths: Vec<PathBuf> = scan
                            .items
                            .iter()
                            .filter(|i| i.selected)
                            .map(|i| i.path.clone())
                            .collect();
                        *action = Some(CleanupActionRequest::CleanSelectedCaches(paths));
                    }
                }
            });
        });

        ui.add_space(10.0);

        let Some(scan) = scan_result else {
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("Application caches have not been scanned yet.")
                        .size(12.0)
                        .color(theme.text_secondary),
                );
                ui.add_space(8.0);
                if ui.add(theme.primary_button("Scan Caches Now")).clicked() {
                    *action = Some(CleanupActionRequest::ScanCaches);
                }
                ui.add_space(20.0);
            });
            return;
        };

        ui.horizontal(|ui| {
            render_search_icon(ui, theme.text_secondary, 13.0);
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut state.cache_search_query)
                    .hint_text("Filter caches by name...")
                    .desired_width(220.0),
            );
            ui.add_space(12.0);
            if ui.add(theme.subtle_button("Select Safe")).clicked() {
                scan.select_safe();
            }
            if ui.add(theme.subtle_button("Select All")).clicked() {
                scan.select_all();
            }
            if ui.add(theme.subtle_button("Clear Selection")).clicked() {
                scan.deselect_all();
            }
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let tot = format!("Total disk cache found: {}", format_bytes(scan.total_bytes));
                ui.label(RichText::new(tot).size(12.0).color(theme.text_secondary));
            });
        });

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(28.0);
            ui.label(
                RichText::new("Application")
                    .size(11.0)
                    .strong()
                    .color(theme.text_secondary),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Safety")
                        .size(11.0)
                        .strong()
                        .color(theme.text_secondary),
                );
                ui.add_space(32.0);
                ui.label(
                    RichText::new("Disk Size")
                        .size(11.0)
                        .strong()
                        .color(theme.text_secondary),
                );
            });
        });
        ui.separator();
        ui.add_space(4.0);

        let query = state.cache_search_query.to_lowercase();
        let mut displayed_count = 0;

        for item in &mut scan.items {
            if !query.is_empty()
                && !item.name.to_lowercase().contains(&query)
                && !item.id.to_lowercase().contains(&query)
            {
                continue;
            }
            displayed_count += 1;

            ui.horizontal(|ui| {
                ui.checkbox(&mut item.selected, "");
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&item.name)
                            .size(12.5)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.label(
                        RichText::new(&item.description)
                            .size(10.5)
                            .color(theme.text_secondary),
                    );
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    ui.add_space(8.0);
                    let (lbl, col) = if item.is_safe {
                        ("Safe to Clean", theme.normal_color)
                    } else {
                        ("Review Needed", theme.warning_color)
                    };
                    ui.label(RichText::new(lbl).size(10.5).color(col));
                    ui.add_space(20.0);
                    ui.label(
                        RichText::new(format_bytes(item.size_bytes))
                            .size(12.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                });
            });
            ui.add_space(6.0);
        }

        if displayed_count == 0 {
            ui.add_space(12.0);
            ui.label(
                RichText::new("No caches match your current search query.")
                    .size(12.0)
                    .color(theme.text_secondary),
            );
        }
    });
}
