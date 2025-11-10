// LSP client implementation
// Handles communication with language servers via stdio

use lsp_types::{
    notification::{DidChangeTextDocument, DidOpenTextDocument, Notification, PublishDiagnostics},
    request::{Initialize, Request},
    Uri, *,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};

/// JSON-RPC message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<serde_json::Value>,
}

/// LSP client for communicating with a language server
pub struct LspClient {
    process: Arc<Mutex<Option<Child>>>,
    next_id: Arc<Mutex<i32>>,
    diagnostic_callback: Arc<Mutex<Option<Box<dyn Fn(Uri, Vec<Diagnostic>) + Send + 'static>>>>,
    workspace_root: PathBuf,
}

impl LspClient {
    /// Create a new LSP client for the given command
    pub fn new(command: &str, args: &[String], workspace_root: PathBuf) -> Result<Self, String> {
        println!("🚀 Starting LSP process: {} with args: {:?}", command, args);
        println!("Workspace root: {:?}", workspace_root);

        let mut process = Command::new(command)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to start language server: {}", e))?;

        println!("✓ LSP process started with PID: {:?}", process.id());

        // Capture stderr in a separate thread for debugging
        if let Some(stderr) = process.stderr.take() {
            std::thread::spawn(move || {
                let reader = BufReader::new(stderr);
                for line in reader.lines() {
                    if let Ok(line) = line {
                        eprintln!("[rust-analyzer stderr] {}", line);
                    }
                }
            });
        }

        Ok(Self {
            process: Arc::new(Mutex::new(Some(process))),
            next_id: Arc::new(Mutex::new(1)),
            diagnostic_callback: Arc::new(Mutex::new(None)),
            workspace_root,
        })
    }

