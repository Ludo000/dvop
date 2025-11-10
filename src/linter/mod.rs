// Linter module for code quality checking
// This module provides linting functionality for various programming languages

pub mod diagnostics_panel;
pub mod rust_linter;
pub mod ui;

use std::path::Path;
use gtk4::prelude::*; // Import GTK prelude for text buffer/tag methods

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

use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use once_cell::sync::Lazy;

// Global storage for file diagnostics
static FILE_DIAGNOSTICS: Lazy<Arc<Mutex<HashMap<String, Vec<Diagnostic>>>>> =
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
    
    if diagnostics.is_empty() {
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
            let mut end_iter = if let (Some(end_line), Some(end_col)) = (diag.end_line, diag.end_column) {
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
                    let mut iter = start_iter.clone();
                    iter.forward_to_line_end();
                    iter
                }
            } else {
                // No end position specified, underline the whole line
                let mut iter = start_iter.clone();
                iter.forward_to_line_end();
                iter
            };
            
            buffer.apply_tag(tag, &start_iter, &end_iter);
        }
    }
}
