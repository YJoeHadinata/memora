use egui::{Layout, RichText, ScrollArea, Ui};

use crate::config::{AppConfig, MenuBarDisplayMode, ThemePreference};
use crate::ui::components::render_about_card;
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;

pub fn render_settings(ui: &mut Ui, config: &mut AppConfig, theme: &AppTheme) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.set_max_width((ui.available_width() - 4.0).max(280.0));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Settings")
                    .size(22.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.add_space(14.0);
            render_monitoring_card(ui, config, theme);
            ui.add_space(16.0);
            render_alerts_card(ui, config, theme);
            ui.add_space(16.0);
            render_about_card(ui, theme);
            ui.add_space(20.0);
        });
}

fn render_monitoring_card(ui: &mut Ui, config: &mut AppConfig, theme: &AppTheme) {
    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "MONITORING PREFERENCES", theme);
        ui.add_space(14.0);

        let sub = "How frequently Mach kernel stats and processes are refreshed";
        setting_row(ui, "Sampling Interval", sub, theme, |ui| {
            let cur = format!("{} seconds", config.refresh_interval_secs);
            combo(ui, "refresh_interval_combo", &cur, |ui| {
                let opts = [
                    (1, "1 second (Fast)"),
                    (2, "2 seconds (Default)"),
                    (3, "3 seconds"),
                    (5, "5 seconds (Low CPU)"),
                ];
                for (v, l) in opts {
                    ui.selectable_value(&mut config.refresh_interval_secs, v, l);
                }
            });
        });
        row_sep(ui);

        let sub = "Choose light, dark, or follow macOS system appearance";
        setting_row(ui, "Appearance", sub, theme, |ui| {
            let cur = match config.theme {
                ThemePreference::System => "System Default",
                ThemePreference::Dark => "Dark",
                ThemePreference::Light => "Light",
            };
            combo(ui, "theme_preference_combo", cur, |ui| {
                let opts = [
                    (ThemePreference::System, "System Default"),
                    (ThemePreference::Dark, "Dark"),
                    (ThemePreference::Light, "Light"),
                ];
                for (val, lbl) in opts {
                    ui.selectable_value(&mut config.theme, val, lbl);
                }
            });
        });
        row_sep(ui);

        let mins = ((config.history_capacity * config.refresh_interval_secs as usize) / 60).max(1);
        let sub = "Duration of memory and swap activity kept in local RAM buffer";
        setting_row(ui, "History Retention", sub, theme, |ui| {
            let cur = format!("{} minutes", mins);
            combo(ui, "history_retention_combo", &cur, |ui| {
                for (cap, l) in [
                    (90, "3 minutes"),
                    (180, "6 minutes (Default)"),
                    (300, "10 minutes"),
                ] {
                    ui.selectable_value(&mut config.history_capacity, cap, l);
                }
            });
        });
        row_sep(ui);

        let sub = "Show RAM icon, live metrics, and quick actions in the top macOS menu bar";
        setting_row(ui, "macOS Menu Bar Companion", sub, theme, |ui| {
            ui.checkbox(&mut config.show_menu_bar, "");
        });

        if config.show_menu_bar {
            row_sep(ui);
            let sub = "Choose whether to show RAM chip icon, numeric usage, or both";
            setting_row(ui, "Menu Bar Display Style", sub, theme, |ui| {
                let cur = match config.menu_bar_display_mode {
                    MenuBarDisplayMode::IconAndUsage => "RAM Icon & Usage",
                    MenuBarDisplayMode::IconOnly => "RAM Icon Only",
                    MenuBarDisplayMode::UsageOnly => "Usage Value Only",
                };
                combo(ui, "menu_bar_style_combo", cur, |ui| {
                    let opts = [
                        (
                            MenuBarDisplayMode::IconAndUsage,
                            "RAM Icon & Usage (e.g. 9.1 GB)",
                        ),
                        (
                            MenuBarDisplayMode::IconOnly,
                            "RAM Icon Only (Clean Minimal)",
                        ),
                        (
                            MenuBarDisplayMode::UsageOnly,
                            "Usage Value Only (e.g. 9.1 GB)",
                        ),
                    ];
                    for (mode, lbl) in opts {
                        ui.selectable_value(&mut config.menu_bar_display_mode, mode, lbl);
                    }
                });
            });
        }
    });
}

fn render_alerts_card(ui: &mut Ui, config: &mut AppConfig, theme: &AppTheme) {
    theme.card_frame().show(ui, |ui| {
        render_section_header(ui, "SMART ALERTS & NOTIFICATIONS", theme);
        ui.add_space(14.0);

        let sub = "Notify when an application or background process consumes excessive RAM";
        setting_row(ui, "High RAM Spike Notifications", sub, theme, |ui| {
            ui.checkbox(&mut config.notify_on_high_ram, "");
        });
        row_sep(ui);

        let sub = "Trigger warning when a single process memory footprint crosses this limit";
        setting_row(ui, "High RAM Alert Threshold", sub, theme, |ui| {
            let cur = format!("{} GB", config.high_ram_threshold_gb);
            combo(ui, "high_ram_threshold_combo", &cur, |ui| {
                for (gb, l) in [
                    (2, "2 GB"),
                    (3, "3 GB (Default)"),
                    (4, "4 GB"),
                    (6, "6 GB"),
                    (8, "8 GB"),
                ] {
                    ui.selectable_value(&mut config.high_ram_threshold_gb, gb, l);
                }
            });
        });
        row_sep(ui);

        let sub = "Alert when overall macOS memory enters Warning or Critical pressure state";
        setting_row(ui, "System Pressure Warnings", sub, theme, |ui| {
            ui.checkbox(&mut config.notify_on_pressure_warning, "");
        });
    });
}

fn combo(ui: &mut Ui, id: &str, sel: &str, add: impl FnOnce(&mut Ui)) {
    egui::ComboBox::from_id_salt(id)
        .selected_text(sel)
        .show_ui(ui, add);
}

fn setting_row(ui: &mut Ui, title: &str, sub: &str, theme: &AppTheme, ctrl: impl FnOnce(&mut Ui)) {
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.label(
                RichText::new(title)
                    .size(13.0)
                    .strong()
                    .color(theme.text_primary),
            );
            ui.label(RichText::new(sub).size(11.0).color(theme.text_secondary));
        });
        ui.with_layout(Layout::right_to_left(egui::Align::Center), ctrl);
    });
}

fn row_sep(ui: &mut Ui) {
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
}
