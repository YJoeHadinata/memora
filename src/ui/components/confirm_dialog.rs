use egui::{Align2, RichText, Window};

use crate::ui::components::{ConfirmAction, ConfirmDialogState, DialogResult};
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

struct Spec {
    title: &'static str,
    heading: String,
    body: String,
    note: Option<(String, bool)>,
    critical: bool,
    btn: &'static str,
}

fn spec(title: &'static str, h: &str, body: String, critical: bool, btn: &'static str) -> Spec {
    Spec {
        title,
        heading: h.into(),
        body,
        note: None,
        critical,
        btn,
    }
}

pub fn render_confirm_dialog(
    ctx: &egui::Context,
    state: &ConfirmDialogState,
    theme: &AppTheme,
) -> Option<DialogResult> {
    let mut result = None;
    let s = build_spec(state);
    let dialog_w = (ctx.viewport_rect().width() * 0.9).clamp(320.0, 440.0);

    Window::new(s.title)
        .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
        .collapsible(false)
        .resizable(false)
        .fixed_size([dialog_w, 200.0])
        .frame(theme.modal_frame())
        .show(ctx, |ui| {
            ui.vertical(|ui| {
                let h_color = if s.critical {
                    theme.critical_color
                } else {
                    theme.text_primary
                };
                ui.label(RichText::new(&s.heading).size(16.0).strong().color(h_color));
                ui.add_space(8.0);
                ui.label(
                    RichText::new(&s.body)
                        .size(12.5)
                        .color(theme.text_secondary),
                );

                if let Some((note, is_green)) = &s.note {
                    ui.add_space(4.0);
                    let nc = if *is_green {
                        theme.normal_color
                    } else {
                        theme.text_secondary
                    };
                    ui.label(RichText::new(note).size(11.0).color(nc));
                }

                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let btn = if s.critical {
                            theme.critical_button(s.btn)
                        } else {
                            theme.primary_button(s.btn)
                        };
                        if ui.add(btn).clicked() {
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

fn build_spec(state: &ConfirmDialogState) -> Spec {
    let p_opt = state.process.as_ref();
    let p_name = p_opt.map_or("Process", |p| p.name.as_str());
    let pid = p_opt.map_or(0, |p| p.pid);

    match &state.action {
        ConfirmAction::SafeQuit => {
            let h = format!("Quit \"{}\"?", p_name);
            let b = format!(
                "Send a normal termination request to {} (PID: {}).",
                p_name, pid
            );
            spec("Quit Application", &h, b, false, "Quit")
        }
        ConfirmAction::ForceQuit => {
            let h = format!("Force Quit \"{}\"?", p_name);
            let b = format!(
                "Warning: Force quitting {} (PID: {}) terminates it.",
                p_name, pid
            );
            spec("Force Quit Process", &h, b, true, "Force Quit")
        }
        ConfirmAction::EmptyTrash {
            item_count,
            total_bytes,
        } => {
            let sz = format_bytes(*total_bytes);
            let b = format!(
                "Permanently delete {} items ({}) from Trash?",
                item_count, sz
            );
            let (t, h) = ("Empty Trash", "Permanently Empty Trash?");
            spec(t, h, b, true, "Empty Trash")
        }
        ConfirmAction::DeleteInstallers {
            count, total_bytes, ..
        } => {
            let sz = format_bytes(*total_bytes);
            let b = format!("Delete {} installer packages ({})?", count, sz);
            let (t, h) = ("Delete Installer Packages", "Delete Selected Installers?");
            spec(t, h, b, true, "Delete Files")
        }
        ConfirmAction::DeleteLargeFiles {
            count, total_bytes, ..
        } => {
            let sz = format_bytes(*total_bytes);
            let b = format!("Move {} selected items ({}) to macOS Trash?", count, sz);
            let (t, h) = ("Move to Trash", "Move Selected Files to Trash?");
            spec(t, h, b, true, "Move to Trash")
        }
        ConfirmAction::CleanCaches {
            count, total_bytes, ..
        } => {
            let sz = format_bytes(*total_bytes);
            let b = format!("Clean {} selected cache targets ({})?", count, sz);
            let (t, h) = ("Clean Application Caches", "Clean Application Caches?");
            spec(t, h, b, false, "Clean Caches")
        }
        ConfirmAction::UninstallApp {
            app_name,
            count,
            total_bytes,
            ..
        } => {
            let h = format!("Uninstall \"{}\"?", app_name);
            let sz = format_bytes(*total_bytes);
            let b = format!("Move {} item(s) totaling {} to macOS Trash?", count, sz);
            let mut s = spec("Uninstall Application", &h, b, true, "Uninstall to Trash");
            let note = "Items are placed into Trash and can be restored.";
            s.note = Some((note.into(), true));
            s
        }
        ConfirmAction::TrashStartupItem { label, path } => {
            let b = format!("Move \"{}\" to macOS Trash and unload it?", label);
            let (t, h) = ("Remove Startup Service", "Remove Startup Service?");
            let mut s = spec(t, h, b, true, "Move to Trash");
            s.note = Some((path.display().to_string(), false));
            s
        }
        ConfirmAction::PurgeRam => {
            let b = "Flush inactive file buffers and disk caches back into free memory.".into();
            let (t, h) = ("Purge Inactive RAM", "Purge Inactive RAM Buffers?");
            spec(t, h, b, false, "Purge RAM")
        }
    }
}
