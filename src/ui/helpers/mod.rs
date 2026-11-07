mod icons;

use egui::{pos2, vec2, Color32, CornerRadius, Rect, RichText, Stroke, Ui};

use crate::config::ThemePreference;
use crate::models::ProcessInfo;
use crate::ui::theme::AppTheme;

pub use self::icons::{render_storage_icon, StorageIconKind};

/// Determines the semantic status dot color for a process based on its type.
pub fn process_status_color(proc: &ProcessInfo, theme: &AppTheme) -> Color32 {
    if proc.is_application {
        theme.accent_color
    } else if proc.is_system_process {
        theme.text_secondary
    } else {
        theme.warning_color
    }
}

/// Renders a small circular status indicator dot.
pub fn render_status_dot(ui: &mut Ui, color: Color32, radius: f32) {
    let size = radius * 2.0;
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), radius, color);
}

/// Renders the official Memora squircle logo with its iconic white 'M' waveform symbol.
pub fn render_app_logo(ui: &mut Ui, size: f32, theme: &AppTheme) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let corner_r = (size * 0.27).round().max(2.0);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(corner_r as u8), theme.accent_color);

    let stroke = Stroke::new((size * 0.08).clamp(1.6, 3.5), Color32::WHITE);
    let pad_x = (size * 0.23).round();
    let pad_y = (size * 0.25).round();

    let p1 = pos2(rect.left() + pad_x, rect.bottom() - pad_y);
    let p2 = pos2(rect.left() + (size * 0.38).round(), rect.top() + pad_y);
    let p3 = pos2(rect.center().x, rect.center().y + (size * 0.08).round());
    let p4 = pos2(rect.right() - (size * 0.38).round(), rect.top() + pad_y);
    let p5 = pos2(rect.right() - pad_x, rect.bottom() - pad_y);

    for pair in [p1, p2, p3, p4, p5].windows(2) {
        ui.painter().line_segment([pair[0], pair[1]], stroke);
    }
}

/// Renders a standard uppercase section header (e.g. "MEMORY BREAKDOWN").
pub fn render_section_header(ui: &mut Ui, title: &str, theme: &AppTheme) {
    ui.label(
        RichText::new(title.to_uppercase())
            .size(11.0)
            .strong()
            .color(theme.text_secondary),
    );
}

/// Renders a clean progress bar with filled and track areas.
pub fn render_progress_bar(
    ui: &mut Ui,
    progress: f32,
    bar_height: f32,
    fill_color: Color32,
    track_color: Color32,
    radius: CornerRadius,
) {
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), bar_height), egui::Sense::hover());
    ui.painter().rect_filled(rect, radius, track_color);
    let clamped = progress.clamp(0.0, 1.0);
    if clamped > 0.0 {
        let fill_w = (rect.width() * clamped).max(radius.nw as f32);
        let fill_rect = Rect::from_min_size(rect.min, vec2(fill_w, bar_height));
        ui.painter().rect_filled(fill_rect, radius, fill_color);
    }
}

/// Renders a tinted pill badge with border.
pub fn render_tinted_pill(ui: &mut Ui, text: &str, color: Color32, font_size: f32) {
    let bg = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28);
    let stroke = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70);
    egui::Frame::new()
        .fill(bg)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, stroke))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(font_size).strong().color(color));
        });
}

/// Renders a subtle pill badge with theme subtle background.
pub fn render_subtle_pill(
    ui: &mut Ui,
    text: &str,
    text_color: Color32,
    theme: &AppTheme,
    font_size: f32,
) {
    egui::Frame::new()
        .fill(theme.subtle_bg)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, theme.border_subtle))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(font_size).color(text_color));
        });
}

/// Renders a native vector magnifying glass search icon.
pub fn render_search_icon(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let r = (size * 0.32).floor();
    let center = pos2(rect.min.x + r + 1.0, rect.min.y + r + 1.0);
    ui.painter()
        .circle_stroke(center, r, Stroke::new(1.2, color));
    let start = pos2(center.x + r * 0.707, center.y + r * 0.707);
    let end = pos2(rect.max.x - 1.0, rect.max.y - 1.0);
    ui.painter()
        .line_segment([start, end], Stroke::new(1.4, color));
}

/// Paints a native Apple Appearance icon (Sun, Moon, or System circle).
pub fn paint_theme_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    radius: f32,
    pref: ThemePreference,
    color: Color32,
) {
    match pref {
        ThemePreference::Dark => {
            painter.circle_filled(center, radius * 0.45, color);
            let (ray_in, ray_out) = (radius * 0.65, radius * 0.95);
            let stroke = Stroke::new(1.2, color);
            for i in 0..8 {
                let (sin, cos) = ((i as f32) * std::f32::consts::PI / 4.0).sin_cos();
                let a = pos2(center.x + cos * ray_in, center.y + sin * ray_in);
                let b = pos2(center.x + cos * ray_out, center.y + sin * ray_out);
                painter.line_segment([a, b], stroke);
            }
        }
        ThemePreference::Light => {
            let r = radius * 0.85;
            let pts = vec![
                pos2(center.x, center.y - r),
                pos2(center.x - r * 0.75, center.y - r * 0.3),
                pos2(center.x - r * 0.75, center.y + r * 0.3),
                pos2(center.x, center.y + r),
                pos2(center.x - r * 0.2, center.y + r * 0.5),
                pos2(center.x - r * 0.35, center.y),
                pos2(center.x - r * 0.2, center.y - r * 0.5),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
        ThemePreference::System => {
            let r = radius * 0.8;
            painter.circle_stroke(center, r, Stroke::new(1.2, color));
            let pts = vec![
                pos2(center.x, center.y - r),
                pos2(center.x - r * 0.7, center.y - r * 0.5),
                pos2(center.x - r * 0.7, center.y + r * 0.5),
                pos2(center.x, center.y + r),
            ];
            painter.add(egui::Shape::convex_polygon(pts, color, Stroke::NONE));
        }
    }
}
