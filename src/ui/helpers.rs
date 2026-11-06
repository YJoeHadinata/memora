use egui::{pos2, vec2, Color32, CornerRadius, Rect, RichText, Stroke, Ui};

use crate::config::ThemePreference;
use crate::models::ProcessInfo;
use crate::ui::theme::AppTheme;

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

    let stroke_w = (size * 0.08).clamp(1.6, 3.5);
    let stroke = Stroke::new(stroke_w, Color32::WHITE);

    let pad_x = (size * 0.23).round();
    let pad_top = (size * 0.25).round();
    let pad_bot = (size * 0.25).round();

    let p1 = pos2(rect.left() + pad_x, rect.bottom() - pad_bot);
    let p2 = pos2(rect.left() + (size * 0.38).round(), rect.top() + pad_top);
    let p3 = pos2(rect.center().x, rect.center().y + (size * 0.08).round());
    let p4 = pos2(rect.right() - (size * 0.38).round(), rect.top() + pad_top);
    let p5 = pos2(rect.right() - pad_x, rect.bottom() - pad_bot);

    ui.painter().line_segment([p1, p2], stroke);
    ui.painter().line_segment([p2, p3], stroke);
    ui.painter().line_segment([p3, p4], stroke);
    ui.painter().line_segment([p4, p5], stroke);
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
    progress: f32, // 0.0 to 1.0
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
        let min_w = radius.nw as f32;
        let fill_width = (rect.width() * clamped).max(min_w);
        let fill_rect = Rect::from_min_size(rect.min, vec2(fill_width, bar_height));
        ui.painter().rect_filled(fill_rect, radius, fill_color);
    }
}

/// Renders a tinted pill badge with border (e.g. for Swap Headroom status).
pub fn render_tinted_pill(ui: &mut Ui, text: &str, color: Color32, font_size: f32) {
    let bg_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 28);
    let stroke_color = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 70);

    egui::Frame::new()
        .fill(bg_color)
        .corner_radius(CornerRadius::same(6))
        .stroke(Stroke::new(1.0, stroke_color))
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(font_size).strong().color(color));
        });
}

/// Renders a subtle pill badge with theme subtle background (e.g. version or metadata tags).
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

/// Native macOS volume and drive categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageIconKind {
    InternalData,
    SystemSnapshot,
    SwapVolume,
    ExternalDrive,
    StandardVolume,
}

/// Renders a native macOS vector storage/volume icon without consumer emojis.
pub fn render_storage_icon(ui: &mut Ui, kind: StorageIconKind, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let stroke = Stroke::new(1.1, color);

    match kind {
        StorageIconKind::InternalData => {
            // Sleek Apple internal drive symbol
            let h = (size * 0.72).floor();
            let w = (size * 0.95).floor();
            let drive_rect = Rect::from_center_size(rect.center(), vec2(w, h));
            painter.rect(
                drive_rect,
                CornerRadius::same(2),
                color.linear_multiply(0.12),
                stroke,
                egui::StrokeKind::Inside,
            );
            // Platter line
            let line_y = (drive_rect.center().y + h * 0.1).floor();
            painter.line_segment(
                [
                    pos2(drive_rect.min.x + 2.0, line_y),
                    pos2(drive_rect.max.x - 2.0, line_y),
                ],
                Stroke::new(1.0, color.linear_multiply(0.6)),
            );
            // Power / Activity LED dot
            let led_pos = pos2(drive_rect.max.x - 3.5, drive_rect.max.y - 2.8);
            painter.circle_filled(led_pos, 1.2, color);
        }
        StorageIconKind::SystemSnapshot => {
            // macOS System Lock / Shield symbol
            let body_w = (size * 0.72).floor();
            let body_h = (size * 0.52).floor();
            let body_rect = Rect::from_min_size(
                pos2(
                    rect.center().x - body_w * 0.5,
                    rect.center().y - body_h * 0.1,
                ),
                vec2(body_w, body_h),
            );
            painter.rect(
                body_rect,
                CornerRadius::same(2),
                color.linear_multiply(0.15),
                stroke,
                egui::StrokeKind::Inside,
            );
            // Shackle loop
            let shackle_r = (body_w * 0.32).floor();
            let pts = [
                pos2(rect.center().x - shackle_r, body_rect.min.y),
                pos2(rect.center().x - shackle_r, body_rect.min.y - shackle_r),
                pos2(rect.center().x + shackle_r, body_rect.min.y - shackle_r),
                pos2(rect.center().x + shackle_r, body_rect.min.y),
            ];
            painter.line_segment([pts[0], pts[1]], stroke);
            painter.line_segment([pts[1], pts[2]], stroke);
            painter.line_segment([pts[2], pts[3]], stroke);
            // Keyhole dot
            painter.circle_filled(body_rect.center(), 1.2, color);
        }
        StorageIconKind::SwapVolume => {
            // Lightning / Fast Memory chevron polygon
            let pts = [
                pos2(rect.center().x + size * 0.12, rect.min.y + size * 0.08),
                pos2(rect.center().x - size * 0.22, rect.center().y + size * 0.04),
                pos2(rect.center().x, rect.center().y + size * 0.04),
                pos2(rect.center().x - size * 0.12, rect.max.y - size * 0.08),
                pos2(rect.center().x + size * 0.22, rect.center().y - size * 0.04),
                pos2(rect.center().x, rect.center().y - size * 0.04),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts.to_vec(),
                color,
                Stroke::NONE,
            ));
        }
        StorageIconKind::ExternalDrive => {
            // Portable external drive enclosure with connection port indicator
            let w = (size * 0.65).floor();
            let h = (size * 0.9).floor();
            let ext_rect = Rect::from_center_size(rect.center(), vec2(w, h));
            painter.rect(
                ext_rect,
                CornerRadius::same(3),
                color.linear_multiply(0.12),
                stroke,
                egui::StrokeKind::Inside,
            );
            // Top connector tab
            let port_w = (w * 0.5).floor();
            let port_rect = Rect::from_center_size(
                pos2(ext_rect.center().x, ext_rect.min.y),
                vec2(port_w, 2.0),
            );
            painter.rect_filled(port_rect, CornerRadius::same(1), color);
            // Activity dot
            painter.circle_filled(pos2(ext_rect.center().x, ext_rect.max.y - 3.0), 1.2, color);
        }
        StorageIconKind::StandardVolume => {
            // Standard macOS volume / folder outline
            let w = (size * 0.88).floor();
            let h = (size * 0.65).floor();
            let base_rect = Rect::from_min_size(
                pos2(rect.center().x - w * 0.5, rect.center().y - h * 0.25),
                vec2(w, h),
            );
            let tab_w = (w * 0.45).floor();
            let tab_h = (size * 0.22).floor();
            let tab_rect = Rect::from_min_size(
                pos2(base_rect.min.x, base_rect.min.y - tab_h + 1.0),
                vec2(tab_w, tab_h),
            );
            painter.rect(
                tab_rect,
                CornerRadius::same(2),
                color.linear_multiply(0.18),
                stroke,
                egui::StrokeKind::Inside,
            );
            painter.rect(
                base_rect,
                CornerRadius::same(2),
                color.linear_multiply(0.12),
                stroke,
                egui::StrokeKind::Inside,
            );
        }
    }
}

