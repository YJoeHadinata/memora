use egui::{vec2, Align, Layout, RichText, ScrollArea, Ui};

use crate::models::InstalledAppItem;
use crate::ui::caches::{render_leftover_kind_pill, CleanupActionRequest, CleanupViewState};
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;
use crate::utils::format::format_bytes;

pub(super) fn render_summary_card(
    ui: &mut Ui,
    state: &CleanupViewState,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        let left_w = (ui.available_width() - 196.0).max(180.0);
        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(vec2(left_w, 0.0), Layout::top_down(Align::Min), |ui| {
                render_section_header(ui, "App Uninstaller & Residuals Cleaner", theme);
                let sub = "Locates installed applications and scrubs hidden caches, preferences, saved state, and container leftovers.";
                ui.add(egui::Label::new(RichText::new(sub).size(12.0).color(theme.text_secondary)).wrap());
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let lbl = if state.is_scanning_uninstaller {
                    "Scanning Apps..."
                } else if state.uninstaller_result.is_none() {
                    "Scan Installed Apps"
                } else {
                    "Rescan Apps"
                };
                let btn = theme.primary_button(RichText::new(lbl).size(11.5));
                if ui.add_enabled(!state.is_scanning_uninstaller, btn).clicked() {
                    *action = Some(CleanupActionRequest::ScanInstalledApps);
                }
            });
        });

        if let Some(ref res) = state.uninstaller_result {
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                let apps = format!("{}", res.total_apps());
                let sz = format_bytes(res.total_reclaimable_bytes());
                let stats = [
                    (apps, "Applications Detected"),
                    (sz, "Reclaimable App & Residual Space"),
                ];
                for (val, lbl) in stats {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(val).size(18.0).strong().color(theme.text_primary));
                        ui.label(RichText::new(lbl).size(11.0).color(theme.text_secondary));
                    });
                    ui.add_space(32.0);
                }
            });
        }
    });
}

pub fn render_selected_app_details(
    ui: &mut Ui,
    app: &mut InstalledAppItem,
    theme: &AppTheme,
    action: &mut Option<CleanupActionRequest>,
) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(&app.name)
                            .size(17.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let v = format!("v{}", app.version);
                    ui.label(RichText::new(v).size(12.0).color(theme.text_secondary));
                });
                let b_id = format!("Bundle: {}", app.bundle_id);
                ui.label(RichText::new(b_id).size(11.0).color(theme.text_secondary));
                let p_str = app.app_path.display().to_string();
                ui.label(RichText::new(p_str).size(10.5).color(theme.text_secondary));
            });

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.vertical(|ui| {
                    let tot = format_bytes(app.total_size_bytes());
                    ui.label(
                        RichText::new(tot)
                            .size(18.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    let sub = format!(
                        "{} app • {} residuals",
                        format_bytes(app.app_size_bytes),
                        format_bytes(app.leftover_size_bytes())
                    );
                    ui.label(RichText::new(sub).size(10.5).color(theme.text_secondary));
                });
            });
        });

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            if ui.add(theme.subtle_button("Select All")).clicked() {
                app.leftovers.iter_mut().for_each(|i| i.selected = true);
            }
            if ui.add(theme.subtle_button("Deselect All")).clicked() {
                app.leftovers.iter_mut().for_each(|i| i.selected = false);
            }
            if ui.add(theme.subtle_button("Reveal in Finder")).clicked() {
                *action = Some(CleanupActionRequest::RevealAppPathInFinder(
                    app.app_path.clone(),
                ));
            }

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let selected_bytes = app.selected_size_bytes();
                let selected_paths = app.selected_paths();
                let can_uninstall = !app.is_system_app && !selected_paths.is_empty();

                let btn_text = if app.is_system_app {
                    "System App Protected".to_string()
                } else if selected_bytes > 0 {
                    format!("Uninstall to Trash ({})", format_bytes(selected_bytes))
                } else {
                    "Select Items to Uninstall".to_string()
                };

                let btn = theme.critical_button(RichText::new(btn_text).size(12.0));
                if ui.add_enabled(can_uninstall, btn).clicked() {
                    let req = CleanupActionRequest::UninstallAppSelected {
                        app_name: app.name.clone(),
                        paths: selected_paths,
                    };
                    *action = Some(req);
                }
            });
        });

        ui.add_space(12.0);
        let title = "Application Files & Residuals to Remove:";
        ui.label(
            RichText::new(title)
                .size(12.5)
                .strong()
                .color(theme.text_primary),
        );
        ui.add_space(6.0);

        ScrollArea::vertical()
            .id_salt("uninstaller_leftovers_scroll")
            .min_scrolled_height(200.0)
            .max_height(380.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for item in &mut app.leftovers {
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut item.selected, "");
                        render_leftover_kind_pill(ui, &item.kind);
                        let p_str = item.path.display().to_string();
                        ui.label(RichText::new(p_str).size(11.0).color(theme.text_secondary));

                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            let sz = format_bytes(item.size_bytes);
                            ui.label(
                                RichText::new(sz)
                                    .size(11.5)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            let f_btn = theme.subtle_button(RichText::new("Finder").size(10.0));
                            if ui
                                .add(f_btn)
                                .on_hover_text("Reveal this item in Finder")
                                .clicked()
                            {
                                let p = item.path.clone();
                                *action = Some(CleanupActionRequest::RevealAppPathInFinder(p));
                            }
                        });
                    });
                    ui.add_space(4.0);
                }
            });
    });
}
