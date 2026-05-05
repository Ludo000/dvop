//! # Status Bar Logging System
//!
//! This module provides a centralized logging system that displays messages in the
//! application's status bar and persists them to a log file.
//!
//! ## Log Levels
//!
//! - **Info** (blue): General information ("File list refreshed", "Switched to main.rs")
//! - **Warning** (yellow): Potential issues ("File type not supported")
//! - **Error** (red): Failures ("Failed to save file: Permission denied")
//! - **Success** (green): Completed operations ("Saved main.rs")
//!
//! ## Architecture
//!
//! - Messages are stored in a **circular buffer** (`VecDeque`) with a maximum of 100 entries.
//!   When the buffer is full, the oldest message is discarded.
//! - The status bar `Label` widget is stored via **thread-local storage** (`thread_local!`)
//!   so any module can call `log_info()`, `log_error()`, etc. without needing a reference
//!   to the label widget.
//! - Messages are also written to `~/.config/dvop/log_history.txt` for persistence across
//!   sessions. This file is loaded at startup to show the last session's log.
//!
//! ## Thread-Local Storage
//!
//! `thread_local!` creates a variable that has a separate instance per thread. Since GTK
//! is single-threaded, there's effectively one instance. This pattern avoids global mutable
//! state (which Rust's borrow checker forbids) by giving each thread its own copy.
//!
//! See FEATURES.md: Feature #117 — Notification System
//! See FEATURES.md: Feature #118 — Log History

// Status logging system for Dvop
// Provides a way to capture and display log messages in the status bar

use gtk4::prelude::*;
use gtk4::Label;

// VecDeque: Short for "Vector Double-Ended Queue". We use it as a "Circular Buffer".
// It's like a pipe: you push new logs in the back, and when it gets too full (100+), 
// it efficiently pops the oldest one off the front.
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

// "impl" blocks define methods and behavior for a struct or enum.
impl LogMessage {
    /// Serialize log message to a string for file storage
    
    // "Serialization" is just a fancy word for turning a struct/object into a 
    // single line of text so we can save it to a file. We use '|' as a separator.
    pub fn to_string(&self) -> String {
        let timestamp = self
            .timestamp
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or(std::time::Duration::from_secs(0))
            .as_secs();
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        let level_str = match self.level {
            LogLevel::Info => "INFO",
            LogLevel::Warning => "WARNING",
            LogLevel::Error => "ERROR",
            LogLevel::Success => "SUCCESS",
        };
        format!("{}|{}|{}", timestamp, level_str, self.message)
    }

    /// Deserialize log message from a string
    
