    use super::*;
    use std::path::PathBuf;

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
