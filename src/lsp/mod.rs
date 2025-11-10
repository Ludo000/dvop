// LSP (Language Server Protocol) client implementation
// This module provides language server integration for enhanced code intelligence

pub mod client;
pub mod rust_analyzer;

use lsp_types::{Diagnostic as LspDiagnostic, DiagnosticSeverity as LspSeverity};

/// Convert LSP diagnostic to our internal diagnostic format
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
