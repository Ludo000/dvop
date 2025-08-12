// Status logging system for the Basado Text Editor
// Provides a way to capture and display log messages in the status bar

use gtk4::prelude::*;
use gtk4::Label;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;

/// Maximum number of log messages to keep in memory
const MAX_LOG_HISTORY: usize = 100;

/// A log message with timestamp and content
#[derive(Clone)]
pub struct LogMessage {
    pub timestamp: std::time::SystemTime,
    pub message: String,
    pub level: LogLevel,
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

/// Store a reference to the current status label (if any)
thread_local! {
    static CURRENT_STATUS_LABEL: std::cell::RefCell<Option<Label>> = std::cell::RefCell::new(None);
}

/// Set the current status label for this thread
pub fn set_status_label(label: &Label) {
    CURRENT_STATUS_LABEL.with(|l| {
        *l.borrow_mut() = Some(label.clone());
    });
}

/// Clear the current status label
pub fn clear_status_label() {
    CURRENT_STATUS_LABEL.with(|l| {
        *l.borrow_mut() = None;
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
            let elapsed = log_message.timestamp
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

    // Update the current status label
    update_status_label(&log_message);
}

/// Register a status bar label to receive log updates (compatibility function)
pub fn register_status_label(label: &Label) {
    set_status_label(label);
    
    // Show ready message initially
    log_info("Ready");
}

/// Log an info message to the status bar
pub fn log_info(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Info);
    // Also print to console for debugging
    println!("[INFO] {}", message);
}

/// Log a warning message to the status bar
pub fn log_warning(message: &str) {
    add_message_to_log(message.to_string(), LogLevel::Warning);
    // Also print to console for debugging
    eprintln!("[WARNING] {}", message);
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
    STATUS_HISTORY.lock()
        .map(|history| history.iter().cloned().collect())
        .unwrap_or_default()
}

/// Clear the log history
pub fn clear_log_history() {
    if let Ok(mut history) = STATUS_HISTORY.lock() {
        history.clear();
    }
    // Show ready message
    log_info("Ready");
}
