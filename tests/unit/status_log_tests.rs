//! Tests for status_log module.
//!
//! Tests are standard Rust practice to ensure the serialization 
//! and history logic doesn't break if someone changes a line of code.

use super::*;

#[ctor::ctor]
fn clear_log_history_before_tests() {
    if let Ok(mut history) = STATUS_HISTORY.lock() {
        history.clear();
    }
}

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
    let serialized = "1700000000|INFO|Test message";
    let msg = LogMessage::from_string(serialized).unwrap();
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
    let deserialized = LogMessage::from_string(&serialized).unwrap();
    assert_eq!(deserialized.message, original.message);
    assert_eq!(deserialized.level, original.level);
}

#[test]
fn test_log_message_deserialization_preserves_pipe_in_message() {
    let msg = LogMessage::from_string("1700000000|ERROR|path=a|reason=bad").unwrap();
    assert_eq!(msg.level, LogLevel::Error);
    assert_eq!(msg.message, "path=a|reason=bad");
}

#[test]
fn test_all_log_levels_serialize_to_expected_tokens() {
    let levels = [(LogLevel::Info, "INFO"), (LogLevel::Warning, "WARNING"), (LogLevel::Error, "ERROR"), (LogLevel::Success, "SUCCESS")];
    for (level, token) in levels {
        let msg = LogMessage { timestamp: std::time::UNIX_EPOCH, message: "message".to_string(), level };
        assert_eq!(msg.to_string(), format!("0|{}|message", token));
    }
}

#[test]
fn test_log_levels() {
    let info = LogMessage { timestamp: std::time::SystemTime::now(), message: "Info".to_string(), level: LogLevel::Info };
    assert_eq!(info.level, LogLevel::Info);
    let warning = LogMessage { timestamp: std::time::SystemTime::now(), message: "Warning".to_string(), level: LogLevel::Warning };
    assert_eq!(warning.level, LogLevel::Warning);
    let error = LogMessage { timestamp: std::time::SystemTime::now(), message: "Error".to_string(), level: LogLevel::Error };
    assert_eq!(error.level, LogLevel::Error);
    let success = LogMessage { timestamp: std::time::SystemTime::now(), message: "Success".to_string(), level: LogLevel::Success };
    assert_eq!(success.level, LogLevel::Success);
}

#[test]
fn test_invalid_log_message_deserialization() {
    assert!(LogMessage::from_string("invalid").is_none());
    assert!(LogMessage::from_string("only|two").is_none());
    assert!(LogMessage::from_string("abc|INFO|message").is_none());
    assert!(LogMessage::from_string("123|INVALID|message").is_none());
}

#[test]
fn test_log_file_path() {
    let log_path = get_log_file_path();
    assert!(log_path.is_absolute());
    assert!(log_path.to_str().unwrap().ends_with("log_history.txt"));
}

#[test]
fn test_max_log_history_constant() {
    assert_eq!(MAX_LOG_HISTORY, 100);
    assert!(MAX_LOG_HISTORY > 0);
}

#[test]
fn log_functions_append_to_history_without_registered_labels() {
    clear_log_history();
    let before = get_log_history().len();
    log_info("info message");
    log_error("error message");
    log_success("success message");
    let history = get_log_history();
    assert_eq!(history.len(), before + 3);
    assert!(history.iter().any(|m| m.message == "info message"));
    assert!(history.iter().any(|m| m.message == "error message"));
    assert!(history.iter().any(|m| m.message == "success message"));
    clear_log_history();
}

#[test]
fn log_message_deserialization_supports_warning_level() {
    let msg = LogMessage::from_string("1700000000|WARNING|disk almost full").unwrap();
    assert_eq!(msg.level, LogLevel::Warning);
    assert_eq!(msg.message, "disk almost full");
}

#[test]
fn log_message_deserialization_supports_success_level() {
    let msg = LogMessage::from_string("1700000000|SUCCESS|saved file").unwrap();
    assert_eq!(msg.level, LogLevel::Success);
    assert_eq!(msg.message, "saved file");
}

#[test]
fn clear_log_history_resets_to_ready_message() {
    clear_log_history();
    log_info("temporary entry");
    clear_log_history();
    let history = get_log_history();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].message, "Ready");
}

#[test]
fn log_error_appends_to_history() {
    clear_log_history();
    let before = get_log_history().len();
    log_error("something failed");
    assert!(get_log_history().len() > before);
    assert!(get_log_history().iter().any(|m| m.message == "something failed"));
}

#[test]
fn log_message_round_trips_pipe_in_message_body() {
    let original = LogMessage {
        timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_000),
        level: LogLevel::Info,
        message: "part|with|pipes".to_string(),
    };
    let serialized = original.to_string();
    let restored = LogMessage::from_string(&serialized).unwrap();
    assert_eq!(restored.message, "part|with|pipes");
    assert_eq!(restored.level, LogLevel::Info);
}

#[test]
fn log_message_to_string_and_from_string_round_trip() {
    let original = LogMessage {
        timestamp: std::time::UNIX_EPOCH + std::time::Duration::from_secs(1_700_000_123),
        level: LogLevel::Warning,
        message: "disk almost full".to_string(),
    };
    let restored = LogMessage::from_string(&original.to_string()).unwrap();
    assert_eq!(restored.level, LogLevel::Warning);
    assert_eq!(restored.message, "disk almost full");
}

#[test]
fn log_message_to_string_uses_expected_level_labels() {
    let cases = [(LogLevel::Info, "INFO"), (LogLevel::Warning, "WARNING"), (LogLevel::Error, "ERROR"), (LogLevel::Success, "SUCCESS")];
    for (level, label) in cases {
        let msg = LogMessage { timestamp: std::time::UNIX_EPOCH, message: "sample".to_string(), level };
        assert!(msg.to_string().contains(label));
    }
}