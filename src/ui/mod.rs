pub mod caches;
pub mod components;
pub mod dashboard;
pub mod helpers;
pub mod processes;
pub mod settings;
pub mod theme;

pub use caches::{
    render_caches, CacheActionRequest, CacheViewState, CleanupActionRequest, CleanupSubTab,
    CleanupViewState,
};
pub use dashboard::{render_dashboard, DashboardAction};
pub use helpers::{
    paint_theme_icon, process_status_color, render_app_logo, render_progress_bar,
    render_search_icon, render_section_header, render_status_dot, render_storage_icon,
    render_subtle_pill, render_tinted_pill, StorageIconKind,
};
pub use processes::{render_processes, ProcessActionRequest, ProcessViewState};
pub use settings::render_settings;
pub use theme::AppTheme;
