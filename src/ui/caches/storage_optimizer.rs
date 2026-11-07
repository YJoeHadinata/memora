use std::path::PathBuf;

use egui::{vec2, Align, Layout, RichText, Ui};

use crate::ui::caches::maintenance::{render_simulators_card, render_snapshots_card};
use crate::ui::caches::{CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub fn render_storage_optimizer(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    render_summary_card(ui, state, theme, action);
    ui.add_space(14.0);
    render_trash_card(ui, state, theme, action);
    ui.add_space(14.0);
    render_installers_card(ui, state, theme, action);
    ui.add_space(14.0);
    render_snapshots_card(ui, state, theme, action);
    ui.add_space(14.0);
    render_simulators_card(ui, theme, action);
}

fn render_summary_card(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Storage Reclaim Overview", theme);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let lbl = if state.is_scanning_storage {
                    "Scanning Storage..."
                } else {
                    "Rescan Storage"
                };
                let btn = theme.subtle_button(RichText::new(lbl).size(11.5));
                if ui.add_enabled(!state.is_scanning_storage, btn).clicked() {
                    *action = Some(CleanupActionRequest::ScanStorageJunk);
                }
            });
        });
        ui.add_space(10.0);
        let total = state
            .storage_junk
            .as_ref()
            .map_or(0, |j| j.total_reclaimable_bytes());
        ui.horizontal(|ui| {
            let color = if total > 0 {
                theme.accent_color
            } else {
                theme.text_secondary
            };
            let tot_str = format_bytes(total);
            stat_block(
                ui,
                &tot_str,
                "Total Readily Reclaimable Space",
                24.0,
                color,
                theme,
            );

            if let Some(ref junk) = state.storage_junk {
                let inst_b: u64 = junk.installers.iter().map(|i| i.size_bytes).sum();
                let t_lbl = format!("Trash ({} items)", junk.trash.item_count);
                let i_lbl = format!("Installers ({} files)", junk.installers.len());
                let blocks = [
                    (format_bytes(junk.trash.total_bytes), t_lbl),
                    (format_bytes(inst_b), i_lbl),
                    (
                        format!("{}", junk.apfs_snapshots.len()),
                        "Local APFS Snapshots".into(),
                    ),
                ];
                for (val, lbl) in blocks {
                    ui.add_space(28.0);
                    stat_block(ui, &val, &lbl, 18.0, theme.text_primary, theme);
                }
            }
        });
    });
}

fn stat_block(ui: &mut Ui, val: &str, lbl: &str, sz: f32, col: egui::Color32, theme: &AppTheme) {
    ui.vertical(|ui| {
        ui.label(RichText::new(val).size(sz).strong().color(col));
        ui.label(RichText::new(lbl).size(11.0).color(theme.text_secondary));
    });
}

fn render_trash_card(
    ui: &mut Ui,
    state: &CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        let left_w = (ui.available_width() - 136.0).max(180.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "macOS Trash (~/.Trash)", theme);
                let msg = match state.storage_junk {
                    Some(ref j) if j.trash.item_count > 0 => format!(
                        "{} across {} items ready to be permanently deleted.",
                        format_bytes(j.trash.total_bytes),
                        j.trash.item_count
                    ),
                    Some(_) => "The macOS Trash is completely empty.".to_string(),
                    None => "Checking Trash status...".to_string(),
                };
                ui.add(
                    egui::Label::new(RichText::new(msg).size(12.0).color(theme.text_secondary))
                        .wrap(),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let has_items = state
                    .storage_junk
                    .as_ref()
                    .is_some_and(|j| j.trash.item_count > 0);
                let btn = theme.critical_button(RichText::new("Empty Trash").size(11.5));
                if ui.add_enabled(has_items, btn).clicked() {
                    *action = Some(CleanupActionRequest::EmptyTrash);
                }
            });
        });
    });
}

fn render_installers_card(
    ui: &mut Ui,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        let sel: Vec<PathBuf> = state
            .storage_junk
            .as_ref()
            .map(|j| j.installers.iter().filter(|i| i.selected).map(|i| i.path.clone()).collect())
            .unwrap_or_default();

        let left_w = (ui.available_width() - 206.0).max(180.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "Leftover Installers & Disk Images", theme);
                let sub = "Downloaded .dmg, .pkg, and .iso disk packages across user folders and volumes.";
                ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(theme.text_secondary)).wrap());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = theme.critical_button(RichText::new("Delete Selected Installers").size(11.5));
                if ui.add_enabled(!sel.is_empty(), btn).clicked() {
                    *action = Some(CleanupActionRequest::DeleteSelectedInstallers(sel));
                }
            });
        });
        ui.add_space(10.0);
        if let Some(ref mut junk) = state.storage_junk {
            if junk.installers.is_empty() {
                let ok_msg = "✓ No leftover installer packages found in user folders or volumes.";
                ui.label(RichText::new(ok_msg).size(12.0).color(theme.normal_color));
            } else {
                ui.horizontal(|ui| {
                    if ui.add(theme.subtle_button("Select All")).clicked() {
                        junk.installers.iter_mut().for_each(|i| i.selected = true);
                    }
                    if ui.add(theme.subtle_button("Deselect All")).clicked() {
                        junk.installers.iter_mut().for_each(|i| i.selected = false);
                    }
                });
                ui.add_space(8.0);
                for item in &mut junk.installers {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut item.selected, "");
                        ui.label(RichText::new(&item.name).size(12.5).strong().color(theme.text_primary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let sz = format_bytes(item.size_bytes);
                            ui.label(RichText::new(sz).size(12.0).strong().color(theme.text_secondary));
                            let age = format!("{}d ago", item.modified_days_ago);
                            ui.label(RichText::new(age).size(11.0).color(theme.text_secondary));
                        });
                    });
                    ui.add_space(4.0);
                }
            }
        } else {
            let msg = "Scanning user folders and volumes for installers...";
            ui.label(RichText::new(msg).size(12.0).color(theme.text_secondary));
        }
    });
}
