use std::borrow::Cow;

use egui::{Color32, CornerRadius, Ui};

use crate::ui::helpers::render_progress_bar;
use crate::ui::theme::AppTheme;

#[derive(Clone)]
pub struct MetricCard<'a> {
    pub title: Cow<'a, str>,
    pub value: Cow<'a, str>,
    pub subtitle: Option<Cow<'a, str>>,
    pub progress: Option<f32>, // 0.0 to 1.0
    pub progress_color: Option<Color32>,
    pub width: Option<f32>,
}

impl<'a> MetricCard<'a> {
    pub fn new(title: impl Into<Cow<'a, str>>, value: impl Into<Cow<'a, str>>) -> Self {
        Self {
            title: title.into(),
            value: value.into(),
            subtitle: None,
            progress: None,
            progress_color: None,
            width: None,
        }
    }

    pub fn subtitle(mut self, subtitle: impl Into<Cow<'a, str>>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    pub fn progress(mut self, progress: f32, color: Color32) -> Self {
        self.progress = Some(progress.clamp(0.0, 1.0));
        self.progress_color = Some(color);
        self
    }

    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    pub fn ui(self, ui: &mut Ui, theme: &AppTheme) {
        let card_w = self.width.unwrap_or(160.0);
        // Margin: 16px left + 16px right + 2px stroke = 34px total frame overhead
        let inner_w = (card_w - 34.0).max(60.0);

        egui::Frame::new()
            .fill(theme.card_bg)
            .corner_radius(theme.card_radius())
            .stroke(theme.card_stroke())
            .inner_margin(egui::Margin::symmetric(16, 14))
            .show(ui, |ui| {
                ui.set_width(inner_w);
                ui.set_max_width(inner_w);
                ui.vertical(|ui| {
                    // Apple-style uppercase metric header
                    ui.label(
                        egui::RichText::new(self.title.to_uppercase())
                            .size(10.5)
                            .strong()
                            .color(theme.text_secondary),
                    );

                    ui.add_space(4.0);

                    // Large Apple SF Pro typographic number
                    ui.label(
                        egui::RichText::new(&*self.value)
                            .size(23.0)
                            .strong()
                            .color(theme.text_primary),
                    );

                    if let Some(sub) = self.subtitle {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new(sub.as_ref())
                                .size(11.0)
                                .color(theme.text_secondary),
                        );
                    }

                    if let Some(prog) = self.progress {
                        ui.add_space(8.0);
                        let track_bg = Color32::from_rgba_unmultiplied(
                            theme.text_secondary.r(),
                            theme.text_secondary.g(),
                            theme.text_secondary.b(),
                            35,
                        );
                        let color = self.progress_color.unwrap_or(theme.accent_color);
                        render_progress_bar(ui, prog, 5.0, color, track_bg, CornerRadius::same(3));
                    }
                });
            });
    }
}
