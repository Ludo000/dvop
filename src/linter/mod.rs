// Linter module for code quality checking
// This module provides linting functionality for various programming languages

pub mod diagnostics_panel;
pub mod rust_linter;
pub mod ui;

use std::path::Path;

/// Represents a lint diagnostic (error, warning, or info)
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// Represents a single diagnostic from the linter
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub line: usize,
    pub column: usize,
    pub end_line: Option<usize>,
    pub end_column: Option<usize>,
    pub rule: String,
}

impl Diagnostic {
    pub fn new(
        severity: DiagnosticSeverity,
        message: String,
        line: usize,
        column: usize,
        rule: String,
    ) -> Self {
        Self {
            severity,
            message,
            line,
            column,
            end_line: None,
            end_column: None,
            rule,
        }
    }

    pub fn with_end_position(mut self, end_line: usize, end_column: usize) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Run linter for a specific file based on its language
pub fn lint_file(file_path: &Path, content: &str) -> Vec<Diagnostic> {
    // Determine language from file extension
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        match ext {
            "rs" => rust_linter::lint_rust_code(content),
            // Add more languages here as they are implemented
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

/// Run linter for a specific language
pub fn lint_by_language(language: &str, content: &str) -> Vec<Diagnostic> {
    match language.to_lowercase().as_str() {
        "rust" => rust_linter::lint_rust_code(content),
        // Add more languages here
        _ => Vec::new(),
    }
}
