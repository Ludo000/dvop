//! # LSP Module — Language Server Protocol Client
//!
//! Implements the client side of the [Language Server Protocol](https://microsoft.github.io/language-server-protocol/)
//! (LSP), enabling real-time diagnostics, hover info, and code intelligence
//! from external language servers (currently only rust-analyzer).
//!
//! ## Architecture
//!
//! - **`client.rs`** — Generic `LspClient` that speaks JSON-RPC 2.0 over
//!   stdio to any language server. Handles `initialize`, `shutdown`,
//!   `textDocument/didOpen`, `didChange`, `didSave`, and listens for
//!   `textDocument/publishDiagnostics` notifications.
//! - **`rust_analyzer.rs`** — `RustAnalyzerManager` that manages one
//!   `LspClient` per Cargo workspace root. Auto-starts rust-analyzer when
//!   a Rust file is opened.
//! - **`mod.rs` (this file)** — `convert_lsp_diagnostic()` translates
//!   `lsp_types::Diagnostic` into our internal `linter::Diagnostic`.
//!
//! ## JSON-RPC Protocol
//!
//! LSP uses JSON-RPC 2.0 with HTTP-style `Content-Length` headers over
//! stdin/stdout. The `LspClient` spawns the server process and reads
//! responses on a background thread (`start_message_loop`).
//!
//! See FEATURES.md: Feature #41 — Rust-Analyzer Integration
//! See FEATURES.md: Feature #47 — Real-Time Diagnostics

// LSP (Language Server Protocol) client implementation
// This module provides language server integration for enhanced code intelligence

pub mod client;
pub mod rust_analyzer;

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity};

/// Converts an `lsp_types::Diagnostic` into the app’s internal `Diagnostic`.
///
/// LSP uses 0-based line/column numbers; our `Diagnostic` uses 1-based, so
/// this function adds 1 to both. The `code` field is mapped to `rule`.
pub fn convert_lsp_diagnostic(lsp_diag: &LspDiagnostic) -> crate::linter::Diagnostic {
    use crate::linter::{Diagnostic, DiagnosticSeverity};

    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    // Single choke point for index bases + severity mapping — keeps rust-analyzer callbacks agnostic of GtkSource conventions.
    // `lsp_types` mirrors the JSON spec; we collapse HINT into Info for our simpler three-level UI.
    let severity = match lsp_diag.severity {
        Some(LspSeverity::ERROR) => DiagnosticSeverity::Error,
        Some(LspSeverity::WARNING) => DiagnosticSeverity::Warning,
        Some(LspSeverity::INFORMATION) | Some(LspSeverity::HINT) => DiagnosticSeverity::Info,
        None => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Info,
    };

    // LSP range is UTF-16 code units on the wire; we cast to usize for our 1-based line/col display.
    // GtkSourceView navigation uses byte indices in places — extreme Unicode edge cases may diverge slightly from strict LSP columns.
    let line = lsp_diag.range.start.line as usize;
    let column = lsp_diag.range.start.character as usize;
    let end_line = lsp_diag.range.end.line as usize;
    let end_column = lsp_diag.range.end.character as usize;

    let rule = lsp_diag
        .code
        .as_ref()
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        .map(|c| match c {
            lsp_types::NumberOrString::Number(n) => n.to_string(),
            lsp_types::NumberOrString::String(s) => s.clone(),
        })
        .unwrap_or_else(|| "lsp_diagnostic".to_string());

    Diagnostic::new(
        severity,
        lsp_diag.message.clone(),
        line + 1, // LSP uses 0-indexed, we use 1-indexed
        column + 1,
        rule,
    )
    .with_end_position(end_line + 1, end_column + 1)
}

/// Language server configuration
#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct LanguageServerConfig {
    pub name: String,
    pub command: String,
    pub args: Vec<String>,
    pub file_extensions: Vec<String>,
}

// "impl" blocks define methods and behavior for a struct or enum.
impl LanguageServerConfig {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn rust_analyzer() -> Self {
        // Spawn bare `rust-analyzer` from PATH — `build.rs` tries to rustup-install the component; extend `args` if you need unstable flags.
        Self {
            name: "rust-analyzer".to_string(),
            command: "rust-analyzer".to_string(),
            args: vec![],
            file_extensions: vec!["rs".to_string()],
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/lsp/mod_tests.rs"]
mod tests;