    // This turns a line of text back into a LogMessage struct. 
    // 'Option' means it returns 'None' if the line is broken or formatted wrong.
    pub fn from_string(s: &str) -> Option<LogMessage> {
        let parts: Vec<&str> = s.splitn(3, '|').collect();
        if parts.len() != 3 {
            return None;
        }

        let timestamp_secs: u64 = parts[0].parse().ok()?;
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
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

/// Severity level of a log message, used for visual styling in the status bar.
///
/// Each level corresponds to a different color in the UI:
/// - `Info` → default (no special color)
/// - `Warning` → yellow/orange
/// - `Error` → red
/// - `Success` → green
///
/// See FEATURES.md: Feature #117 — Notification System
#[derive(Clone, PartialEq, Debug)]
pub enum LogLevel {
    /// General information messages (file opened, tab switched, etc.)
    Info,
    /// Warnings about potential issues (unsupported file type, etc.)
    Warning,
    /// Error messages (file save failed, permission denied, etc.)
    Error,
    /// Success messages (file saved, operation completed, etc.)
    Success,
}

/// Global log history storage — a thread-safe circular buffer of recent log messages.
///
/// Uses `Lazy` for deferred initialization (created on first access) and
/// `Arc<Mutex<VecDeque<...>>>` for thread-safe, bounded storage.
/// `VecDeque` is a double-ended queue that efficiently supports both push-back
/// and pop-front operations (needed for the circular buffer behavior).

// Mutex ensures that even if two things try to log at the exact same millisecond, 
// they take turns and don't corrupt the history list.
static STATUS_HISTORY: once_cell::sync::Lazy<Arc<Mutex<VecDeque<LogMessage>>>> =
    // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
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

    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    match fs::read_to_string(&log_file) {
        Ok(contents) => {
            // lock() acquires the Mutex lock. It blocks until the lock is available.
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

    // lock() acquires the Mutex lock. It blocks until the lock is available.
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

// thread_local! is a way to have a "Global" variable that is safe. 
// RefCell allows us to swap the Label in and out even though it's technically immutable.
thread_local! {
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
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
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
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
        // borrow() gets read-only access to the data inside a RefCell.
        if let Some(ref label) = *l.borrow() {
            // Update the label text
            label.set_text(&log_message.message);

            // Set appropriate CSS class based on log level
            // We remove all possible classes first so colors don't stack/conflict.
            label.remove_css_class("status-log-info");
            label.remove_css_class("status-log-warning");
            label.remove_css_class("status-log-error");
            label.remove_css_class("status-log-success");

            // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
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

/// Registers the GTK `Label` widgets that display log messages in the status bar.
///
/// Must be called once during UI setup (see `build_ui()` in `main.rs`). The labels
/// are stored in `thread_local!` storage so `log_info()` / `log_error()` etc. can
/// update them without needing a reference to the window.
pub fn register_status_labels(status_label: &Label, secondary_label: &Label) {
    set_status_label(status_label);
    set_secondary_status_label(secondary_label);

    // Show ready message initially
    log_info("Ready");
}

/// Logs an informational message to the status bar and console.
///
/// The message appears in the primary status label and is appended to the
/// in-memory history ring buffer. Also printed to stdout for debugging.
///
/// See FEATURES.md: Feature #117 — Notification System
pub fn log_info(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Info);
    // Also print to console for debugging
    println!("[INFO] {}", message);
}

/// Logs an error message to the status bar (displayed in red) and stderr.
///
/// See FEATURES.md: Feature #117 — Notification System
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

// This is used if we ever want to open a "Log Viewer" window. 
// It copies everything from the RAM storage into a simple Vector.
pub fn get_log_history() -> Vec<LogMessage> {
    STATUS_HISTORY
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        .lock()
        .map(|history| history.iter().cloned().collect())
        .unwrap_or_default()
}

/// Clear the log history
pub fn clear_log_history() {
    // lock() acquires the Mutex lock. It blocks until the lock is available.
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

#[cfg(test)]
mod tests {
    // Tests are standard Rust practice to ensure the serialization 
    // and history logic doesn't break if someone changes a line of code.
    use super::*;

    #[test]
    fn test_log_message_serialization() {
        let msg = LogMessage {
            timestamp: std::time::SystemTime::now(),
            message: "Test message".to_string(),
            level: LogLevel::Info,
        };

        let serialized = msg.to_string();
        assert!(serialized.contains("INFO"));
        assert!(serialized.contains("Test message"));
    }

    #[test]
    fn test_log_message_deserialization() {
        let timestamp = 1700000000u64;
        let serialized = format!("{}|INFO|Test message", timestamp);
        
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let msg = LogMessage::from_string(&serialized).unwrap();
        assert_eq!(msg.message, "Test message");
        assert_eq!(msg.level, LogLevel::Info);
    }

    #[test]
    fn test_log_message_roundtrip() {
        let original = LogMessage {
            timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(1700000000),
            message: "Roundtrip test".to_string(),
            level: LogLevel::Warning,
        };

        let serialized = original.to_string();
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let deserialized = LogMessage::from_string(&serialized).unwrap();

        assert_eq!(deserialized.message, original.message);
        assert_eq!(deserialized.level, original.level);
    }

    #[test]
    fn test_log_levels() {
        let info = LogMessage {
            timestamp: std::time::SystemTime::now(),
            message: "Info".to_string(),
            level: LogLevel::Info,
        };
        assert_eq!(info.level, LogLevel::Info);

        let warning = LogMessage {
            timestamp: std::time::SystemTime::now(),
            message: "Warning".to_string(),
            level: LogLevel::Warning,
        };
        assert_eq!(warning.level, LogLevel::Warning);

        let error = LogMessage {
            timestamp: std::time::SystemTime::now(),
            message: "Error".to_string(),
            level: LogLevel::Error,
        };
        assert_eq!(error.level, LogLevel::Error);

        let success = LogMessage {
            timestamp: std::time::SystemTime::now(),
            message: "Success".to_string(),
            level: LogLevel::Success,
        };
        assert_eq!(success.level, LogLevel::Success);
    }

    #[test]
    fn test_invalid_log_message_deserialization() {
        assert!(LogMessage::from_string("invalid").is_none());
        assert!(LogMessage::from_string("only|two").is_none());
        assert!(LogMessage::from_string("abc|INFO|message").is_none()); // Invalid timestamp
        assert!(LogMessage::from_string("123|INVALID|message").is_none()); // Invalid level
    }

    #[test]
    fn test_log_file_path() {
        let log_path = get_log_file_path();
        assert!(log_path.is_absolute());
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        assert!(log_path.to_str().unwrap().ends_with("log_history.txt"));
    }

    #[test]
    fn test_max_log_history_constant() {
        assert_eq!(MAX_LOG_HISTORY, 100);
        assert!(MAX_LOG_HISTORY > 0);
    }
}
