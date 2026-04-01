//! # Linter Module — Code Quality Diagnostics
//!
//! Provides in-editor diagnostics (errors, warnings, info) from two sources:
//!
//! 1. **Built-in linters** — `lint_file()` dispatches on file extension to
//!    language-specific checkers (currently `.ui` files via `gtk_ui_linter`).
//! 2. **Extension linters** — script extensions can define `linter` contributions
//!    that output JSON diagnostics (see `extensions/hooks.rs`).
//! 3. **LSP diagnostics** — rust-analyzer publishes diagnostics via the LSP
//!    protocol; they are stored here via `store_file_diagnostics()`.
//!
//! All diagnostics converge into the `Diagnostic` struct and are rendered as:
//! - Wavy underlines in the editor buffer (`apply_diagnostic_underlines()`)
//! - A clickable list in the diagnostics panel (`diagnostics_panel.rs`)
//! - A summary count in the status bar
//!
//! See FEATURES.md: Feature #47 — Real-Time Diagnostics
//! See FEATURES.md: Feature #48 — Inline Error Highlighting
//! See FEATURES.md: Feature #49 — Diagnostics Panel

pub mod diagnostics_panel;
pub mod gtk_ui_linter;
pub mod ui;

use std::path::Path;
use gtk4::prelude::*; // Import GTK prelude for text buffer/tag methods

/// Severity level for a diagnostic — determines the underline color and icon.
///
/// - `Error` → red wavy underline, ❌ icon
/// - `Warning` → yellow wavy underline, ⚠️ icon
/// - `Info` → blue underline, ℹ️ icon
#[derive(Debug, Clone, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

/// A single diagnostic message with location information.
///
/// This is the common currency for all diagnostic sources (built-in linters,
/// extension linters, LSP). The `line`/`column` fields are 1-based to match
/// editor line numbers. `end_line`/`end_column` are optional — when present,
/// the underline spans from `(line, column)` to `(end_line, end_column)`.
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
    let mut diagnostics = lint_file_builtin(file_path, content);

    // Append diagnostics from extension linters
    let ext_diags = crate::extensions::hooks::run_extension_linters(file_path);
    diagnostics.extend(ext_diags);

    diagnostics
}

/// Run only built-in (fast, local) linters — no extension subprocesses.
/// Safe to call on the GTK main thread.
pub fn lint_file_builtin(file_path: &Path, content: &str) -> Vec<Diagnostic> {
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        match ext {
            "rs" => Vec::new(), // Rust files use rust-analyzer via LSP, not local linter
            "ui" => gtk_ui_linter::lint_gtk_ui(content),
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    }
}

/// Run linter for a specific language
pub fn lint_by_language(language: &str, _content: &str) -> Vec<Diagnostic> {
    match language.to_lowercase().as_str() {
        // Rust diagnostics are handled by the rust-diagnostics extension (via rust-analyzer LSP)
        // Add more languages here
        _ => Vec::new(),
    }
}

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Type alias for diagnostics storage
type DiagnosticsMap = Arc<Mutex<HashMap<String, Vec<Diagnostic>>>>;

// Global storage for file diagnostics
static FILE_DIAGNOSTICS: Lazy<DiagnosticsMap> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Store diagnostics for a file
pub fn store_file_diagnostics(file_path: &str, diagnostics: Vec<Diagnostic>) {
    if let Ok(mut map) = FILE_DIAGNOSTICS.lock() {
        if diagnostics.is_empty() {
            map.remove(file_path);
        } else {
            map.insert(file_path.to_string(), diagnostics);
        }
    }
}

/// Get diagnostics for a file
pub fn get_file_diagnostics(file_path: &str) -> Vec<Diagnostic> {
    FILE_DIAGNOSTICS
        .lock()
        .ok()
        .and_then(|map| map.get(file_path).cloned())
        .unwrap_or_default()
}

