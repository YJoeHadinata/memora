use std::fs;
use std::path::Path;
use std::process::Command;

/// Extracts a string key from a macOS `.plist` file.
/// Uses a fast in-memory XML parser for text plists and falls back to `plutil` for binary plists.
pub fn extract_plist_string(plist_path: &Path, key: &str) -> Option<String> {
    if !key.contains('.') {
        if let Ok(bytes) = fs::read(plist_path) {
            if !bytes.starts_with(b"bplist") {
                if let Ok(xml) = std::str::from_utf8(&bytes) {
                    if let Some(val) = parse_xml_plist_string(xml, key) {
                        return Some(val);
                    }
                }
            }
        }
    }

    let output = Command::new("plutil")
        .args(["-extract", key, "raw", "-o", "-", plist_path.to_str()?])
        .output()
        .ok()?;

    if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !val.is_empty() {
            return Some(val);
        }
    }
    None
}

/// Extracts a boolean key from a macOS `.plist` file.
/// Uses a fast in-memory XML parser for text plists and falls back to `plutil` for binary plists.
pub fn extract_plist_bool(plist_path: &Path, key: &str) -> Option<bool> {
    if !key.contains('.') {
        if let Ok(bytes) = fs::read(plist_path) {
            if !bytes.starts_with(b"bplist") {
                if let Ok(xml) = std::str::from_utf8(&bytes) {
                    if let Some(val) = parse_xml_plist_bool(xml, key) {
                        return Some(val);
                    }
                }
            }
        }
    }

    let output = Command::new("plutil")
        .args(["-extract", key, "raw", "-o", "-", plist_path.to_str()?])
        .output()
        .ok()?;

    if output.status.success() {
        let val = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_lowercase();
        if val == "true" || val == "1" {
            return Some(true);
        } else if val == "false" || val == "0" {
            return Some(false);
        }
    }
    None
}

fn parse_xml_plist_string(xml: &str, key: &str) -> Option<String> {
    let needle = format!("<key>{}</key>", key);
    let idx = xml.find(&needle)?;
    let rest = xml[idx + needle.len()..].trim_start();
    if let Some(after_open) = rest.strip_prefix("<string>") {
        let end = after_open.find("</string>")?;
        let val = after_open[..end].trim();
        if !val.is_empty() {
            return Some(val.to_string());
        }
    }
    None
}

fn parse_xml_plist_bool(xml: &str, key: &str) -> Option<bool> {
    let needle = format!("<key>{}</key>", key);
    let idx = xml.find(&needle)?;
    let rest = xml[idx + needle.len()..].trim_start();
    if rest.starts_with("<true/>") || rest.starts_with("<true />") {
        Some(true)
    } else if rest.starts_with("<false/>") || rest.starts_with("<false />") {
        Some(false)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_xml_plist_helpers() {
        let xml = r#"
        <plist version="1.0">
        <dict>
            <key>Label</key>
            <string>com.example.agent</string>
            <key>RunAtLoad</key>
            <true/>
            <key>KeepAlive</key>
            <false/>
        </dict>
        </plist>
        "#;
        assert_eq!(
            parse_xml_plist_string(xml, "Label").as_deref(),
            Some("com.example.agent")
        );
        assert_eq!(parse_xml_plist_bool(xml, "RunAtLoad"), Some(true));
        assert_eq!(parse_xml_plist_bool(xml, "KeepAlive"), Some(false));
    }
}
