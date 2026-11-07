use egui::{vec2, Align, Layout, RichText, Ui};

use crate::models::MemorySnapshot;
use crate::ui::caches::{CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub fn render_ram_and_maintenance(
    ui: &mut Ui,
    snapshot: Option<&MemorySnapshot>,
    state: &mut CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "System RAM File Cache", theme);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let lbl = if state.is_purging_ram {
                    "Purging RAM..."
                } else {
                    "Purge RAM Cache"
                };
                let btn = theme.primary_button(RichText::new(lbl).size(11.5));
                if ui.add_enabled(!state.is_purging_ram, btn).clicked() {
                    *action = Some(CleanupActionRequest::PurgeRamCache);
                }
            });
        });

        ui.add_space(10.0);
        if let Some(snap) = snapshot {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(28.0, 10.0);
                for (val, label, col) in [
                    (snap.cached_files_bytes, "Cached Files in RAM", theme.text_primary),
                    (snap.purgeable_bytes, "Immediately Purgeable", theme.normal_color),
                    (snap.free_bytes, "Completely Free RAM", theme.text_secondary),
                ] {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(format_bytes(val)).size(23.0).strong().color(col));
                        ui.label(RichText::new(label).size(11.0).color(theme.text_secondary));
                    });
                }
            });
        }

        ui.add_space(12.0);
        let note = "How macOS handles RAM cache: macOS retains closed files in RAM as 'Cached Files' so reopening them is instantaneous. Under memory pressure, macOS purges them automatically. Purging forces the XNU kernel to flush inactive buffers back to free memory.";
        ui.label(RichText::new(note).size(11.0).color(theme.text_secondary));
    });

    ui.add_space(16.0);

    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "Quick System Diagnostics", theme);
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui
                .add(theme.subtle_button(RichText::new("Flush DNS Cache").size(11.5)))
                .clicked()
            {
                *action = Some(CleanupActionRequest::FlushDns);
            }
            ui.add_space(8.0);
            if ui
                .add(theme.subtle_button(RichText::new("Reset QuickLook Thumbnails").size(11.5)))
                .clicked()
            {
                *action = Some(CleanupActionRequest::FlushQuickLook);
            }
        });
    });
}

pub(super) fn render_snapshots_card(
    ui: &mut Ui,
    state: &CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        let count = state
            .storage_junk
            .as_ref()
            .map_or(0, |j| j.apfs_snapshots.len());
        let left_w = (ui.available_width() - 196.0).max(180.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "APFS Local Snapshots (Time Machine)", theme);
                let sub = "Time Machine retains local snapshots on APFS drives when backup targets are disconnected.";
                ui.add(
                    egui::Label::new(RichText::new(sub).size(12.0).color(theme.text_secondary))
                        .wrap(),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = theme.primary_button(RichText::new("Purge Local Snapshots").size(11.5));
                if ui.add_enabled(count > 0, btn).clicked() {
                    *action = Some(CleanupActionRequest::PurgeApfsSnapshots);
                }
            });
        });
        ui.add_space(8.0);
        let (msg, col) = if count == 0 {
            (
                "✓ No orphaned APFS local snapshots found on root drive.".to_string(),
                theme.normal_color,
            )
        } else {
            (
                format!(
                    "Found {} local APFS snapshot(s). Purging them recovers disk headroom.",
                    count
                ),
                theme.warning_color,
            )
        };
        ui.label(RichText::new(msg).size(12.0).color(col));
    });
}

pub(super) fn render_simulators_card(
    ui: &mut Ui,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        let left_w = (ui.available_width() - 226.0).max(180.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "Developer Simulator Runtimes", theme);
                let sub = "Removes unavailable iOS/watchOS/tvOS simulator runtimes left behind by Xcode updates.";
                ui.add(
                    egui::Label::new(RichText::new(sub).size(12.0).color(theme.text_secondary))
                        .wrap(),
                );
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let btn = theme.subtle_button(RichText::new("Prune Unavailable Simulators").size(11.5));
                if ui.add(btn).clicked() {
                    *action = Some(CleanupActionRequest::PruneUnavailableSimulators);
                }
            });
        });
    });
}
