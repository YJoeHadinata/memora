use egui::{CornerRadius, RichText, Ui};

use crate::models::{DiskVolume, StorageSnapshot};
use crate::ui::helpers::{
    render_progress_bar, render_section_header, render_storage_icon, render_tinted_pill,
    StorageIconKind,
};
use crate::ui::theme::AppTheme;
use crate::utils::format::{format_bytes, format_percent};

pub(super) fn render_storage_volumes_card(
    ui: &mut Ui,
    storage: &StorageSnapshot,
    theme: &AppTheme,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (status_title, _) = storage.swap_headroom_status();
                let badge_color = match status_title {
                    "Healthy" => theme.normal_color,
                    "Moderate" => theme.warning_color,
                    _ => theme.critical_color,
                };
                render_tinted_pill(
                    ui,
                    &format!("Swap Headroom: {}", status_title),
                    badge_color,
                    10.5,
                );
                ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                    let hdr = format!("STORAGE VOLUMES ({} MOUNTED)", storage.volumes.len());
                    render_section_header(ui, &hdr, theme);
                });
            });
        });

        ui.add_space(6.0);
        let desc = format!(
            "{} • Sisa ruang SSD tersedia untuk swap memori: {}",
            storage.swap_headroom_status().1,
            format_bytes(storage.swap_headroom_bytes)
        );
        ui.label(RichText::new(desc).size(11.0).color(theme.text_secondary));
        ui.add_space(14.0);

        let user_vols = storage.user_volumes();
        let helper_vols = storage.helper_volumes();
        for (idx, vol) in user_vols.iter().enumerate() {
            if idx > 0 {
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);
            }
            render_volume_row(ui, vol, theme);
        }

        if !helper_vols.is_empty() {
            let id = ui.make_persistent_id("show_system_helper_volumes");
            let mut show = ui.data_mut(|d| *d.get_temp_mut_or_default::<bool>(id));
            ui.add_space(10.0);
            let action_word = if show { "Hide" } else { "Show" };
            let lbl = format!(
                "{} Internal System Volumes ({} hidden)",
                action_word,
                helper_vols.len()
            );
            if ui
                .add(theme.subtle_button(RichText::new(lbl).size(11.0)))
                .clicked()
            {
                show = !show;
                ui.data_mut(|d| d.insert_temp(id, show));
            }
            if show {
                ui.add_space(8.0);
                for (idx, vol) in helper_vols.iter().enumerate() {
                    if idx > 0 || !user_vols.is_empty() {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                    }
                    render_volume_row(ui, vol, theme);
                }
            }
        }
    });
}

fn render_volume_row(ui: &mut Ui, vol: &DiskVolume, theme: &AppTheme) {
    let used_pct = vol.used_percent();
    let bar_color = if used_pct > 88.0 {
        theme.critical_color
    } else if used_pct > 75.0 {
        theme.warning_color
    } else {
        theme.accent_color
    };
    let icon_kind = match vol.mount_point.as_str() {
        _ if vol.is_removable => StorageIconKind::ExternalDrive,
        "/System/Volumes/Data" => StorageIconKind::InternalData,
        "/" => StorageIconKind::SystemSnapshot,
        "/System/Volumes/VM" => StorageIconKind::SwapVolume,
        _ => StorageIconKind::StandardVolume,
    };

    ui.horizontal(|ui| {
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let stats = format!(
                "{} / {} ({})",
                format_bytes(vol.used_bytes),
                format_bytes(vol.total_bytes),
                format_percent(used_pct)
            );
            ui.label(
                RichText::new(stats)
                    .size(12.0)
                    .strong()
                    .color(theme.text_primary),
            );
            if ui.available_width() > 80.0 {
                let free_str = format!("{} free •", format_bytes(vol.available_bytes));
                ui.label(
                    RichText::new(free_str)
                        .size(11.0)
                        .color(theme.text_secondary),
                );
            }
            ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                render_storage_icon(ui, icon_kind, theme.accent_color, 16.0);
                ui.add_space(6.0);
                ui.label(
                    RichText::new(&vol.name)
                        .size(13.0)
                        .strong()
                        .color(theme.text_primary),
                );
            });
        });
    });
    ui.add_space(6.0);
    render_progress_bar(
        ui,
        used_pct / 100.0,
        6.0,
        bar_color,
        theme.border_subtle,
        CornerRadius::same(3),
    );
}
