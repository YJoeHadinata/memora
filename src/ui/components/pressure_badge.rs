use egui::{vec2, Color32, CornerRadius, Pos2, Stroke, Ui};

use crate::models::MemoryPressure;
use crate::ui::theme::AppTheme;

pub fn render_pressure_badge(ui: &mut Ui, pressure: MemoryPressure, theme: &AppTheme) {
    let color = theme.pressure_color(pressure);
    let label = pressure.label();

    let padding = vec2(10.0, 4.0);
    let text_galley =
        ui.painter()
            .layout_no_wrap(label.to_string(), egui::FontId::proportional(11.5), color);

    let dot_radius = 3.5;
    let dot_spacing = 6.0;
    let desired_size = vec2(
        padding.x * 2.0 + dot_radius * 2.0 + dot_spacing + text_galley.size().x,
        padding.y * 2.0 + text_galley.size().y.max(dot_radius * 2.0),
    );

    let (rect, _response) = ui.allocate_exact_size(desired_size, egui::Sense::hover());

    // Background pill with subtle Apple tint
    let bg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 32);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(12), bg_color);
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(12),
        Stroke::new(
            1.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 85),
        ),
        egui::StrokeKind::Inside,
    );

    // Glowing status dot
    let dot_center = Pos2::new(rect.left() + padding.x + dot_radius, rect.center().y);
    ui.painter().circle_filled(dot_center, dot_radius, color);

    // Status text
    let text_pos = Pos2::new(
        dot_center.x + dot_radius + dot_spacing,
        rect.center().y - text_galley.size().y / 2.0,
    );
    ui.painter().galley(text_pos, text_galley, color);
}
