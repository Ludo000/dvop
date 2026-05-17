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
