use egui::{vec2, Layout, RichText, ScrollArea, Ui};

use crate::config::{AppConfig, MenuBarDisplayMode, ThemePreference};
use crate::ui::helpers::{
    render_app_logo, render_section_header, render_subtle_pill, render_tinted_pill,
};
use crate::ui::theme::AppTheme;

pub fn render_settings(ui: &mut Ui, config: &mut AppConfig, theme: &AppTheme) {
    ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail_width = (ui.available_width() - 4.0).max(280.0);
            ui.set_max_width(avail_width);
            ui.add_space(4.0);

            ui.label(
                RichText::new("Settings")
                    .size(22.0)
                    .strong()
                    .color(theme.text_primary),
            );

            ui.add_space(14.0);

            // General Preferences Card (macOS System Settings Group)
            theme.card_frame().show(ui, |ui| {
                render_section_header(ui, "MONITORING PREFERENCES", theme);
                ui.add_space(14.0);

                    // Row 1: Refresh Interval
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Sampling Interval")
                                    .size(13.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new("How frequently Mach kernel stats and processes are refreshed")
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("refresh_interval_combo")
                                .selected_text(format!("{} seconds", config.refresh_interval_secs))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut config.refresh_interval_secs, 1, "1 second (Fast)");
                                    ui.selectable_value(&mut config.refresh_interval_secs, 2, "2 seconds (Default)");
                                    ui.selectable_value(&mut config.refresh_interval_secs, 3, "3 seconds");
                                    ui.selectable_value(&mut config.refresh_interval_secs, 5, "5 seconds (Low CPU)");
                                });
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Row 2: Theme preference
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("Appearance")
                                    .size(13.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new("Choose light, dark, or follow macOS system appearance")
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("theme_preference_combo")
                                .selected_text(match config.theme {
                                    ThemePreference::System => "System Default",
                                    ThemePreference::Dark => "Dark",
                                    ThemePreference::Light => "Light",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(&mut config.theme, ThemePreference::System, "System Default");
                                    ui.selectable_value(&mut config.theme, ThemePreference::Dark, "Dark");
                                    ui.selectable_value(&mut config.theme, ThemePreference::Light, "Light");
                                });
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Row 3: History retention
                    ui.horizontal(|ui| {
                        let mut minutes = (config.history_capacity * config.refresh_interval_secs as usize) / 60;
                        if minutes == 0 {
                            minutes = 1;
                        }

                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("History Retention")
                                    .size(13.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new("Duration of memory and swap activity kept in local RAM buffer")
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            egui::ComboBox::from_id_salt("history_retention_combo")
                                .selected_text(format!("{} minutes", minutes))
                                .show_ui(ui, |ui| {
                                    if ui.selectable_label(config.history_capacity == 90, "3 minutes").clicked() {
                                        config.history_capacity = 90;
                                    }
                                    if ui.selectable_label(config.history_capacity == 180, "6 minutes (Default)").clicked() {
                                        config.history_capacity = 180;
                                    }
                                    if ui.selectable_label(config.history_capacity == 300, "10 minutes").clicked() {
                                        config.history_capacity = 300;
                                    }
                                });
                        });
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    // Row 4: Menu Bar Companion
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new("macOS Menu Bar Companion")
                                    .size(13.0)
                                    .strong()
                                    .color(theme.text_primary),
                            );
                            ui.label(
                                RichText::new("Show RAM icon, live metrics, and quick actions in the top macOS menu bar")
                                    .size(11.0)
                                    .color(theme.text_secondary),
                            );
                        });

                        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut config.show_menu_bar, "");
                        });
                    });

                    if config.show_menu_bar {
                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(10.0);

                        // Row 5: Menu Bar Display Style
                        ui.horizontal(|ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new("Menu Bar Display Style")
                                        .size(13.0)
                                        .strong()
                                        .color(theme.text_primary),
                                );
                                ui.label(
                                    RichText::new("Choose whether to show RAM chip icon, numeric usage, or both")
                                        .size(11.0)
                                        .color(theme.text_secondary),
                                );
                            });

                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                egui::ComboBox::from_id_salt("menu_bar_style_combo")
                                    .selected_text(match config.menu_bar_display_mode {
                                        MenuBarDisplayMode::IconAndUsage => "RAM Icon & Usage",
                                        MenuBarDisplayMode::IconOnly => "RAM Icon Only",
                                        MenuBarDisplayMode::UsageOnly => "Usage Value Only",
                                    })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(
                                            &mut config.menu_bar_display_mode,
                                            MenuBarDisplayMode::IconAndUsage,
                                            "RAM Icon & Usage (e.g. 💾 9.1 GB)",
                                        );
                                        ui.selectable_value(
                                            &mut config.menu_bar_display_mode,
                                            MenuBarDisplayMode::IconOnly,
                                            "RAM Icon Only (Clean Minimal)",
                                        );
                                        ui.selectable_value(
                                            &mut config.menu_bar_display_mode,
                                            MenuBarDisplayMode::UsageOnly,
                                            "Usage Value Only (e.g. 9.1 GB)",
                                        );
                                    });
                            });
                        });
                    }
                });

            ui.add_space(16.0);

            // Smart Alerts & Notifications Card
            theme.card_frame().show(ui, |ui| {
                render_section_header(ui, "SMART ALERTS & NOTIFICATIONS", theme);
                ui.add_space(14.0);

                // Row 1: High RAM Usage Alert
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("High RAM Spike Notifications")
                                .size(13.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new("Notify when an application or background process consumes excessive RAM")
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut config.notify_on_high_ram, "");
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Row 2: Threshold Dropdown
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("High RAM Alert Threshold")
                                .size(13.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new("Trigger warning when a single process memory footprint crosses this limit")
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        egui::ComboBox::from_id_salt("high_ram_threshold_combo")
                            .selected_text(format!("{} GB", config.high_ram_threshold_gb))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut config.high_ram_threshold_gb, 2, "2 GB");
                                ui.selectable_value(&mut config.high_ram_threshold_gb, 3, "3 GB (Default)");
                                ui.selectable_value(&mut config.high_ram_threshold_gb, 4, "4 GB");
                                ui.selectable_value(&mut config.high_ram_threshold_gb, 6, "6 GB");
                                ui.selectable_value(&mut config.high_ram_threshold_gb, 8, "8 GB");
                            });
                    });
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Row 3: Memory Pressure Warning
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new("System Pressure Warnings")
                                .size(13.0)
                                .strong()
                                .color(theme.text_primary),
                        );
                        ui.label(
                            RichText::new("Alert when overall macOS memory enters Warning or Critical pressure state")
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                    });

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.checkbox(&mut config.notify_on_pressure_warning, "");
                    });
                });
            });

            ui.add_space(16.0);

            // About Memora Card (Apple HIG About Panel)
            theme.card_frame().show(ui, |ui| {
                render_section_header(ui, "ABOUT MEMORA", theme);
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    // Official App Logo
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

                            // Version pill badge
                            render_tinted_pill(ui, "v0.1.0", theme.accent_color, 10.5);
                        });

                        ui.label(
                            RichText::new("Understand Your Mac. Keep It Healthy.")
                                .size(12.0)
                                .color(theme.text_secondary),
                        );
                    });
                });

                ui.add_space(14.0);

                ui.label(
                    RichText::new(
                        "Memora is a privacy-first macOS system and memory monitor built in Rust with egui.\n\
                         It reads directly from native macOS XNU kernel interfaces (Mach host_statistics64, sysctl, and libproc) \
                         to provide authentic, transparent system health metrics identical to Activity Monitor.\n\n\
                         Memora does not use gimmicks or aggressive cache purges. \
                         macOS virtual memory management operates best when inactive buffers remain available until needed."
                    )
                    .size(11.5)
                    .color(theme.text_secondary),
                );

                ui.add_space(14.0);

                // Metadata Badges Row
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(8.0, 6.0);

                    let badges = [
                        ("License: MIT", theme.text_secondary),
                        ("Platform: Apple Silicon (ARM64)", theme.accent_color),
                        ("100% Offline & Private", theme.normal_color),
                    ];

                    for (text, color) in badges {
                        render_subtle_pill(ui, text, color, theme, 11.0);
                    }
                });
            });

            ui.add_space(20.0);
        });
}
