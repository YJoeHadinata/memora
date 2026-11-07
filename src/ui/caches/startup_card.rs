use egui::{vec2, Align, Color32, Layout, RichText, Ui};

use crate::models::{StartupItem, StartupItemKind, StartupItemStatus};
use crate::ui::caches::{render_badge_pill, CleanupActionRequest};
use crate::ui::theme::AppTheme;

pub fn render_startup_item_card(
    ui: &mut Ui,
    idx: usize,
    item: &StartupItem,
    theme: &AppTheme,
    requested_action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            let (dot_color, status_text, pill_bg, pill_fg) = match item.status {
                StartupItemStatus::Running(pid) => (
                    theme.normal_color,
                    format!("Running (PID {})", pid),
                    Color32::from_rgb(20, 83, 45),
                    Color32::from_rgb(134, 239, 172),
                ),
                StartupItemStatus::Loaded => (
                    theme.accent_color,
                    "Idle / Loaded".to_string(),
                    Color32::from_rgb(30, 58, 138),
                    Color32::from_rgb(147, 197, 253),
                ),
                StartupItemStatus::Disabled => (
                    theme.warning_color,
                    "Disabled".to_string(),
                    Color32::from_rgb(120, 53, 15),
                    Color32::from_rgb(253, 230, 138),
                ),
                StartupItemStatus::NotLoaded => (
                    theme.text_secondary,
                    "Not Loaded".to_string(),
                    Color32::from_rgb(51, 65, 85),
                    Color32::from_rgb(203, 213, 225),
                ),
            };

            let (dot_rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), egui::Sense::hover());
            ui.painter()
                .circle_filled(dot_rect.center(), 4.0, dot_color);
            ui.add_space(4.0);

            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&item.display_name)
                            .size(13.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    render_badge_pill(ui, &status_text, pill_bg, pill_fg);

                    let (kind_bg, kind_fg) = match item.kind {
                        StartupItemKind::UserAgent => (
                            Color32::from_rgb(55, 48, 163),
                            Color32::from_rgb(199, 210, 254),
                        ),
                        StartupItemKind::SystemAgent => (
                            Color32::from_rgb(71, 85, 105),
                            Color32::from_rgb(203, 213, 225),
                        ),
                        StartupItemKind::SystemDaemon => (
                            Color32::from_rgb(88, 28, 135),
                            Color32::from_rgb(216, 180, 254),
                        ),
                    };
                    render_badge_pill(ui, item.kind.label(), kind_bg, kind_fg);
                    if item.run_at_load {
                        render_badge_pill(
                            ui,
                            "Login Item",
                            Color32::from_rgb(30, 41, 59),
                            Color32::from_rgb(148, 163, 184),
                        );
                    }
                });
                ui.add_space(2.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&item.label)
                            .size(10.5)
                            .color(theme.text_secondary),
                    );
                    if let Some(ref prog) = item.program_path {
                        ui.label(RichText::new("•").size(10.5).color(theme.text_secondary));
                        let display_prog = if prog.len() > 60 {
                            format!("...{}", &prog[prog.len() - 57..])
                        } else {
                            prog.clone()
                        };
                        ui.label(
                            RichText::new(display_prog)
                                .size(10.5)
                                .color(theme.text_secondary),
                        );
                    }
                });
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if ui
                    .add(theme.subtle_button(RichText::new("Finder").size(11.0)))
                    .on_hover_text("Reveal plist in Finder")
                    .clicked()
                {
                    *requested_action = Some(CleanupActionRequest::RevealStartupItem(idx));
                }
                if item.kind.is_user_writable() {
                    if ui
                        .add(theme.critical_button(RichText::new("Trash").size(11.0)))
                        .on_hover_text("Disable and move service plist to Trash")
                        .clicked()
                    {
                        *requested_action = Some(CleanupActionRequest::TrashStartupItem(idx));
                    }
                    let toggle_btn = if item.is_active() {
                        theme.subtle_button(RichText::new("Disable").size(11.0))
                    } else {
                        theme.primary_button(RichText::new("Enable").size(11.0))
                    };
                    if ui.add(toggle_btn).clicked() {
                        *requested_action = Some(CleanupActionRequest::ToggleStartupItem(idx));
                    }
                } else {
                    render_badge_pill(
                        ui,
                        "System (Protected)",
                        Color32::from_rgb(51, 65, 85),
                        Color32::from_rgb(148, 163, 184),
                    );
                }
            });
        });
    });
}
