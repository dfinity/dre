use log::{error, info, warn};
use std::process::Command;

pub struct DesktopNotifier;

impl DesktopNotifier {
    /// Sends an informational notification.
    ///
    /// # Arguments
    ///
    /// * `title` - A title for the notification.
    /// * `message` - The message body of the notification.
    pub fn send_info(title: &str, message: &str) {
        DesktopNotifier::notify(title, message, "info");
    }

    /// Sends a critical notification.
    ///
    /// # Arguments
    ///
    /// * `title` - A title for the notification.
    /// * `message` - The message body of the notification.
    pub fn send_critical(title: &str, message: &str) {
        DesktopNotifier::notify(title, message, "critical");
    }

    #[cfg(target_os = "macos")]
    fn notify(title: &str, message: &str, level: &str) {
        if level == "critical" {
            warn!("{}: {}", title, message);
        } else {
            info!("{}: {}", title, message);
        }

        let command_result = Command::new("osascript")
            .arg("-e")
            .arg(format!(
                r#"display notification "{}" with title "{}" subtitle "{}""#,
                message, title, level
            ))
            .output();

        match command_result {
            Ok(output) => {
                if !output.status.success() {
                    error!("Failed to send macOS notification: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(err) => {
                error!("Notification command not found or failed: {}", err);
            }
        }
    }

    #[cfg(target_os = "linux")]
    fn notify(title: &str, message: &str, level: &str) {
        let urgency = if level == "critical" {
            warn!("{}: {}", title, message);
            "critical"
        } else {
            info!("{}: {}", title, message);
            "normal"
        };

        let command_result = Command::new("notify-send")
            .arg("-u")
            .arg(urgency)
            .arg("-a")
            .arg("dre")
            .arg(title)
            .arg(message)
            .output();

        match command_result {
            Ok(output) => {
                if !output.status.success() {
                    error!("Failed to send Linux notification: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(err) => {
                error!("Notification command not found or failed: {}", err);
            }
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    fn notify() {
        info!("Notification system is not supported on this operating system.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fs_err::{File, OpenOptions};
    use log::{LevelFilter, Metadata, Record};
    use std::io::Write;
    use std::sync::Mutex;
    use std::sync::Once;

    static INIT: Once = Once::new();

    fn initialize() {
        INIT.call_once(|| {
            // Initialize logger
            let file = OpenOptions::new().create(true).write(true).truncate(true).open("test.log").unwrap();

            log::set_boxed_logger(Box::new(SimpleLogger { file: Mutex::new(file) })).unwrap();
            log::set_max_level(LevelFilter::Info);
        });
    }

    struct SimpleLogger {
        file: Mutex<File>,
    }

    impl log::Log for SimpleLogger {
        fn enabled(&self, metadata: &Metadata) -> bool {
            metadata.level() <= log::max_level()
        }

        fn log(&self, record: &Record) {
            if self.enabled(record.metadata()) {
                let mut file = self.file.lock().unwrap();
                writeln!(file, "{} - {}", record.level(), record.args()).unwrap();
            }
        }

        fn flush(&self) {}
    }

    #[test]
    fn test_send_info_notification() {
        initialize();
        DesktopNotifier::send_info("Test Info", "This is an info notification");
    }

    #[test]
    fn test_send_critical_notification() {
        initialize();
        DesktopNotifier::send_critical("Test Critical", "This is a critical notification");
    }

    #[test]
    fn test_unsupported_os() {
        initialize();
        if !cfg!(target_os = "macos") && !cfg!(target_os = "linux") {
            DesktopNotifier::send_info("Unsupported OS Test", "Testing unsupported OS handling");
            DesktopNotifier::send_critical("Unsupported OS Test", "Testing unsupported OS handling");
        }
    }
}
