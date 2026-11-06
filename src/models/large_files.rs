use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LargeFileCategory {
    #[default]
    All,
    Archive,
    DiskImage,
    Media,
    Document,
    Other,
}

impl LargeFileCategory {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All Categories",
            Self::Archive => "Archives",
            Self::DiskImage => "Disk Images",
            Self::Media => "Media",
            Self::Document => "Documents",
            Self::Other => "Other",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        let ext = ext.to_lowercase();
        match ext.as_str() {
            "zip" | "tar" | "gz" | "tgz" | "7z" | "rar" | "bz2" | "xz" => Self::Archive,
            "dmg" | "iso" | "img" | "vmdk" | "qcow2" | "vdi" => Self::DiskImage,
            "mp4" | "mov" | "mkv" | "avi" | "m4v" | "webm" | "wav" | "mp3" | "flac" | "aac" => {
                Self::Media
            }
            "pdf" | "psd" | "ai" | "sketch" | "fig" | "key" | "pages" | "docx" | "xlsx"
            | "pptx" => Self::Document,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SizeFilter {
    #[default]
    Min50MB,
    Min100MB,
    Min500MB,
    Min1GB,
}

impl SizeFilter {
    pub fn min_bytes(&self) -> u64 {
        match self {
            Self::Min50MB => 50 * 1024 * 1024,
            Self::Min100MB => 100 * 1024 * 1024,
            Self::Min500MB => 500 * 1024 * 1024,
            Self::Min1GB => 1024 * 1024 * 1024,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Min50MB => "> 50 MB",
            Self::Min100MB => "> 100 MB",
            Self::Min500MB => "> 500 MB",
            Self::Min1GB => "> 1 GB",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgeFilter {
    #[default]
    Any,
    OlderThan1Month,
    OlderThan6Months,
    OlderThan1Year,
}

impl AgeFilter {
    pub fn min_days(&self) -> u64 {
        match self {
            Self::Any => 0,
            Self::OlderThan1Month => 30,
            Self::OlderThan6Months => 180,
            Self::OlderThan1Year => 365,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Any => "Any Age",
            Self::OlderThan1Month => "> 1 Month",
            Self::OlderThan6Months => "> 6 Months",
            Self::OlderThan1Year => "> 1 Year",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LargeFileItem {
    pub path: PathBuf,
    pub name: String,
    pub size_bytes: u64,
    pub category: LargeFileCategory,
    pub modified_days_ago: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LargeFilesScanResult {
    pub files: Vec<LargeFileItem>,
    pub scanned_at: Option<SystemTime>,
}

impl LargeFilesScanResult {
    pub fn total_selected_bytes(&self) -> u64 {
        self.files
            .iter()
            .filter(|f| f.selected)
            .map(|f| f.size_bytes)
            .sum()
    }

    pub fn total_bytes(&self) -> u64 {
        self.files.iter().map(|f| f.size_bytes).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_detection() {
        assert_eq!(
            LargeFileCategory::from_extension("dmg"),
            LargeFileCategory::DiskImage
        );
        assert_eq!(
            LargeFileCategory::from_extension("ZIP"),
            LargeFileCategory::Archive
        );
        assert_eq!(
            LargeFileCategory::from_extension("mp4"),
            LargeFileCategory::Media
        );
        assert_eq!(
            LargeFileCategory::from_extension("pdf"),
            LargeFileCategory::Document
        );
        assert_eq!(
            LargeFileCategory::from_extension("unknown"),
            LargeFileCategory::Other
        );
    }

    #[test]
    fn test_size_filters() {
        assert_eq!(SizeFilter::Min50MB.min_bytes(), 50 * 1024 * 1024);
        assert_eq!(SizeFilter::Min1GB.min_bytes(), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_scan_result_totals() {
        let result = LargeFilesScanResult {
            files: vec![
                LargeFileItem {
                    path: PathBuf::from("/test1.dmg"),
                    name: "test1.dmg".to_string(),
                    size_bytes: 100,
                    category: LargeFileCategory::DiskImage,
                    modified_days_ago: 10,
                    selected: true,
                },
                LargeFileItem {
                    path: PathBuf::from("/test2.mp4"),
                    name: "test2.mp4".to_string(),
                    size_bytes: 200,
                    category: LargeFileCategory::Media,
                    modified_days_ago: 5,
                    selected: false,
                },
            ],
            scanned_at: None,
        };

        assert_eq!(result.total_bytes(), 300);
        assert_eq!(result.total_selected_bytes(), 100);
    }
}
