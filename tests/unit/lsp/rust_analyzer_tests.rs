    use super::*;

    #[test]
    fn test_rust_analyzer_manager_creation() {
        let manager = RustAnalyzerManager::new();
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let clients = manager.clients.lock().unwrap();
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_rust_analyzer_manager_default() {
        let manager = RustAnalyzerManager::default();
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let clients = manager.clients.lock().unwrap();
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_rust_analyzer_shutdown_empty() {
        let manager = RustAnalyzerManager::new();
        
        // Should not panic when shutting down with no clients
        manager.shutdown();
        
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        let clients = manager.clients.lock().unwrap();
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_is_rust_analyzer_available() {
        // This will check if rust-analyzer is in PATH
        // The result depends on the system, so we just verify it doesn't panic
        let _available = RustAnalyzerManager::is_rust_analyzer_available();
        // Test passes if it doesn't crash
    }
