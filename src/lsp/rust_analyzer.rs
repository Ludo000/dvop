//! # Rust-Analyzer Manager — Per-Workspace LSP Client Pool
//!
//! Maintains a `HashMap<PathBuf, Arc<LspClient>>` mapping each Cargo
//! workspace root to its rust-analyzer instance. When a Rust file is opened,
//! `get_client()` looks up (or creates) the client for that workspace.
//!
//! The manager is stored in a global `OnceLock` so it can be accessed from
//! both the `RustDiagnosticsExtension` (native extension) and the linter UI.
//!
//! ## Workspace Detection
//!
//! `find_workspace_root()` walks up the directory tree looking for
//! `Cargo.toml` to determine the workspace root, which rust-analyzer needs
//! as its `rootUri`.
//!
//! See FEATURES.md: Feature #41 — Rust-Analyzer Integration

// Rust-analyzer language server integration
// Provides Rust-specific LSP functionality

use super::client::LspClient;
use super::LanguageServerConfig;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Manages rust-analyzer client instances, one per Cargo workspace.
///
/// Uses `Arc<Mutex<HashMap<PathBuf, Arc<LspClient>>>>` so the manager can
/// be shared safely between the extension system and the linter UI thread.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_analyzer_manager_creation() {
        let manager = RustAnalyzerManager::new();
        let clients = manager.clients.lock().unwrap();
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_rust_analyzer_manager_default() {
        let manager = RustAnalyzerManager::default();
        let clients = manager.clients.lock().unwrap();
        assert_eq!(clients.len(), 0);
    }

    #[test]
    fn test_rust_analyzer_shutdown_empty() {
        let manager = RustAnalyzerManager::new();
        
        // Should not panic when shutting down with no clients
        manager.shutdown();
        
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
}
