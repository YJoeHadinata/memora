use std::path::PathBuf;

use egui::{Align, Color32, Layout, RichText, Ui};

use crate::models::{AgeFilter, LargeFileCategory, SizeFilter};
use crate::ui::caches::{render_badge_pill, CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub(super) fn render_header_card(
    ui: &mut Ui,
    state: &CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
    min_bytes: u64,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Large & Old Files Finder", theme);
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let scan_lbl = if state.is_scanning_large_files {
                    "Scanning Drives & Folders..."
                } else if state.large_files_result.is_none() {
                    "Scan Storage & Volumes"
                } else {
                    "Rescan Storage"
                };
                let btn = theme.primary_button(RichText::new(scan_lbl).size(11.5));
                if ui.add_enabled(!state.is_scanning_large_files, btn).clicked() {
                    *action = Some(CleanupActionRequest::ScanLargeFiles(min_bytes));
                }
                if let Some(ref scan) = state.large_files_result {
                    let sel: Vec<PathBuf> = scan
                        .files
                        .iter()
                        .filter(|f| f.selected)
                        .map(|f| f.path.clone())
                        .collect();
                    if !sel.is_empty() {
                        let lbl = format!(
                            "Move to Trash ({} • {})",
                            sel.len(),
                            format_bytes(scan.total_selected_bytes())
                        );
                        let c_btn = theme.critical_button(RichText::new(lbl).size(11.5));
                        if ui.add_enabled(!state.is_cleaning_large_files, c_btn).clicked() {
                            *action = Some(CleanupActionRequest::DeleteSelectedLargeFiles(sel));
                        }
                    }
                }
            });
        });
        ui.add_space(6.0);
        let desc = "Uncover huge forgotten video exports, disk images, archives, and untouched files taking up space.";
        ui.label(RichText::new(desc).size(12.0).color(theme.text_secondary));

        if let Some(ref scan) = state.large_files_result {
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let tot = format_bytes(scan.total_bytes());
                    ui.label(RichText::new(tot).size(24.0).strong().color(theme.accent_color));
                    let sub = format!("Total Large Files ({} files)", scan.files.len());
                    ui.label(RichText::new(sub).size(11.5).color(theme.text_secondary));
                });
                ui.add_space(32.0);
                let sel_bytes = scan.total_selected_bytes();
                let sel_count = scan.files.iter().filter(|f| f.selected).count();
                ui.vertical(|ui| {
                    let col = if sel_bytes > 0 {
                        theme.critical_color
                    } else {
                        theme.text_secondary
                    };
                    ui.label(RichText::new(format_bytes(sel_bytes)).size(24.0).strong().color(col));
                    let sub = format!("Selected to Trash ({} files)", sel_count);
                    ui.label(RichText::new(sub).size(11.5).color(theme.text_secondary));
                });
            });
        }
    });
}

pub fn render_filter_strip(ui: &mut Ui, state: &mut CleanupViewState, theme: &AppTheme) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Category:")
                .size(11.5)
                .strong()
                .color(theme.text_secondary),
        );
        for cat in [
            LargeFileCategory::All,
            LargeFileCategory::DiskImage,
            LargeFileCategory::Archive,
            LargeFileCategory::Media,
            LargeFileCategory::Document,
            LargeFileCategory::Other,
        ] {
            let btn = if state.large_files_category_filter == cat {
                theme.primary_button(RichText::new(cat.label()).size(11.0).strong())
            } else {
                theme.subtle_button(RichText::new(cat.label()).size(11.0))
            };
            if ui.add(btn).clicked() {
                state.large_files_category_filter = cat;
            }
        }
    });
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Minimum Size:")
                .size(11.5)
                .strong()
                .color(theme.text_secondary),
        );
        for s in [
            SizeFilter::Min50MB,
            SizeFilter::Min100MB,
            SizeFilter::Min500MB,
            SizeFilter::Min1GB,
        ] {
            let btn = if state.large_files_size_filter == s {
                theme.primary_button(RichText::new(s.label()).size(11.0).strong())
            } else {
                theme.subtle_button(RichText::new(s.label()).size(11.0))
            };
            if ui.add(btn).clicked() {
                state.large_files_size_filter = s;
            }
        }
        ui.add_space(16.0);
        ui.label(
            RichText::new("Age:")
                .size(11.5)
                .strong()
                .color(theme.text_secondary),
        );
        for a in [
            AgeFilter::Any,
            AgeFilter::OlderThan1Month,
            AgeFilter::OlderThan6Months,
            AgeFilter::OlderThan1Year,
        ] {
            let btn = if state.large_files_age_filter == a {
                theme.primary_button(RichText::new(a.label()).size(11.0).strong())
            } else {
                theme.subtle_button(RichText::new(a.label()).size(11.0))
            };
            if ui.add(btn).clicked() {
                state.large_files_age_filter = a;
            }
        }
    });
}

pub fn render_category_pill(ui: &mut Ui, cat: LargeFileCategory) {
    let (label, bg, fg) = match cat {
        LargeFileCategory::DiskImage => (
            "Disk Image",
            Color32::from_rgb(30, 58, 138),
            Color32::from_rgb(147, 197, 253),
        ),
        LargeFileCategory::Archive => (
            "Archive",
            Color32::from_rgb(120, 53, 15),
            Color32::from_rgb(253, 230, 138),
        ),
        LargeFileCategory::Media => (
            "Media",
            Color32::from_rgb(88, 28, 135),
            Color32::from_rgb(216, 180, 254),
        ),
        LargeFileCategory::Document => (
            "Document",
            Color32::from_rgb(19, 78, 74),
            Color32::from_rgb(153, 246, 228),
        ),
        _ => (
            "Other",
            Color32::from_rgb(51, 65, 85),
            Color32::from_rgb(203, 213, 225),
        ),
    };
    render_badge_pill(ui, label, bg, fg);
}
