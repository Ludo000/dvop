// Rust-analyzer language server integration
// Provides Rust-specific LSP functionality

use super::client::LspClient;
use super::LanguageServerConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Manager for rust-analyzer instances
pub struct RustAnalyzerManager {
    clients: Arc<Mutex<HashMap<PathBuf, Arc<LspClient>>>>,
}

impl RustAnalyzerManager {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Shutdown all rust-analyzer clients
    pub fn shutdown(&self) {
        let mut clients = self.clients.lock().unwrap();
        for (workspace, client) in clients.drain() {
            println!(
                "🛑 Shutting down rust-analyzer for workspace: {:?}",
                workspace
            );
            let _ = client.shutdown();
        }
    }

    /// Get or create a rust-analyzer client for the given workspace
    pub fn get_client(&self, workspace_root: PathBuf) -> Result<Arc<LspClient>, String> {
        let mut clients = self.clients.lock().unwrap();

        if let Some(client) = clients.get(&workspace_root) {
            return Ok(client.clone());
        }

        // Check if rust-analyzer is available
        if !Self::is_rust_analyzer_available() {
            return Err("rust-analyzer not found in PATH".to_string());
        }

        let config = LanguageServerConfig::rust_analyzer();
        let client = LspClient::new(&config.command, &config.args, workspace_root.clone())?;

        // Initialize the client
        client.initialize()?;

        // Start message loop
        client.start_message_loop();

        let client_arc = Arc::new(client);
        clients.insert(workspace_root, client_arc.clone());

        Ok(client_arc)
    }

    /// Check if rust-analyzer is available in PATH
    fn is_rust_analyzer_available() -> bool {
        std::process::Command::new("rust-analyzer")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }
}

impl Default for RustAnalyzerManager {
    fn default() -> Self {
        Self::new()
    }
}
