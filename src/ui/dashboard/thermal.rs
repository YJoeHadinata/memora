use egui::{vec2, CornerRadius, Rect, RichText, Stroke, Ui};

use crate::models::{SensorReading, ThermalPressureState, ThermalSnapshot};
use crate::ui::helpers::render_section_header;
use crate::ui::theme::AppTheme;

pub(super) fn render_thermal_card(ui: &mut Ui, thermal: &ThermalSnapshot, theme: &AppTheme) {
    theme.card_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            render_section_header(ui, "Thermal & Temperature Telemetry", theme);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                render_thermal_badge(ui, thermal.thermal_state, theme);
            });
        });
        ui.add_space(10.0);

        let temp_opt = thermal.primary_temp();
        let temp_val = temp_opt.unwrap_or(0.0);
        let temp_color = if temp_val >= 85.0 {
            theme.critical_color
        } else if temp_val >= 72.0 {
            theme.warning_color
        } else {
            theme.normal_color
        };

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                let temp_str = temp_opt
                    .map(|t| format!("{:.1}°C", t))
                    .unwrap_or_else(|| "N/A".to_string());
                ui.label(
                    RichText::new(temp_str)
                        .size(24.0)
                        .strong()
                        .color(temp_color),
                );
                let sub = format!(
                    "SoC Die Temperature • {}",
                    thermal.thermal_state.description()
                );
                ui.label(RichText::new(sub).size(11.5).color(theme.text_secondary));
            });

            if let Some(st) = thermal.storage_temp {
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(16.0);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(format!("{:.1}°C", st))
                            .size(20.0)
                            .strong()
                            .color(theme.accent_color),
                    );
                    ui.label(
                        RichText::new("SSD Flash Storage")
                            .size(11.5)
                            .color(theme.text_secondary),
                    );
                });
            }
        });

        ui.add_space(10.0);

        // Temperature gauge bar (scale 30°C - 100°C)
        let gauge_pct = ((temp_val - 30.0) / 70.0).clamp(0.0, 1.0);
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), 6.0), egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, CornerRadius::same(3), theme.subtle_bg);
        if gauge_pct > 0.0 {
            let filled =
                Rect::from_min_size(rect.min, vec2(rect.width() * gauge_pct, rect.height()));
            ui.painter()
                .rect_filled(filled, CornerRadius::same(3), temp_color);
        }

        if !thermal.sensors.is_empty() {
            ui.add_space(12.0);
            render_sensor_chips(ui, &thermal.sensors, theme);
        }
    });
}

fn render_sensor_chips(ui: &mut Ui, sensors: &[SensorReading], theme: &AppTheme) {
    ui.label(
        RichText::new(format!("Active Sensors ({})", sensors.len()))
            .size(11.0)
            .color(theme.text_secondary),
    );
    ui.add_space(4.0);

    // Show top core sensors (up to 6) in a compact wrap layout
    let cores: Vec<&SensorReading> = sensors
        .iter()
        .filter(|s| s.label == "CPU/GPU Core")
        .take(6)
        .collect();

    if !cores.is_empty() {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
            for s in cores {
                let col = if s.temp_celsius >= 85.0 {
                    theme.critical_color
                } else if s.temp_celsius >= 72.0 {
                    theme.warning_color
                } else {
                    theme.text_primary
                };
                let text = format!("{}: {:.1}°C", s.name.replace("PMU ", ""), s.temp_celsius);
                let (r, resp) = ui.allocate_exact_size(vec2(105.0, 22.0), egui::Sense::hover());
                ui.painter().rect(
                    r,
                    CornerRadius::same(4),
                    theme.subtle_bg,
                    Stroke::new(1.0, theme.border_subtle),
                    egui::StrokeKind::Inside,
                );
                let tr = Rect::from_min_size(r.min + vec2(6.0, 3.0), r.size() - vec2(12.0, 6.0));
                ui.painter().text(
                    tr.min,
                    egui::Align2::LEFT_TOP,
                    text,
                    egui::FontId::proportional(10.5),
                    col,
                );
                resp.on_hover_text(format!("{} ({}): {:.1}°C", s.name, s.label, s.temp_celsius));
            }
        });
    }
}

fn render_thermal_badge(ui: &mut Ui, state: ThermalPressureState, theme: &AppTheme) {
    let (color, label) = match state {
        ThermalPressureState::Nominal => (theme.normal_color, "Nominal"),
        ThermalPressureState::Fair => (theme.accent_color, "Fair"),
        ThermalPressureState::Serious => (theme.warning_color, "Serious"),
        ThermalPressureState::Critical => (theme.critical_color, "Critical"),
        ThermalPressureState::Unknown => (theme.text_secondary, "Unknown"),
    };

    let (rect, resp) = ui.allocate_exact_size(vec2(72.0, 20.0), egui::Sense::hover());
    let bg = color.gamma_multiply(0.15);
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        bg,
        Stroke::new(1.0, color.gamma_multiply(0.5)),
        egui::StrokeKind::Inside,
    );
    let dot_center = rect.left_center() + vec2(8.0, 0.0);
    ui.painter().circle_filled(dot_center, 3.5, color);
    let text_pos = dot_center + vec2(7.0, -5.5);
    ui.painter().text(
        text_pos,
        egui::Align2::LEFT_TOP,
        label,
        egui::FontId::proportional(10.5),
        color,
    );
    resp.on_hover_text(state.description());
}
