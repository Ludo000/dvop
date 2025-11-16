// Status logging system for Dvop
// Provides a way to capture and display log messages in the status bar

use gtk4::prelude::*;
use gtk4::Label;
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Maximum number of log messages to keep in memory
const MAX_LOG_HISTORY: usize = 100;

/// A log message with timestamp and content
#[derive(Clone)]
pub struct LogMessage {
    pub timestamp: std::time::SystemTime,
    pub message: String,
    pub level: LogLevel,
}

impl LogMessage {
    /// Serialize log message to a string for file storage
    pub fn to_string(&self) -> String {
        let timestamp = self
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();
        let level_str = match self.level {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Success => "SUCCESS",
        };
        format!("{}|{}|{}", timestamp, level_str, self.message)
    }

    /// Deserialize log message from a string
    pub fn from_string(s: &str) -> Option<LogMessage> {
        let parts: Vec<&str> = s.splitn(3, '|').collect();
        if parts.len() != 3 {
            return None;
        }

        let timestamp_secs: u64 = parts[0].parse().ok()?;
        let level = match parts[1] {
            "INFO" => LogLevel::Info,
            "WARNING" => LogLevel::Warning,
            "ERROR" => LogLevel::Error,
            "SUCCESS" => LogLevel::Success,
            _ => return None,
        };
        let message = parts[2].to_string();

        let timestamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(timestamp_secs);

        Some(LogMessage {
            timestamp,
            message,
            level,
        })
    }
}

/// Log levels for different types of messages
#[derive(Clone, PartialEq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
}

/// Simple status log storage (no global state, just for history)
static STATUS_HISTORY: once_cell::sync::Lazy<Arc<Mutex<VecDeque<LogMessage>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(VecDeque::new())));

/// Get the log file path
fn get_log_file_path() -> PathBuf {
    // Use the same configuration directory as settings
    let config_dir = crate::settings::get_config_dir_public();
    config_dir.join("log_history.txt")
}

/// Load log history from file
pub fn load_log_history() {
    let log_file = get_log_file_path();

    // Create the config directory if it doesn't exist
    if let Some(parent) = log_file.parent() {
        if !parent.exists() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory for logs: {}", e);
                return;
            }
        }
    }

    match fs::read_to_string(&log_file) {
        Ok(contents) => {
            if let Ok(mut history) = STATUS_HISTORY.lock() {
                history.clear();

                // Parse each line as a log message
                for line in contents.lines() {
                    if let Some(log_message) = LogMessage::from_string(line) {
                        history.push_back(log_message);
                    }
                }

                // Keep only the last MAX_LOG_HISTORY messages
                while history.len() > MAX_LOG_HISTORY {
                    history.pop_front();
                }

                println!("Loaded {} log messages from history", history.len());
            }
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // File doesn't exist yet, that's fine for first run
            println!("Log history file not found, starting fresh");
        }
        Err(e) => {
            eprintln!("Failed to load log history: {}", e);
        }
    }
}

/// Save log history to file
fn save_log_history() {
    let log_file = get_log_file_path();

    if let Ok(history) = STATUS_HISTORY.lock() {
        let mut contents = String::new();
        for message in history.iter() {
            contents.push_str(&message.to_string());
            contents.push('\n');
        }

        if let Err(e) = fs::write(&log_file, contents) {
            eprintln!("Failed to save log history: {}", e);
        }
    }
}

// Store a reference to the current secondary status label (if any)
thread_local! {
    static CURRENT_SECONDARY_STATUS_LABEL: std::cell::RefCell<Option<Label>> = const { std::cell::RefCell::new(None) };
}

/// Set the current secondary status label for this thread
pub fn set_secondary_status_label(label: &Label) {
    CURRENT_SECONDARY_STATUS_LABEL.with(|l| {
        *l.borrow_mut() = Some(label.clone());
    });
}

// Store a reference to the current status label (if any)
thread_local! {
    static CURRENT_STATUS_LABEL: std::cell::RefCell<Option<Label>> = const { std::cell::RefCell::new(None) };
}

/// Set the current status label for this thread
pub fn set_status_label(label: &Label) {
    CURRENT_STATUS_LABEL.with(|l| {
        *l.borrow_mut() = Some(label.clone());
    });
}

/// Update status label with a log message
fn update_status_label(log_message: &LogMessage) {
    CURRENT_STATUS_LABEL.with(|l| {
        if let Some(ref label) = *l.borrow() {
            // Update the label text
            label.set_text(&log_message.message);

            // Set appropriate CSS class based on log level
            label.remove_css_class("status-log-info");
            label.remove_css_class("status-log-warning");
            label.remove_css_class("status-log-error");
            label.remove_css_class("status-log-success");

            let css_class = match log_message.level {
                LogLevel::Info => "status-log-info",
                LogLevel::Warning => "status-log-warning",
                LogLevel::Error => "status-log-error",
                LogLevel::Success => "status-log-success",
            };
            label.add_css_class(css_class);

            // Update tooltip with timestamp and level
            let elapsed = log_message
                .timestamp
                .elapsed()
                .map(|d| format!("{:.1}s ago", d.as_secs_f32()))
                .unwrap_or_else(|_| "just now".to_string());

            let level_str = match log_message.level {
                LogLevel::Info => "Info",
                LogLevel::Warning => "Warning",
                LogLevel::Error => "Error",
                LogLevel::Success => "Success",
            };

            let tooltip = format!("{} ({}): {}", level_str, elapsed, log_message.message);
            label.set_tooltip_text(Some(&tooltip));
        }
    });
}

/// Add a message to history and update UI
fn add_message_to_log(message: String, level: LogLevel) {
    let log_message = LogMessage {
        timestamp: std::time::SystemTime::now(),
        message,
        level,
    };

    // Add to history
    if let Ok(mut history) = STATUS_HISTORY.lock() {
        history.push_back(log_message.clone());

        // Keep only the last MAX_LOG_HISTORY messages
        if history.len() > MAX_LOG_HISTORY {
            history.pop_front();
        }
    }

    // Save to file asynchronously
    save_log_history();

    // Update the current status label
    update_status_label(&log_message);
}

/// Register status bar labels to receive log updates
pub fn register_status_labels(status_label: &Label, secondary_label: &Label) {
    set_status_label(status_label);
    set_secondary_status_label(secondary_label);

    // Show ready message initially
    log_info("Ready");
}

/// Log an info message to the status bar
pub fn log_info(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Info);
    // Also print to console for debugging
    println!("[INFO] {}", message);
}

/// Log an error message to the status bar
pub fn log_error(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Error);
    // Also print to console for debugging
    eprintln!("[ERROR] {}", message);
}

/// Log a success message to the status bar
pub fn log_success(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Success);
    // Also print to console for debugging
    println!("[SUCCESS] {}", message);
}

/// Get the complete message history
pub fn get_log_history() -> Vec<LogMessage> {
    STATUS_HISTORY
        .lock()
        .map(|history| history.iter().cloned().collect())
        .unwrap_or_default()
}

/// Clear the log history
pub fn clear_log_history() {
    if let Ok(mut history) = STATUS_HISTORY.lock() {
        history.clear();
    }

    // Also clear the saved file
    let log_file = get_log_file_path();
    if let Err(e) = fs::write(&log_file, "") {
        eprintln!("Failed to clear log history file: {}", e);
    }

    // Show ready message
    log_info("Ready");
}
