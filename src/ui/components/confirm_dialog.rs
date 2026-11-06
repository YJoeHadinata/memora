use std::path::PathBuf;

use egui::{Align2, RichText, Window};

use crate::models::ProcessInfo;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

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

pub fn render_confirm_dialog(
    ctx: &egui::Context,
    state: &ConfirmDialogState,
    theme: &AppTheme,
) -> Option<DialogResult> {
    let mut result = None;

    let title = match &state.action {
        ConfirmAction::SafeQuit => "Quit Application",
        ConfirmAction::ForceQuit => "Force Quit Process",
        ConfirmAction::EmptyTrash { .. } => "Empty Trash",
        ConfirmAction::DeleteInstallers { .. } => "Delete Installer Packages",
        ConfirmAction::DeleteLargeFiles { .. } => "Move to Trash",
        ConfirmAction::CleanCaches { .. } => "Clean Application Caches",
        ConfirmAction::PurgeRam => "Purge Inactive RAM",
    };

    let dialog_w = (ctx.viewport_rect().width() * 0.9).clamp(320.0, 440.0);

    Window::new(title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .fixed_size([dialog_w, 200.0])
        .frame(theme.modal_frame())
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                match &state.action {
                    ConfirmAction::SafeQuit => {
                        let name = state
                            .process
                            .as_ref()
                            .map(|p| p.name.as_str())
                            .unwrap_or("Process");
                        let pid = state.process.as_ref().map(|p| p.pid).unwrap_or(0);

                        ui.label(
                            RichText::new(format!("Quit \"{}\"?", name))
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "This will send a normal termination request to {} (PID: {}).",
                                name, pid
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::ForceQuit => {
                        let name = state
                            .process
                            .as_ref()
                            .map(|p| p.name.as_str())
                            .unwrap_or("Process");
                        let pid = state.process.as_ref().map(|p| p.pid).unwrap_or(0);

                        ui.label(
                            RichText::new(format!("Force Quit \"{}\"?", name))
                                .size(16.0)
                                .strong()
                                .color(theme.critical_color),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Warning: Force quitting {} (PID: {}) will immediately terminate it. Any unsaved data or changes will be lost.",
                                name, pid
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::EmptyTrash {
                        item_count,
                        total_bytes,
                    } => {
                        ui.label(
                            RichText::new("Permanently Empty Trash?")
                                .size(16.0)
                                .strong()
                                .color(theme.critical_color),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Are you sure you want to permanently delete {} items ({}) from the Trash? This operation cannot be undone.",
                                item_count,
                                format_bytes(*total_bytes)
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::DeleteInstallers {
                        count, total_bytes, ..
                    } => {
                        ui.label(
                            RichText::new("Delete Selected Installers?")
                                .size(16.0)
                                .strong()
                                .color(theme.critical_color),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Are you sure you want to delete {} installer packages ({}) from ~/Downloads? Download them again later if needed.",
                                count,
                                format_bytes(*total_bytes)
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::DeleteLargeFiles {
                        count, total_bytes, ..
                    } => {
                        ui.label(
                            RichText::new("Move Selected Files to Trash?")
                                .size(16.0)
                                .strong()
                                .color(theme.critical_color),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Move {} selected items ({}) to macOS Trash? You can restore them from the Trash later if needed.",
                                count,
                                format_bytes(*total_bytes)
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::CleanCaches {
                        count, total_bytes, ..
                    } => {
                        ui.label(
                            RichText::new("Clean Application Caches?")
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(format!(
                                "Clean {} selected cache targets ({})? The apps will regenerate their cache files automatically as needed.",
                                count,
                                format_bytes(*total_bytes)
                            ))
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                    ConfirmAction::PurgeRam => {
                        ui.label(
                            RichText::new("Purge Inactive RAM Buffers?")
                                .size(16.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(
                                "This commands the macOS XNU kernel to flush inactive file buffers and disk caches back into free memory.",
                            )
                            .size(12.5)
                            .color(theme.text_secondary),
                        );
                    }
                }

                ui.add_space(20.0);

                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let confirm_btn = match &state.action {
                            ConfirmAction::SafeQuit => theme.primary_button("Quit"),
                            ConfirmAction::ForceQuit => theme.critical_button("Force Quit"),
                            ConfirmAction::EmptyTrash { .. } => {
                                theme.critical_button("Empty Trash")
                            }
                            ConfirmAction::DeleteInstallers { .. } => {
                                theme.critical_button("Delete Files")
                            }
                            ConfirmAction::DeleteLargeFiles { .. } => {
                                theme.critical_button("Move to Trash")
                            }
                            ConfirmAction::CleanCaches { .. } => {
                                theme.primary_button("Clean Caches")
                            }
                            ConfirmAction::PurgeRam => theme.primary_button("Purge RAM"),
                        };

                        if ui.add(confirm_btn).clicked() {
                            result = Some(DialogResult::Confirmed(state.action.clone()));
                        }

                        if ui.add(theme.subtle_button("Cancel")).clicked() {
                            result = Some(DialogResult::Cancelled);
                        }
                    });
                });
            });
        });

    result
}
