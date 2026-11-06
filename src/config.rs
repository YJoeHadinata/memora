use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::models::{ProcessSortColumn, SortDirection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreference {
    System,
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MenuBarDisplayMode {
    #[default]
    IconAndUsage,
    IconOnly,
    UsageOnly,
}

fn default_true() -> bool {
    true
}

fn default_menu_bar_mode() -> MenuBarDisplayMode {
    MenuBarDisplayMode::IconAndUsage
}

fn default_ram_threshold() -> u32 {
    3
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub refresh_interval_secs: u64,
    pub theme: ThemePreference,
    pub history_capacity: usize,
    pub show_only_applications: bool,
    pub sort_column: ProcessSortColumn,
    pub sort_direction: SortDirection,
    #[serde(default = "default_true")]
    pub show_menu_bar: bool,
    #[serde(default = "default_menu_bar_mode")]
    pub menu_bar_display_mode: MenuBarDisplayMode,
    #[serde(default = "default_true")]
    pub notify_on_high_ram: bool,
    #[serde(default = "default_ram_threshold")]
    pub high_ram_threshold_gb: u32,
    #[serde(default = "default_true")]
    pub notify_on_pressure_warning: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            refresh_interval_secs: 2,
            theme: ThemePreference::System,
            history_capacity: 180, // 180 points * 2s = 6 minutes of history
            show_only_applications: false,
            sort_column: ProcessSortColumn::Memory,
            sort_direction: SortDirection::Descending,
            show_menu_bar: true,
            menu_bar_display_mode: MenuBarDisplayMode::IconAndUsage,
            notify_on_high_ram: true,
            high_ram_threshold_gb: 3,
            notify_on_pressure_warning: true,
        }
    }
}

impl AppConfig {
    pub fn config_path() -> Option<PathBuf> {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("Memora")
                .join("config.json"),
        )
    }

    pub fn load() -> Self {
        if let Some(path) = Self::config_path() {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            let json = serde_json::to_string_pretty(self)?;
            fs::write(path, json)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serde() {
        let config = AppConfig::default();
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: AppConfig = serde_json::from_str(&serialized).unwrap();

        assert_eq!(
            config.refresh_interval_secs,
            deserialized.refresh_interval_secs
        );
        assert_eq!(config.theme, deserialized.theme);
        assert_eq!(config.history_capacity, deserialized.history_capacity);
    }
}
