use std::sync::Arc;

use memora::app::MemoraApp;

fn main() -> eframe::Result<()> {
    // Initialize tracing subscriber for clean logs
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("Starting Memora v0.1.0 on macOS...");

    let icon_data =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon_256.png"))
            .expect("Valid Memora app icon");

    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        viewport: egui::ViewportBuilder::default()
            .with_title("Memora")
            .with_icon(Arc::new(icon_data))
            .with_inner_size([960.0, 680.0])
            .with_min_inner_size([800.0, 540.0])
            .with_active(true),
        ..Default::default()
    };

    eframe::run_native(
        "Memora",
        native_options,
        Box::new(|cc| Ok(Box::new(MemoraApp::new(cc)))),
    )
}
