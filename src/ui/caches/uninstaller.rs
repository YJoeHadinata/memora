use egui::{
    vec2, Align, Color32, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui, UiBuilder,
};

use crate::models::InstalledAppItem;
use crate::ui::caches::uninstaller_details::{render_selected_app_details, render_summary_card};
use crate::ui::caches::{render_badge_pill, CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_search_icon;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub fn render_app_uninstaller(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    render_summary_card(ui, state, theme, action);
    ui.add_space(14.0);

    if state.uninstaller_result.is_none() {
        theme.card_frame().show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(30.0);
                if state.is_scanning_uninstaller {
                    ui.spinner();
                    ui.add_space(10.0);
                    let m = "Scanning installed apps and inspecting ~/Library for leftovers...";
                    ui.label(RichText::new(m).size(13.0).color(theme.text_secondary));
                } else {
                    let h = "No Applications Scanned Yet";
                    ui.label(
                        RichText::new(h)
                            .size(16.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.add_space(6.0);
                    let m = "Click 'Scan Installed Apps' above to analyze apps and leftovers.";
                    ui.label(RichText::new(m).size(12.0).color(theme.text_secondary));
                    ui.add_space(16.0);
                    let btn = theme.primary_button(RichText::new("Scan Installed Apps").size(12.5));
                    if ui.add(btn).clicked() {
                        *action = Some(CleanupActionRequest::ScanInstalledApps);
                    }
                }
                ui.add_space(30.0);
            });
        });
        return;
    }

    let q_raw = state.uninstaller_search_query.to_lowercase();
    let query = q_raw.trim().to_string();
    let Some(ref mut result) = state.uninstaller_result else {
        return;
    };

    let filtered: Vec<usize> = result
        .apps
        .iter()
        .enumerate()
        .filter(|(_, a)| {
            query.is_empty()
                || a.name.to_lowercase().contains(&query)
                || a.bundle_id.to_lowercase().contains(&query)
        })
        .map(|(idx, _)| idx)
        .collect();

    let total_len = result.apps.len();
    if state.selected_app_index.is_none_or(|idx| idx >= total_len) {
        state.selected_app_index = filtered.first().copied();
    }

    let left_w = 330.0;
    let right_w = (ui.available_width() - left_w - 16.0).max(320.0);

    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
            theme.card_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    render_search_icon(ui, theme.text_secondary, 13.0);
                    let input_w = (ui.available_width() - 8.0).max(100.0);
                    let edit = egui::TextEdit::singleline(&mut state.uninstaller_search_query)
                        .hint_text("Search apps...")
                        .desired_width(input_w);
                    ui.add(edit);
                });
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                ScrollArea::vertical()
                    .id_salt("uninstaller_apps_scroll")
                    .min_scrolled_height(480.0)
                    .max_height(540.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.add_space(30.0);
                            ui.vertical_centered(|ui| {
                                let m = "No matching apps found.";
                                ui.label(RichText::new(m).size(12.0).color(theme.text_secondary));
                            });
                            ui.add_space(30.0);
                            return;
                        }
                        for &idx in &filtered {
                            let is_sel = state.selected_app_index == Some(idx);
                            if render_app_row(ui, &result.apps[idx], is_sel, theme) {
                                state.selected_app_index = Some(idx);
                            }
                            ui.add_space(3.0);
                        }
                    });
            });
        });

        ui.allocate_ui_with_layout(vec2(right_w, 0.0), Layout::top_down(Align::Min), |ui| {
            if let Some(idx) = state.selected_app_index.filter(|&i| i < result.apps.len()) {
                render_selected_app_details(ui, &mut result.apps[idx], theme, action);
            }
        });
    });
}

fn render_app_row(ui: &mut Ui, app: &InstalledAppItem, is_sel: bool, theme: &AppTheme) -> bool {
    let sz_vec = vec2(ui.available_width(), 52.0);
    let (row_rect, row_resp) = ui.allocate_exact_size(sz_vec, egui::Sense::click());
    let bg = if is_sel {
        theme.accent_color.linear_multiply(0.18)
    } else if row_resp.hovered() {
        theme.subtle_bg
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if is_sel {
        Stroke::new(1.0, theme.accent_color)
    } else {
        Stroke::NONE
    };
    let inside = egui::StrokeKind::Inside;
    ui.painter()
        .rect(row_rect, CornerRadius::same(6), bg, stroke, inside);

    let content_rect = row_rect.shrink2(vec2(10.0, 6.0));
    let bldr = UiBuilder::new()
        .max_rect(content_rect)
        .layout(Layout::left_to_right(Align::Center));
    let mut child_ui = ui.new_child(bldr);

    child_ui.horizontal(|ui| {
        let col_sz = vec2(ui.available_width() - 75.0, content_rect.height());
        ui.allocate_ui_with_layout(col_sz, Layout::top_down(Align::Min), |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(&app.name)
                        .size(12.5)
                        .strong()
                        .color(theme.text_primary),
                );
                if app.is_system_app {
                    let b = Color32::from_rgb(71, 85, 105);
                    let t = Color32::from_rgb(203, 213, 225);
                    render_badge_pill(ui, "System", b, t);
                } else if app.leftover_count() > 0 {
                    let lbl = format!("+{} leftovers", app.leftover_count());
                    let b = Color32::from_rgb(20, 83, 45);
                    let t = Color32::from_rgb(134, 239, 172);
                    render_badge_pill(ui, &lbl, b, t);
                }
            });
            let id_txt = if app.bundle_id.is_empty() {
                &app.name
            } else {
                &app.bundle_id
            };
            ui.label(RichText::new(id_txt).size(10.5).color(theme.text_secondary));
        });
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let sz = format_bytes(app.total_size_bytes());
            ui.label(
                RichText::new(sz)
                    .size(11.5)
                    .strong()
                    .color(theme.text_primary),
            );
        });
    });

    row_resp.clicked()
}