    /// Initialize the language server
    pub fn initialize(&self) -> Result<(), String> {
        let workspace_url = url::Url::from_file_path(&self.workspace_root)
            .map_err(|_| "Failed to create workspace URL")?;
        let workspace_uri = workspace_url
            .as_str()
            .parse::<Uri>()
            .map_err(|e| format!("Failed to parse URI: {}", e))?;

        let init_params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: Some(workspace_uri.clone()),
            #[allow(deprecated)]
            root_path: None,
            initialization_options: None,
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities {
                    synchronization: Some(TextDocumentSyncClientCapabilities {
                        dynamic_registration: Some(false),
                        will_save: Some(false),
                        will_save_wait_until: Some(false),
                        did_save: Some(false),
                    }),
                    publish_diagnostics: Some(PublishDiagnosticsClientCapabilities {
                        related_information: Some(true),
                        tag_support: None,
                        version_support: Some(true),
                        code_description_support: Some(false),
                        data_support: Some(false),
                    }),
                    ..Default::default()
                }),
                workspace: Some(WorkspaceClientCapabilities {
                    apply_edit: Some(false),
                    workspace_edit: None,
                    did_change_configuration: None,
                    did_change_watched_files: None,
                    symbol: None,
                    execute_command: None,
                    workspace_folders: Some(true),
                    configuration: Some(false),
                    semantic_tokens: None,
                    code_lens: None,
                    file_operations: None,
                    inline_value: None,
                    inlay_hint: None,
                    diagnostic: None,
                }),
                ..Default::default()
            },
            trace: Some(TraceValue::Off),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: workspace_uri,
                name: self
                    .workspace_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("workspace")
                    .to_string(),
            }]),
            client_info: Some(ClientInfo {
                name: "dvop".to_string(),
                version: Some("0.1.0".to_string()),
            }),
            locale: None,
            work_done_progress_params: lsp_types::WorkDoneProgressParams {
                work_done_token: None,
            },
        };

        let id = self.send_request::<Initialize>(init_params)?;
        println!("Sent initialize request with id: {}", id);

        // Send initialized notification
        self.send_notification::<lsp_types::notification::Initialized>(InitializedParams {})?;

        Ok(())
    }

    /// Shutdown the language server gracefully
    pub fn shutdown(&self) -> Result<(), String> {
        println!("🛑 Sending shutdown request to language server");

        // Send shutdown request (no params needed)
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let current_id = *next_id;
            *next_id += 1;
            current_id
        };

        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(id.into())),
            method: Some("shutdown".to_string()),
            params: Some(serde_json::Value::Null),
            result: None,
            error: None,
        };

        let _ = self.send_message(&message);

        // Send exit notification
        let exit_message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some("exit".to_string()),
            params: None,
            result: None,
            error: None,
        };

        let _ = self.send_message(&exit_message);

        // Kill the process
        let mut process = self.process.lock().unwrap();
        if let Some(ref mut child) = *process {
            let _ = child.kill();
            let _ = child.wait();
            println!("✓ Language server process terminated");
        }
        *process = None;

        Ok(())
    }

    /// Send a request to the language server
    fn send_request<R: Request>(&self, params: R::Params) -> Result<i32, String>
    where
        R::Params: Serialize,
    {
        let id = {
            let mut next_id = self.next_id.lock().unwrap();
            let current_id = *next_id;
            *next_id += 1;
            current_id
        };

        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::Value::Number(id.into())),
            method: Some(R::METHOD.to_string()),
            params: Some(serde_json::to_value(params).map_err(|e| e.to_string())?),
            result: None,
            error: None,
        };

        self.send_message(&message)?;
        Ok(id)
    }

    /// Send a notification to the language server
    fn send_notification<N: Notification>(&self, params: N::Params) -> Result<(), String>
    where
        N::Params: Serialize,
    {
        let message = JsonRpcMessage {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: Some(N::METHOD.to_string()),
            params: Some(serde_json::to_value(params).map_err(|e| e.to_string())?),
            result: None,
            error: None,
        };

        self.send_message(&message)
    }

    /// Send a JSON-RPC message to the language server
    fn send_message(&self, message: &JsonRpcMessage) -> Result<(), String> {
        let json = serde_json::to_string(message).map_err(|e| e.to_string())?;
        let content = format!("Content-Length: {}\r\n\r\n{}", json.len(), json);

        let mut process = self.process.lock().unwrap();
        if let Some(ref mut child) = *process {
            if let Some(ref mut stdin) = child.stdin {
                stdin
                    .write_all(content.as_bytes())
                    .map_err(|e| format!("Failed to write to language server: {}", e))?;
                stdin
                    .flush()
                    .map_err(|e| format!("Failed to flush: {}", e))?;
                println!(
                    "Sent LSP message: {}",
                    message.method.as_ref().unwrap_or(&"response".to_string())
                );
            }
        }

        Ok(())
    }

    /// Notify the server that a document was opened
    pub fn did_open(
        &self,
        uri: Uri,
        language_id: String,
        version: i32,
        text: String,
    ) -> Result<(), String> {
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri,
                language_id,
                version,
                text,
            },
        };

        self.send_notification::<DidOpenTextDocument>(params)
    }

    /// Notify the server that a document was changed
    pub fn did_change(&self, uri: Uri, version: i32, text: String) -> Result<(), String> {
        let params = DidChangeTextDocumentParams {
            text_document: VersionedTextDocumentIdentifier { uri, version },
            content_changes: vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        };

        self.send_notification::<DidChangeTextDocument>(params)
    }

    /// Notify the server that a document was saved
    pub fn did_save(&self, uri: Uri, text: Option<String>) -> Result<(), String> {
        let params = DidSaveTextDocumentParams {
            text_document: TextDocumentIdentifier { uri },
            text,
        };

        self.send_notification::<lsp_types::notification::DidSaveTextDocument>(params)
    }

    /// Set the callback for receiving diagnostics
    pub fn set_diagnostic_callback<F>(&self, callback: F)
    where
        F: Fn(Uri, Vec<Diagnostic>) + Send + 'static,
    {
        let mut cb = self.diagnostic_callback.lock().unwrap();
        *cb = Some(Box::new(callback));
    }

    /// Start listening for messages from the language server
    pub fn start_message_loop(&self) {
        let process = self.process.clone();
        let diagnostic_callback = self.diagnostic_callback.clone();

        std::thread::spawn(move || {
            let mut process_guard = process.lock().unwrap();
            if let Some(ref mut child) = *process_guard {
                if let Some(stdout) = child.stdout.take() {
                    drop(process_guard); // Release lock before long-running loop

                    let reader = BufReader::new(stdout);
                    Self::read_messages(reader, diagnostic_callback);
                }
            }
        });
    }

    /// Read and parse messages from the language server
    fn read_messages<R: BufRead>(
        mut reader: R,
        diagnostic_callback: Arc<Mutex<Option<Box<dyn Fn(Uri, Vec<Diagnostic>) + Send + 'static>>>>,
    ) {
        loop {
            // Read Content-Length header
            let mut header_line = String::new();
            if reader.read_line(&mut header_line).is_err() {
                break;
            }

            if header_line.trim().is_empty() {
                continue;
            }

            let content_length: usize =
                if let Some(len_str) = header_line.strip_prefix("Content-Length: ") {
                    len_str.trim().parse().unwrap_or(0)
                } else {
                    continue;
                };

            // Read empty line
            let mut empty_line = String::new();
            if reader.read_line(&mut empty_line).is_err() {
                break;
            }

            // Read content
            let mut content = vec![0u8; content_length];
            if reader.read_exact(&mut content).is_err() {
                break;
            }

            let content_str = String::from_utf8_lossy(&content);

            // Parse JSON-RPC message
            if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(&content_str) {
                Self::handle_message(message, &diagnostic_callback);
            }
        }
    }

    /// Handle a received message from the language server
    fn handle_message(
        message: JsonRpcMessage,
        diagnostic_callback: &Arc<
            Mutex<Option<Box<dyn Fn(Uri, Vec<Diagnostic>) + Send + 'static>>>,
        >,
    ) {
        if let Some(method) = &message.method {
            if method == PublishDiagnostics::METHOD {
                if let Some(params) = message.params {
                    if let Ok(diag_params) =
                        serde_json::from_value::<PublishDiagnosticsParams>(params)
                    {
                        println!(
                            "Received {} diagnostics for {:?}",
                            diag_params.diagnostics.len(),
                            diag_params.uri.as_str()
                        );

                        let cb = diagnostic_callback.lock().unwrap();
                        if let Some(ref callback) = *cb {
                            callback(diag_params.uri, diag_params.diagnostics);
                        }
                    }
                }
            }
        }
    }
}

impl Drop for LspClient {
    fn drop(&mut self) {
        let mut process = self.process.lock().unwrap();
        if let Some(mut child) = process.take() {
            let _ = child.kill();
        }
    }
}
