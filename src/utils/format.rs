use chrono::{DateTime, Local};
use std::time::SystemTime;

/// Formats a byte count into a human-readable string using binary units (KiB, MiB, GiB)
/// with standard macOS GB/MB representation.
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    const TB: u64 = 1024 * GB;

    if bytes >= TB {
        format!("{:.2} TB", bytes as f64 / TB as f64)
    } else if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Formats a ratio as a human-readable percentage.
pub fn format_percent(percent: f32) -> String {
    format!("{:.1}%", percent.clamp(0.0, 100.0))
}

/// Formats a SystemTime as a localized HH:MM:SS string.
pub fn format_time_hhmmss(time: SystemTime) -> String {
    let dt: DateTime<Local> = time.into();
    dt.format("%H:%M:%S").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1.00 GB");
        assert_eq!(format_bytes(16 * 1024 * 1024 * 1024), "16.00 GB");
        assert_eq!(format_bytes(1024 * 1024 * 1024 * 1024), "1.00 TB");
    }

    #[test]
    fn test_format_percent() {
        assert_eq!(format_percent(0.0), "0.0%");
        assert_eq!(format_percent(64.28), "64.3%");
        assert_eq!(format_percent(100.0), "100.0%");
        assert_eq!(format_percent(120.0), "100.0%");
        assert_eq!(format_percent(-5.0), "0.0%");
    }
}
