pub mod confirm_dialog;
pub mod metric_card;
pub mod pressure_badge;
pub mod process_inspector;

use std::path::PathBuf;

use egui::{vec2, RichText, Ui};

use crate::models::ProcessInfo;
use crate::ui::helpers::{
    render_app_logo, render_section_header, render_subtle_pill, render_tinted_pill,
};
use crate::ui::theme::AppTheme;

pub use confirm_dialog::render_confirm_dialog;
pub use metric_card::MetricCard;
pub use pressure_badge::render_pressure_badge;
pub use process_inspector::{render_process_inspector, InspectorAction};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmAction {
    SafeQuit,
    ForceQuit,
    EmptyTrash {
        item_count: usize,
        total_bytes: u64,
    },
    DeleteInstallers {
        count: usize,
        total_bytes: u64,
        paths: Vec<PathBuf>,
    },
    DeleteLargeFiles {
        count: usize,
        total_bytes: u64,
        paths: Vec<PathBuf>,
    },
    CleanCaches {
        count: usize,
        total_bytes: u64,
        paths: Vec<PathBuf>,
    },
    UninstallApp {
        app_name: String,
        count: usize,
        total_bytes: u64,
        paths: Vec<PathBuf>,
    },
    TrashStartupItem {
        label: String,
        path: PathBuf,
    },
    PurgeRam,
}

#[derive(Debug, Clone)]
pub struct ConfirmDialogState {
    pub process: Option<ProcessInfo>,
    pub action: ConfirmAction,
}

pub enum DialogResult {
    Confirmed(ConfirmAction),
    Cancelled,
}

/// Renders the About Memora information card used in Settings.
pub fn render_about_card(ui: &mut Ui, theme: &AppTheme) {
    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "ABOUT MEMORA", theme);
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            render_app_logo(ui, 38.0, theme);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Memora")
                            .size(17.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let ver = format!("v{}", env!("CARGO_PKG_VERSION"));
                    render_tinted_pill(ui, &ver, theme.accent_color, 10.5);
                });
                let tag = "Understand Your Mac. Keep It Healthy.";
                ui.label(RichText::new(tag).size(12.0).color(theme.text_secondary));
            });
        });
        ui.add_space(14.0);
        let desc = "Memora is a privacy-first macOS system and memory monitor built in Rust with egui.\n\
                    It reads directly from native macOS XNU kernel interfaces (Mach host_statistics64, sysctl, and libproc) \
                    to provide authentic, transparent system health metrics identical to Activity Monitor.\n\n\
                    Memora does not use gimmicks or aggressive cache purges. \
                    macOS virtual memory management operates best when inactive buffers remain available until needed.";
        ui.label(RichText::new(desc).size(11.5).color(theme.text_secondary));
        ui.add_space(14.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
            for (txt, col) in [
                ("License: MIT", theme.text_secondary),
                ("Platform: Apple Silicon (ARM64)", theme.accent_color),
                ("100% Offline & Private", theme.normal_color),
            ] {
                render_subtle_pill(ui, txt, col, theme, 11.0);
            }
        });
    });
}