/// Apply diagnostic underlines to a source buffer
pub fn apply_diagnostic_underlines(buffer: &sourceview5::Buffer, file_path: &str) {
    let mut diagnostics = get_file_diagnostics(file_path);
    
    println!("🖊️  apply_diagnostic_underlines for {}: {} diagnostics", file_path, diagnostics.len());
    
    let tag_table = buffer.tag_table();
    
    // Clear all existing diagnostic tags first
    let start_iter = buffer.start_iter();
    let end_iter = buffer.end_iter();
    
    if let Some(tag) = tag_table.lookup("diagnostic-info-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }
    if let Some(tag) = tag_table.lookup("diagnostic-warning-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }
    if let Some(tag) = tag_table.lookup("diagnostic-error-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }
    
    println!("✓ Cleared all diagnostic tags");
    
    if diagnostics.is_empty() {
        println!("✓ No diagnostics to apply, returning");
        return;
    }
    
    // Sort diagnostics by severity (Info > Warning > Error) so more severe ones are applied last and take precedence
    diagnostics.sort_by(|a, b| {
        let severity_value = |s: &DiagnosticSeverity| match s {
            DiagnosticSeverity::Info => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Error => 2,
        };
        severity_value(&a.severity).cmp(&severity_value(&b.severity))
    });
    
    let tag_table = buffer.tag_table();
    
    // Create or get tags for each severity level with proper priorities
    let info_tag = if let Some(tag) = tag_table.lookup("diagnostic-info-underline") {
        tag
    } else {
        let tag = gtk4::TextTag::new(Some("diagnostic-info-underline"));
        tag.set_underline(gtk4::pango::Underline::Error); // Wavy underline for better visibility
        tag.set_underline_rgba(Some(&gtk4::gdk::RGBA::new(0.2, 0.8, 1.0, 1.0))); // Brighter blue #33ccff
        tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(0.2, 0.8, 1.0, 0.15))); // Semi-transparent blue background
        tag_table.add(&tag);
        tag.set_priority(tag_table.size() - 1); // Set priority after adding
        tag
    };
    
    let warning_tag = if let Some(tag) = tag_table.lookup("diagnostic-warning-underline") {
        tag
    } else {
        let tag = gtk4::TextTag::new(Some("diagnostic-warning-underline"));
        tag.set_underline(gtk4::pango::Underline::Error);
        tag.set_underline_rgba(Some(&gtk4::gdk::RGBA::new(1.0, 0.8, 0.0, 1.0))); // Brighter orange #ffcc00
        tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(1.0, 0.8, 0.0, 0.15))); // Semi-transparent orange background
        tag_table.add(&tag);
        tag.set_priority(tag_table.size() - 1); // Set priority after adding (higher than info)
        tag
    };
    
    let error_tag = if let Some(tag) = tag_table.lookup("diagnostic-error-underline") {
        tag
    } else {
        let tag = gtk4::TextTag::new(Some("diagnostic-error-underline"));
        tag.set_underline(gtk4::pango::Underline::Error);
        tag.set_underline_rgba(Some(&gtk4::gdk::RGBA::new(1.0, 0.2, 0.2, 1.0))); // Brighter red #ff3333
        tag.set_background_rgba(Some(&gtk4::gdk::RGBA::new(1.0, 0.2, 0.2, 0.15))); // Semi-transparent red background
        tag_table.add(&tag);
        tag.set_priority(tag_table.size() - 1); // Set priority after adding (highest)
        tag
    };
    
    // Apply tags for each diagnostic (sorted by severity, so errors override warnings/info)
    for diag in diagnostics {
        let tag = match diag.severity {
            DiagnosticSeverity::Error => &error_tag,
            DiagnosticSeverity::Warning => &warning_tag,
            DiagnosticSeverity::Info => &info_tag,
        };
        
        // Convert to 0-based indexing
        let line = if diag.line > 0 { diag.line - 1 } else { 0 };
        let col = if diag.column > 0 { diag.column - 1 } else { 0 };
        
        // Get start iterator
        if let Some(mut start_iter) = buffer.iter_at_line(line as i32) {
            // Move to column
            for _ in 0..col {
                if !start_iter.forward_char() {
                    break;
                }
            }
            
            // Determine end position
            let end_iter = if let (Some(end_line), Some(end_col)) = (diag.end_line, diag.end_column) {
                let end_line_idx = if end_line > 0 { end_line - 1 } else { 0 };
                let end_col_idx = if end_col > 0 { end_col - 1 } else { 0 };
                
                if let Some(mut iter) = buffer.iter_at_line(end_line_idx as i32) {
                    for _ in 0..end_col_idx {
                        if !iter.forward_char() {
                            break;
                        }
                    }
                    iter
                } else {
                    let mut iter = start_iter;
                    iter.forward_to_line_end();
                    iter
                }
            } else {
                // No end position specified, underline the whole line
                let mut iter = start_iter;
                iter.forward_to_line_end();
                iter
            };
            
            buffer.apply_tag(tag, &start_iter, &end_iter);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_creation() {
        let diag = Diagnostic {
            message: "Test error".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 10,
            column: 5,
            end_line: Some(10),
            end_column: Some(15),
            rule: "E001".to_string(),
        };

        assert_eq!(diag.message, "Test error");
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.line, 10);
        assert_eq!(diag.column, 5);
    }

    #[test]
    fn test_diagnostic_severity_levels() {
        let error = DiagnosticSeverity::Error;
        let warning = DiagnosticSeverity::Warning;
        let info = DiagnosticSeverity::Info;

        assert_eq!(error, DiagnosticSeverity::Error);
        assert_eq!(warning, DiagnosticSeverity::Warning);
        assert_eq!(info, DiagnosticSeverity::Info);
    }

    #[test]
    fn test_store_and_get_file_diagnostics() {
        let path = "/test/file.rs";
        let diag = vec![Diagnostic {
            message: "Test diagnostic".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 5,
            column: 10,
            end_line: Some(5),
            end_column: Some(20),
            rule: "E001".to_string(),
        }];

        store_file_diagnostics(path, diag);
        let retrieved = get_file_diagnostics(path);

        assert_eq!(retrieved.len(), 1);
        assert_eq!(retrieved[0].message, "Test diagnostic");
    }

    #[test]
    fn test_clear_file_diagnostics() {
        let path = "/test/clear.rs";
        let diag = vec![Diagnostic {
            message: "Test".to_string(),
            severity: DiagnosticSeverity::Error,
            line: 1,
            column: 1,
            end_line: None,
            end_column: None,
            rule: "E".to_string(),
        }];

        store_file_diagnostics(path, diag);
        assert_eq!(get_file_diagnostics(path).len(), 1);

        store_file_diagnostics(path, vec![]);
        assert_eq!(get_file_diagnostics(path).len(), 0);
    }

    #[test]
    fn test_empty_file_diagnostics() {
        let path = "/test/empty.rs";
        let diagnostics = get_file_diagnostics(path);
        assert_eq!(diagnostics.len(), 0);
    }

    #[test]
    fn test_lint_by_language() {
        let rust_code = "fn main() { }";
        let diagnostics = lint_by_language("rust", rust_code);
        // Rust diagnostics are now handled by the extension, so this returns empty
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_lint_file_rs_defers_to_extension() {
        // Rust files return empty from the local linter —
        // real diagnostics come from the rust-diagnostics extension via LSP
        let path = Path::new("test.rs");
        let diagnostics = lint_file(path, "fn main() { let x = 5 }");
        assert!(diagnostics.is_empty(), "Local linter should return empty for .rs files");
    }

    #[test]
    fn test_lint_file_unknown_extension() {
        // Unknown file extensions should return empty (no linter registered)
        let path = Path::new("file.xyz");
        let diagnostics = lint_file(path, "some content");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_diagnostic_with_end_position() {
        let diag = Diagnostic::new(
            DiagnosticSeverity::Warning,
            "unused variable".to_string(),
            5, 10,
            "W001".to_string(),
        ).with_end_position(5, 15);

        assert_eq!(diag.end_line, Some(5));
        assert_eq!(diag.end_column, Some(15));
        assert_eq!(diag.severity, DiagnosticSeverity::Warning);
    }

    #[test]
    fn test_diagnostic_new_constructor() {
        let diag = Diagnostic::new(
            DiagnosticSeverity::Info,
            "hint message".to_string(),
            1, 1,
            "I001".to_string(),
        );
        assert_eq!(diag.line, 1);
        assert_eq!(diag.column, 1);
        assert_eq!(diag.end_line, None);
        assert_eq!(diag.end_column, None);
        assert_eq!(diag.rule, "I001");
    }
}
