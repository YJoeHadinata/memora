use std::sync::atomic::Ordering;
use std::time::Duration;

use egui::{vec2, Color32, CornerRadius, RichText, Stroke};

use crate::app::{AppTab, MemoraApp};
use crate::config::ThemePreference;
use crate::ui::components::render_pressure_badge;
use crate::ui::{paint_theme_icon, render_app_logo, AppTheme};
use crate::utils::format::format_bytes;

impl eframe::App for MemoraApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        if self.config.show_menu_bar && ui.ctx().input(|i| i.viewport().close_requested()) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(false));
            self.is_window_visible.store(false, Ordering::Relaxed);
            self.processes.clear();
            self.processes.shrink_to_fit();
            crate::system::macos::purge_heap_memory();
        }

        #[cfg(target_os = "macos")]
        if crate::system::macos::menu_bar::take_show_window_request() {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::Visible(true));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            self.is_window_visible.store(true, Ordering::Relaxed);
        }

        self.check_background_messages();

        #[cfg(target_os = "macos")]
        if !self.is_window_visible.load(Ordering::Relaxed) {
            ui.ctx().request_repaint_after(Duration::from_secs(2));
            return;
        }

        self.update_theme(ui.visuals().dark_mode);
        self.handle_keyboard_shortcuts(ui);
        ui.ctx().request_repaint_after(Duration::from_millis(500));

        let theme = self.theme.clone();
        self.render_top_panel(ui, &theme);
        self.render_bottom_panel(ui, &theme);
        self.render_central_panel(ui, &theme);
        self.render_modals(ui, &theme);
    }
}

impl MemoraApp {
    fn render_top_panel(&mut self, ui: &mut egui::Ui, theme: &AppTheme) {
        let frame = egui::Frame::new()
            .fill(theme.card_bg)
            .stroke(Stroke::new(1.0, theme.border_subtle))
            .inner_margin(egui::Margin::symmetric(16, 10));

        egui::Panel::top("top_header_panel")
            .frame(frame)
            .show(ui, |ui| {
                let avail_w = ui.available_width();
                let (is_compact, is_very_compact) = (avail_w < 720.0, avail_w < 580.0);

                ui.horizontal(|ui| {
                    render_app_logo(ui, 22.0, theme);
                    ui.add_space(6.0);
                    ui.label(
                        RichText::new("Memora")
                            .size(16.0)
                            .strong()
                            .color(theme.text_primary),
                    );
                    ui.add_space(if is_compact { 10.0 } else { 20.0 });

                    let tabs = [
                        (AppTab::Overview, "Overview", "⌘1"),
                        (AppTab::Processes, "Processes", "⌘2"),
                        (AppTab::Cleanup, "Cleanup", "⌘3"),
                        (AppTab::Settings, "Settings", "⌘4"),
                    ];

                    egui::Frame::new()
                        .fill(theme.subtle_bg)
                        .corner_radius(CornerRadius::same(8))
                        .stroke(Stroke::new(1.0, theme.border_subtle))
                        .inner_margin(egui::Margin::symmetric(3, 3))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(2.0, 0.0);
                            for (tab, label, sc) in tabs {
                                let lbl = if is_compact {
                                    label.to_string()
                                } else {
                                    format!("{} ({})", label, sc)
                                };
                                let (bg, fg) = if self.active_tab == tab {
                                    (theme.accent_color, Color32::WHITE)
                                } else {
                                    (Color32::TRANSPARENT, theme.text_secondary)
                                };
                                let btn = egui::Button::new(
                                    RichText::new(lbl).size(11.5).strong().color(fg),
                                )
                                .fill(bg)
                                .corner_radius(CornerRadius::same(6));
                                if ui.add(btn).clicked() {
                                    self.active_tab = tab;
                                }
                            }
                        });

                    ui.add_space(8.0);
                    let (t_rect, t_resp) =
                        ui.allocate_exact_size(vec2(28.0, 24.0), egui::Sense::click());
                    let t_bg = if t_resp.hovered() {
                        theme.card_bg
                    } else {
                        theme.subtle_bg
                    };
                    ui.painter().rect(
                        t_rect,
                        CornerRadius::same(6),
                        t_bg,
                        Stroke::new(1.0, theme.border_subtle),
                        egui::StrokeKind::Inside,
                    );
                    let ic = if t_resp.hovered() {
                        theme.accent_color
                    } else {
                        theme.text_secondary
                    };
                    paint_theme_icon(ui.painter(), t_rect.center(), 7.0, self.config.theme, ic);
                    if t_resp.on_hover_text("Toggle Theme (⌘T)").clicked() {
                        self.config.theme = match self.config.theme {
                            ThemePreference::Dark => ThemePreference::Light,
                            _ => ThemePreference::Dark,
                        };
                        let _ = self.config.save();
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(ref snap) = self.latest_snapshot {
                            render_pressure_badge(ui, snap.pressure, theme);
                            if !is_very_compact {
                                ui.add_space(10.0);
                                let txt = format!(
                                    "{} / {}",
                                    format_bytes(snap.used_bytes),
                                    format_bytes(snap.total_bytes)
                                );
                                ui.label(RichText::new(txt).size(11.5).color(theme.text_secondary));
                            }
                        }
                        if let Some(ref th) = self.latest_thermal {
                            if let Some(temp) = th.primary_temp() {
                                ui.add_space(10.0);
                                let tc = if temp >= 85.0 {
                                    theme.critical_color
                                } else if temp >= 72.0 {
                                    theme.warning_color
                                } else {
                                    theme.normal_color
                                };
                                let h = format!(
                                    "Thermal State: {}\n{}",
                                    th.thermal_state.label(),
                                    th.temp_summary()
                                );
                                ui.label(
                                    RichText::new(format!("{:.1}°C", temp))
                                        .size(11.5)
                                        .strong()
                                        .color(tc),
                                )
                                .on_hover_text(h);
                            }
                        }
                    });
                });
            });
    }
}
