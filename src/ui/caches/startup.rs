use egui::{Align, CornerRadius, Layout, RichText, ScrollArea, Stroke, Ui};

use crate::models::StartupItemKind;
use crate::ui::caches::startup_card::render_startup_item_card;
use crate::ui::caches::{CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_search_icon;
use crate::ui::theme::AppTheme;

pub fn render_startup_manager(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    requested_action: &mut Option<CleanupActionRequest>,
) {
    render_header_card(ui, state, theme, requested_action);
    ui.add_space(12.0);
    render_filter_bar(ui, state, theme);
    ui.add_space(10.0);

    let Some(ref result) = state.startup_result else {
        ui.add_space(30.0);
        ui.vertical_centered(|ui| {
            let msg = if state.is_scanning_startup {
                ui.spinner();
                ui.add_space(8.0);
                "Analyzing background services and launch agents..."
            } else {
                "No startup services scanned yet."
            };
            ui.label(RichText::new(msg).size(13.0).color(theme.text_secondary));
        });
        return;
    };

    let query = state.startup_search_query.to_lowercase();
    let filtered_indices: Vec<usize> = result
        .items
        .iter()
        .enumerate()
        .filter(|(_, item)| {
            if state.startup_running_only && !item.status.is_running() {
                return false;
            }
            if state
                .startup_filter_kind
                .as_ref()
                .is_some_and(|k| &item.kind != k)
            {
                return false;
            }
            query.is_empty()
                || item.display_name.to_lowercase().contains(&query)
                || item.label.to_lowercase().contains(&query)
                || item
                    .program_path
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&query)
        })
        .map(|(idx, _)| idx)
        .collect();

    if filtered_indices.is_empty() {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("No background services matching filters.")
                    .size(13.0)
                    .color(theme.text_secondary),
            );
        });
        return;
    }

    ScrollArea::vertical()
        .id_salt("startup_items_scroll")
        .max_height(480.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for &idx in &filtered_indices {
                render_startup_item_card(ui, idx, &result.items[idx], theme, requested_action);
                ui.add_space(4.0);
            }
        });
}

fn render_header_card(
    ui: &mut Ui,
    state: &CleanupViewState,
    theme: &AppTheme,
    requested_action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label(RichText::new("STARTUP & BACKGROUND ITEMS").size(11.0).strong().color(theme.text_secondary));
                let sub = "Manage login agents, background daemons, and disable unneeded background services.";
                ui.label(RichText::new(sub).size(12.0).color(theme.text_secondary));
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if state.is_scanning_startup {
                    ui.spinner();
                    ui.label(RichText::new("Scanning services...").size(12.0).color(theme.text_secondary));
                } else if ui.add(theme.primary_button(RichText::new("Rescan Services").size(12.0))).clicked() {
                    *requested_action = Some(CleanupActionRequest::ScanStartupItems);
                }
            });
        });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if let Some(ref res) = state.startup_result {
                let metrics = [
                    (res.items.len(), "Total Services", theme.text_primary),
                    (res.running_count(), "Active Daemons", theme.normal_color),
                    (res.user_agents_count(), "User Agents", theme.accent_color),
                    (res.disabled_count(), "Disabled Items", theme.warning_color),
                ];
                for (i, (val, label, color)) in metrics.into_iter().enumerate() {
                    if i > 0 {
                        ui.add_space(28.0);
                    }
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format!("{}", val)).size(18.0).strong().color(color));
                        ui.label(RichText::new(label).size(11.0).color(theme.text_secondary));
                    });
                }
            } else {
                let msg = "Click \"Rescan Services\" or visit tab to load startup items.";
                ui.label(RichText::new(msg).size(12.0).color(theme.text_secondary));
            }
        });
    });
}

fn render_filter_bar(ui: &mut Ui, state: &mut CleanupViewState, theme: &AppTheme) {
    ui.horizontal(|ui| {
        egui::Frame::new()
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .corner_radius(CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(8, 4))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    render_search_icon(ui, theme.text_secondary, 13.0);
                    ui.add_space(4.0);
                    let edit = egui::TextEdit::singleline(&mut state.startup_search_query)
                        .hint_text("Search services by name, label, or path...")
                        .desired_width(260.0);
                    ui.add(edit);
                });
            });
        ui.add_space(10.0);

        let all_active = state.startup_filter_kind.is_none() && !state.startup_running_only;
        if ui.add(chip_btn("All", all_active, theme)).clicked() {
            state.startup_filter_kind = None;
            state.startup_running_only = false;
        }
        if ui
            .add(chip_btn("Running Only", state.startup_running_only, theme))
            .clicked()
        {
            state.startup_running_only = !state.startup_running_only;
        }
        let user_active = state.startup_filter_kind == Some(StartupItemKind::UserAgent)
            && !state.startup_running_only;
        if ui
            .add(chip_btn("User Agents", user_active, theme))
            .clicked()
        {
            state.startup_filter_kind = (!user_active).then_some(StartupItemKind::UserAgent);
            state.startup_running_only = false;
        }
        let sys_active = state.startup_filter_kind == Some(StartupItemKind::SystemDaemon)
            && !state.startup_running_only;
        if ui
            .add(chip_btn("System Daemons", sys_active, theme))
            .clicked()
        {
            state.startup_filter_kind = (!sys_active).then_some(StartupItemKind::SystemDaemon);
            state.startup_running_only = false;
        }
    });
}

fn chip_btn(label: &str, active: bool, theme: &AppTheme) -> egui::Button<'static> {
    let txt = RichText::new(label.to_string()).size(11.5);
    if active {
        theme.primary_button(txt)
    } else {
        theme.subtle_button(txt)
    }
}
