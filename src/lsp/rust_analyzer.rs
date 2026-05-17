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
    // Arc<Mutex<T>> provides thread-safe shared mutable state. Arc for multiple owners across threads, Mutex for locking.
    clients: Arc<Mutex<HashMap<PathBuf, Arc<LspClient>>>>,
}

// "impl" blocks define methods and behavior for a struct or enum.
impl RustAnalyzerManager {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn new() -> Self {
        Self {
            // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
            clients: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Shutdown all rust-analyzer clients
    pub fn shutdown(&self) {
        // App quit path — tear down every pooled workspace subprocess so nothing outlives the GTK teardown.
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
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
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let mut clients = self.clients.lock().unwrap();

        if let Some(client) = clients.get(&workspace_root) {
            // Same Cargo workspace → one RA process — new files under that root only add `did_open` traffic, not another child.
            return Ok(client.clone()); // `Arc` clone is cheap — shares the existing LSP process
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
        // Spawns stdout reader thread — pairs with blocking `initialize()` handshake on this caller thread.
        client.start_message_loop();

        let client_arc = Arc::new(client);
        clients.insert(workspace_root, client_arc.clone());

        Ok(client_arc) // one subprocess per unique Cargo workspace root
    }

    /// Check if rust-analyzer is available in PATH
    fn is_rust_analyzer_available() -> bool {
        // Cheap `PATH` probe — `get_client` returns a clear error instead of a failed spawn deep in `LspClient::new`.
        std::process::Command::new("rust-analyzer")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }
}

// "impl" blocks define methods and behavior for a struct or enum.
impl Default for RustAnalyzerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "../../tests/unit/lsp/rust_analyzer_tests.rs"]
mod tests;