/// Renders a native vector magnifying glass search icon.
pub fn render_search_icon(ui: &mut Ui, color: Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let r = (size * 0.32).floor();
    let center = pos2(rect.min.x + r + 1.0, rect.min.y + r + 1.0);
    painter.circle_stroke(center, r, Stroke::new(1.2, color));
    let handle_start = pos2(center.x + r * 0.707, center.y + r * 0.707);
    let handle_end = pos2(rect.max.x - 1.0, rect.max.y - 1.0);
    painter.line_segment([handle_start, handle_end], Stroke::new(1.4, color));
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
            // Sun icon (center core + 8 radial rays)
            let core_r = radius * 0.45;
            painter.circle_filled(center, core_r, color);
            let ray_in = radius * 0.65;
            let ray_out = radius * 0.95;
            let stroke = Stroke::new(1.2, color);
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::PI / 4.0;
                let (sin, cos) = angle.sin_cos();
                painter.line_segment(
                    [
                        pos2(center.x + cos * ray_in, center.y + sin * ray_in),
                        pos2(center.x + cos * ray_out, center.y + sin * ray_out),
                    ],
                    stroke,
                );
            }
        }
        ThemePreference::Light => {
            // Moon icon (crescent polygon)
            let r = radius * 0.85;
            let pts = [
                pos2(center.x, center.y - r),
                pos2(center.x - r * 0.75, center.y - r * 0.3),
                pos2(center.x - r * 0.75, center.y + r * 0.3),
                pos2(center.x, center.y + r),
                pos2(center.x - r * 0.2, center.y + r * 0.5),
                pos2(center.x - r * 0.35, center.y),
                pos2(center.x - r * 0.2, center.y - r * 0.5),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts.to_vec(),
                color,
                Stroke::NONE,
            ));
        }
        ThemePreference::System => {
            // System Default Appearance (half-filled circle)
            let r = radius * 0.8;
            painter.circle_stroke(center, r, Stroke::new(1.2, color));
            let pts = [
                pos2(center.x, center.y - r),
                pos2(center.x - r * 0.7, center.y - r * 0.5),
                pos2(center.x - r * 0.7, center.y + r * 0.5),
                pos2(center.x, center.y + r),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts.to_vec(),
                color,
                Stroke::NONE,
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_status_color() {
        let theme = AppTheme::dark();

        let app_proc = ProcessInfo {
            pid: 1,
            name: "App".to_string(),
            path: None,
            memory_bytes: 0,
            resident_bytes: 0,
            peak_memory_bytes: 0,
            pageins: 0,
            disk_read_bytes: 0,
            disk_write_bytes: 0,
            cpu_usage: None,
            is_application: true,
            is_system_process: false,
            is_protected: false,
            user_id: None,
            bundle_id: None,
            architecture: crate::models::ProcessArchitecture::AppleSilicon,
        };
        assert_eq!(process_status_color(&app_proc, &theme), theme.accent_color);

        let sys_proc = ProcessInfo {
            is_application: false,
            is_system_process: true,
            ..app_proc.clone()
        };
        assert_eq!(
            process_status_color(&sys_proc, &theme),
            theme.text_secondary
        );

        let bg_proc = ProcessInfo {
            is_application: false,
            is_system_process: false,
            ..app_proc
        };
        assert_eq!(process_status_color(&bg_proc, &theme), theme.warning_color);
    }

    #[test]
    fn test_storage_icon_kind_variants() {
        let kinds = [
            StorageIconKind::InternalData,
            StorageIconKind::SystemSnapshot,
            StorageIconKind::SwapVolume,
            StorageIconKind::ExternalDrive,
            StorageIconKind::StandardVolume,
        ];
        assert_eq!(kinds.len(), 5);
    }
}
