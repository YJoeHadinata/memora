use std::process::Command;

/// Sends an asynchronous native macOS Notification banner via Notification Center.
pub fn send_notification(title: &str, body: &str) {
    let title_clean = title.replace('\\', "\\\\").replace('"', "\\\"");
    let body_clean = body.replace('\\', "\\\\").replace('"', "\\\"");

    std::thread::spawn(move || {
        let script = format!(
            "display notification \"{}\" with title \"{}\"",
            body_clean, title_clean
        );
        let _ = Command::new("osascript").args(["-e", &script]).output();
    });
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_notification_clean_strings() {
        let raw_title = "Memora \"Alert\" \\ test";
        let raw_body = "Process \"Chrome\" using > 3GB";
        let title_clean = raw_title.replace('\\', "\\\\").replace('"', "\\\"");
        let body_clean = raw_body.replace('\\', "\\\\").replace('"', "\\\"");

        assert!(title_clean.contains("\\\""));
        assert!(body_clean.contains("\\\""));
    }
}
