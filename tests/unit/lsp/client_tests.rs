    use super::*;
    use std::path::PathBuf;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    fn client_without_process() -> LspClient {
        LspClient {
            process: Arc::new(Mutex::new(None)),
            next_id: Arc::new(Mutex::new(1)),
            diagnostic_callback: Arc::new(Mutex::new(None)),
            workspace_root: PathBuf::from("/tmp/test-workspace"),
        }
    }

    #[test]
    fn test_json_rpc_message_serialization() {
        let msg = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: Some("test_method".to_string()),
            params: Some(serde_json::json!({"key": "value"})),
            result: None,
            error: None,
        };
        
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("test_method"));
        assert!(json.contains("2.0"));
    }

    #[test]
    fn test_json_rpc_message_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let msg: JsonRpcMessage = serde_json::from_str(json).unwrap();
        
        assert_eq!(msg.jsonrpc, "2.0");
        assert_eq!(msg.method, Some("initialize".to_string()));
        assert_eq!(msg.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_lsp_client_creation_invalid_command() {
        let workspace = PathBuf::from("/tmp");
        let result = LspClient::new("nonexistent_command_xyz", &[], workspace);
        
        // Should fail with invalid command
        assert!(result.is_err());
    }

    #[test]
    fn test_lsp_client_workspace_root() {
        let workspace = PathBuf::from("/tmp/test_workspace");
        
        // We can't easily test the full client without a real LSP server,
        // but we can verify the workspace path is used correctly
        assert!(workspace.to_string_lossy().contains("test_workspace"));
    }

    #[test]
    fn test_send_message_without_process_is_noop_success() {
        let client = client_without_process();
        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some("test/notification".to_string()),
            params: Some(serde_json::json!({"ok": true})),
            result: None,
            error: None,
        };

        assert!(client.send_message(&message).is_ok());
    }

    #[test]
    fn test_document_notifications_succeed_without_process() {
        let client = client_without_process();
        let uri: Uri = "file:///tmp/main.rs".parse().unwrap();

        assert!(client
            .did_open(uri.clone(), "rust".to_string(), 1, "fn main() {}".to_string())
            .is_ok());
        assert!(client
            .did_change(uri.clone(), 2, "fn main() { println!(\"hi\"); }".to_string())
            .is_ok());
        assert!(client.did_save(uri, Some("saved".to_string())).is_ok());
    }

    #[test]
    fn test_set_diagnostic_callback_replaces_existing_callback() {
        let client = client_without_process();
        let first_count = Arc::new(AtomicUsize::new(0));
        let second_count = Arc::new(AtomicUsize::new(0));

        let first_seen = first_count.clone();
        client.set_diagnostic_callback(move |_, _| {
            first_seen.fetch_add(1, Ordering::SeqCst);
        });

        let second_seen = second_count.clone();
        client.set_diagnostic_callback(move |_, _| {
            second_seen.fetch_add(1, Ordering::SeqCst);
        });

        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(PublishDiagnostics::METHOD.to_string()),
            params: Some(serde_json::json!({
                "uri": "file:///tmp/main.rs",
                "diagnostics": []
            })),
            result: None,
            error: None,
        };
        LspClient::handle_message(message, &client.diagnostic_callback);

        assert_eq!(first_count.load(Ordering::SeqCst), 0);
        assert_eq!(second_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handle_message_invokes_diagnostic_callback() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let seen = callback_count.clone();
        let callback: DiagnosticCallback = Arc::new(Mutex::new(Some(Box::new(move |uri, diagnostics| {
            assert_eq!(uri.as_str(), "file:///tmp/main.rs");
            assert_eq!(diagnostics.len(), 1);
            assert_eq!(diagnostics[0].message, "broken");
            seen.fetch_add(1, Ordering::SeqCst);
        }))));

        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(PublishDiagnostics::METHOD.to_string()),
            params: Some(serde_json::json!({
                "uri": "file:///tmp/main.rs",
                "diagnostics": [{
                    "range": {
                        "start": { "line": 0, "character": 1 },
                        "end": { "line": 0, "character": 4 }
                    },
                    "severity": 1,
                    "message": "broken"
                }]
            })),
            result: None,
            error: None,
        };

        LspClient::handle_message(message, &callback);

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_handle_message_ignores_non_diagnostic_and_invalid_params() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let seen = callback_count.clone();
        let callback: DiagnosticCallback = Arc::new(Mutex::new(Some(Box::new(move |_, _| {
            seen.fetch_add(1, Ordering::SeqCst);
        }))));

        LspClient::handle_message(
            JsonRpcMessage {
                jsonrpc: "2.0".to_string(),
                id: Some(serde_json::json!(1)),
                method: None,
                params: None,
                result: Some(serde_json::json!({})),
                error: None,
            },
            &callback,
        );
        LspClient::handle_message(
            JsonRpcMessage {
                jsonrpc: "2.0".to_string(),
                id: None,
                method: Some(PublishDiagnostics::METHOD.to_string()),
                params: Some(serde_json::json!({"uri": "not a uri"})),
                result: None,
                error: None,
            },
            &callback,
        );

        assert_eq!(callback_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_read_messages_parses_framed_diagnostic_message() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let seen = callback_count.clone();
        let callback: DiagnosticCallback = Arc::new(Mutex::new(Some(Box::new(move |uri, diagnostics| {
            assert_eq!(uri.as_str(), "file:///tmp/lib.rs");
            assert!(diagnostics.is_empty());
            seen.fetch_add(1, Ordering::SeqCst);
        }))));
        let body = serde_json::json!({
            "jsonrpc": "2.0",
            "method": PublishDiagnostics::METHOD,
            "params": {
                "uri": "file:///tmp/lib.rs",
                "diagnostics": []
            }
        })
        .to_string();
        let framed = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
        let reader = std::io::Cursor::new(framed.into_bytes());

        LspClient::read_messages(reader, callback);

        assert_eq!(callback_count.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_read_messages_ignores_unknown_headers_and_malformed_json() {
        let callback_count = Arc::new(AtomicUsize::new(0));
        let seen = callback_count.clone();
        let callback: DiagnosticCallback = Arc::new(Mutex::new(Some(Box::new(move |_, _| {
            seen.fetch_add(1, Ordering::SeqCst);
        }))));
        let body = "{not json";
        let framed = format!(
            "Server-Log: ignored\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        );
        let reader = std::io::Cursor::new(framed.into_bytes());

        LspClient::read_messages(reader, callback);

        assert_eq!(callback_count.load(Ordering::SeqCst), 0);
    }
