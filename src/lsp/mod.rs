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

    let severity = match lsp_diag.severity {
        Some(LspSeverity::ERROR) => DiagnosticSeverity::Error,
        Some(LspSeverity::WARNING) => DiagnosticSeverity::Warning,
        Some(LspSeverity::INFORMATION) | Some(LspSeverity::HINT) => DiagnosticSeverity::Info,
        None => DiagnosticSeverity::Info,
        _ => DiagnosticSeverity::Info,
    };

    let line = lsp_diag.range.start.line as usize;
    let column = lsp_diag.range.start.character as usize;
    let end_line = lsp_diag.range.end.line as usize;
    let end_column = lsp_diag.range.end.character as usize;

    let rule = lsp_diag
        .code
        .as_ref()
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

impl LanguageServerConfig {
    pub fn rust_analyzer() -> Self {
        Self {
            name: "rust-analyzer".to_string(),
            command: "rust-analyzer".to_string(),
            args: vec![],
            file_extensions: vec!["rs".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_analyzer_config() {
        let config = LanguageServerConfig::rust_analyzer();
        assert_eq!(config.name, "rust-analyzer");
        assert_eq!(config.command, "rust-analyzer");
        assert_eq!(config.file_extensions, vec!["rs"]);
        assert!(config.args.is_empty());
    }

    #[test]
    fn test_lsp_diagnostic_conversion_error() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position {
                    line: 10,
                    character: 5,
                },
                end: Position {
                    line: 10,
                    character: 15,
                },
            },
            severity: Some(LspSeverity::ERROR),
            code: None,
            message: "Test error".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Error);
        assert_eq!(diag.line, 11); // 1-indexed
        assert_eq!(diag.column, 6); // 1-indexed
    }

    #[test]
    fn test_lsp_diagnostic_conversion_warning() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 5, character: 10 },
                end: Position { line: 5, character: 20 },
            },
            severity: Some(LspSeverity::WARNING),
            code: None,
            message: "Test warning".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_lsp_diagnostic_conversion_info() {
        use lsp_types::{Diagnostic as LspDiagnostic, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            severity: Some(LspSeverity::INFORMATION),
            code: None,
            message: "Test info".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.severity, crate::linter::DiagnosticSeverity::Info);
    }

    #[test]
    fn test_lsp_diagnostic_with_code() {
        use lsp_types::{Diagnostic as LspDiagnostic, NumberOrString, Position, Range};

        let lsp_diag = LspDiagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 5 },
            },
            severity: Some(LspSeverity::ERROR),
            code: Some(NumberOrString::String("E0425".to_string())),
            message: "Cannot find value".to_string(),
            source: None,
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let diag = convert_lsp_diagnostic(&lsp_diag);
        assert_eq!(diag.message, "Cannot find value");
    }
}
