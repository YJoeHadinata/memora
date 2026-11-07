use egui::{pos2, vec2, Color32, CornerRadius, Rect, Stroke, Ui};

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
            let line_y = (drive_rect.center().y + h * 0.1).floor();
            painter.line_segment(
                [
                    pos2(drive_rect.min.x + 2.0, line_y),
                    pos2(drive_rect.max.x - 2.0, line_y),
                ],
                Stroke::new(1.0, color.linear_multiply(0.6)),
            );
            painter.circle_filled(
                pos2(drive_rect.max.x - 3.5, drive_rect.max.y - 2.8),
                1.2,
                color,
            );
        }
        StorageIconKind::SystemSnapshot => {
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
            let shackle_r = (body_w * 0.32).floor();
            let pts = [
                pos2(rect.center().x - shackle_r, body_rect.min.y),
                pos2(rect.center().x - shackle_r, body_rect.min.y - shackle_r),
                pos2(rect.center().x + shackle_r, body_rect.min.y - shackle_r),
                pos2(rect.center().x + shackle_r, body_rect.min.y),
            ];
            for pair in pts.windows(2) {
                painter.line_segment([pair[0], pair[1]], stroke);
            }
            painter.circle_filled(body_rect.center(), 1.2, color);
        }
        StorageIconKind::SwapVolume => {
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
            let port_rect = Rect::from_center_size(
                pos2(ext_rect.center().x, ext_rect.min.y),
                vec2((w * 0.5).floor(), 2.0),
            );
            painter.rect_filled(port_rect, CornerRadius::same(1), color);
            painter.circle_filled(pos2(ext_rect.center().x, ext_rect.max.y - 3.0), 1.2, color);
        }
        StorageIconKind::StandardVolume => {
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
