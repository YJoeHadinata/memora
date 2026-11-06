use egui::{Color32, CornerRadius, Stroke};

use crate::models::MemoryPressure;

/// Apple Human Interface Guidelines (macOS) Design System Theme
#[derive(Debug, Clone)]
pub struct AppTheme {
    pub bg_fill: Color32,
    pub card_bg: Color32,
    pub card_hover: Color32,
    pub subtle_bg: Color32,
    pub border_color: Color32,
    pub border_subtle: Color32,
    pub text_primary: Color32,
    pub text_secondary: Color32,
    pub text_tertiary: Color32,
    pub accent_color: Color32,
    pub normal_color: Color32,
    pub warning_color: Color32,
    pub critical_color: Color32,
    pub purple_color: Color32,
    pub cyan_color: Color32,
}

impl AppTheme {
    /// macOS Dark Mode (Graphite / Sonoma / Sequoia Dark)
    pub fn dark() -> Self {
        Self {
            bg_fill: Color32::from_rgb(20, 20, 22), // macOS System Background (Secondary Dark)
            card_bg: Color32::from_rgb(32, 33, 36), // macOS Elevated Grouped Surface
            card_hover: Color32::from_rgb(42, 43, 48), // Hover state
            subtle_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 12), // Subtle pill/capsule background
            border_color: Color32::from_rgba_unmultiplied(255, 255, 255, 22), // 1px Apple border
            border_subtle: Color32::from_rgba_unmultiplied(255, 255, 255, 14),
            text_primary: Color32::from_rgb(245, 245, 247), // Apple Typography Primary
            text_secondary: Color32::from_rgb(152, 152, 157), // Apple System Gray (Label Secondary)
            text_tertiary: Color32::from_rgb(110, 110, 115), // Tertiary label
            accent_color: Color32::from_rgb(10, 132, 255),  // Apple System Blue (Dark)
            normal_color: Color32::from_rgb(48, 209, 88),   // Apple System Green (Dark)
            warning_color: Color32::from_rgb(255, 159, 10), // Apple System Orange/Amber
            critical_color: Color32::from_rgb(255, 69, 58), // Apple System Red
            purple_color: Color32::from_rgb(191, 90, 242),  // Apple System Purple
            cyan_color: Color32::from_rgb(100, 210, 255),   // Apple System Cyan
        }
    }

    /// macOS Light Mode (System Gray 6 / Clean White)
    pub fn light() -> Self {
        Self {
            bg_fill: Color32::from_rgb(242, 242, 247), // macOS System Gray 6 (Canvas)
            card_bg: Color32::from_rgb(255, 255, 255), // Pure White Card
            card_hover: Color32::from_rgb(248, 249, 251), // Subtle hover
            subtle_bg: Color32::from_rgba_unmultiplied(0, 0, 0, 12), // Subtle pill background
            border_color: Color32::from_rgba_unmultiplied(0, 0, 0, 20), // Subtle hairline stroke
            border_subtle: Color32::from_rgba_unmultiplied(0, 0, 0, 12),
            text_primary: Color32::from_rgb(29, 29, 31), // Apple Typography Primary Dark
            text_secondary: Color32::from_rgb(134, 134, 139), // Apple System Gray (Light)
            text_tertiary: Color32::from_rgb(174, 174, 178),
            accent_color: Color32::from_rgb(0, 122, 255), // Apple System Blue (Light)
            normal_color: Color32::from_rgb(52, 199, 89), // Apple System Green (Light)
            warning_color: Color32::from_rgb(255, 149, 0), // Apple System Orange
            critical_color: Color32::from_rgb(255, 59, 48), // Apple System Red
            purple_color: Color32::from_rgb(175, 82, 222), // Apple System Purple
            cyan_color: Color32::from_rgb(50, 173, 230),  // Apple System Cyan
        }
    }

    pub fn pressure_color(&self, pressure: MemoryPressure) -> Color32 {
        match pressure {
            MemoryPressure::Normal => self.normal_color,
            MemoryPressure::Warning => self.warning_color,
            MemoryPressure::Critical => self.critical_color,
            MemoryPressure::Unknown => self.text_secondary,
        }
    }

    pub fn card_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border_color)
    }

    pub fn card_radius(&self) -> CornerRadius {
        CornerRadius::same(12)
    }

    pub fn button_radius(&self) -> CornerRadius {
        CornerRadius::same(7)
    }

    /// Standard elevated macOS card frame with 16px inner padding and 12px rounded corners.
    pub fn card_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.card_bg)
            .corner_radius(self.card_radius())
            .stroke(self.card_stroke())
            .inner_margin(egui::Margin::same(16))
    }

    /// Subtle pill container frame for segmented controls or grouped controls.
    pub fn subtle_frame(&self, radius: CornerRadius, margin: egui::Margin) -> egui::Frame {
        egui::Frame::new()
            .fill(self.subtle_bg)
            .corner_radius(radius)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .inner_margin(margin)
    }

    /// Standard macOS modal window frame with 20px padding and 14px rounded corners.
    pub fn modal_frame(&self) -> egui::Frame {
        egui::Frame::new()
            .fill(self.card_bg)
            .stroke(self.card_stroke())
            .corner_radius(CornerRadius::same(14))
            .inner_margin(egui::Margin::same(20))
    }

    /// Primary accent button (e.g. Purge RAM, Quit)
    pub fn primary_button<'a>(&self, text: impl Into<egui::RichText>) -> egui::Button<'a> {
        egui::Button::new(text.into().color(Color32::WHITE).strong())
            .fill(self.accent_color)
            .corner_radius(self.button_radius())
    }

    /// Subtle secondary button (e.g. Cancel, Done, Details)
    pub fn subtle_button<'a>(&self, text: impl Into<egui::RichText>) -> egui::Button<'a> {
        egui::Button::new(text.into().color(self.text_primary))
            .fill(self.subtle_bg)
            .stroke(Stroke::new(1.0, self.border_subtle))
            .corner_radius(self.button_radius())
    }

    /// Destructive critical button (e.g. Force Quit, Clean Caches)
    pub fn critical_button<'a>(&self, text: impl Into<egui::RichText>) -> egui::Button<'a> {
        egui::Button::new(text.into().color(Color32::WHITE).strong())
            .fill(self.critical_color)
            .corner_radius(self.button_radius())
    }
}
