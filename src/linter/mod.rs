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
//!
//! ## Two places diagnostics are stored (important)
//!
//! This module owns **`FILE_DIAGNOSTICS`** (`store_file_diagnostics` / `get_file_diagnostics`):
//! a map from **filesystem path string** → diagnostics. That map feeds `apply_diagnostic_underlines`,
//! which paints squiggles in the `GtkSourceView` buffer.
//!
//! The **`linter::ui`** submodule also keeps **`DIAGNOSTICS_STORE`**, keyed by **LSP-style URI**
//! (`file:///...`). That copy drives the diagnostics side panel and status-bar counts. Producers such
//! as the Rust LSP callback update **both** stores so the panel and the editor agree. If you add a
//! new diagnostic source, follow the same pattern: update URI store for UI, path store for underlines
//! (or call the helpers in `ui.rs` that already do both).

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
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
    pub end_line: Option<usize>,
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
    pub end_column: Option<usize>,
    pub rule: String,
}

// "impl" blocks define methods and behavior for a struct or enum.
impl Diagnostic {
    // pub makes this function public, allowing it to be used from outside this module.
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

    // pub makes this function public, allowing it to be used from outside this module.
    pub fn with_end_position(mut self, end_line: usize, end_column: usize) -> Self {
        // Builder-style setter — LSP ranges fill this; omit for point diagnostics (single underline cell).
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Run linter for a specific file based on its language
#[allow(dead_code)]
// pub makes this function public, allowing it to be used from outside this module.
pub fn lint_file(file_path: &Path, content: &str) -> Vec<Diagnostic> {
    // Determine language from file extension
    let mut diagnostics = lint_file_builtin(file_path, content);

    // Append diagnostics from extension linters
    // Extension linters may spawn subprocesses — prefer calling `lint_file_builtin` from latency-sensitive GTK callbacks instead.
    // Script extensions may contribute JSON diagnostics — merged into same vec as built-ins.
    let ext_diags = crate::extensions::hooks::run_extension_linters(file_path);
    diagnostics.extend(ext_diags);

    diagnostics
}

/// Run only built-in (fast, local) linters — no extension subprocesses.
/// Safe to call on the GTK main thread.
pub fn lint_file_builtin(file_path: &Path, content: &str) -> Vec<Diagnostic> {
    if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
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
    // `run_linter` calls this when there is no file path (Untitled tabs, etc.) — only GtkSource’s language id is known, unlike `lint_file_builtin` which keys off `Path::extension`.
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    match language.to_lowercase().as_str() {
        // Rust diagnostics are handled by the rust-diagnostics extension (via rust-analyzer LSP)
        // Add more languages here
        _ => Vec::new(),
    }
}

use std::sync::{Arc, Mutex};
use std::collections::{HashMap, HashSet};
use once_cell::sync::Lazy;

/// Paths for which we have applied diagnostic underline tags (skip full-buffer tag clears when empty).
static FILES_WITH_APPLIED_UNDERLINE_TAGS: Lazy<Mutex<HashSet<String>>> =
    Lazy::new(|| Mutex::new(HashSet::new()));

fn underline_debug(msg: impl AsRef<str>) {
    if std::env::var("DVOP_DEBUG_UNDERLINES").ok().as_deref() == Some("1") {
        println!("{}", msg.as_ref());
    }
}

/// Call when a tab/file is closed so underline fast-path tracking does not leak.
pub fn forget_diagnostic_underline_tracking_for_path(file_path: &str) {
    // Without this, `apply_diagnostic_underlines` may skip clearing tags on the next empty-diagnostic update for a reused path.
    if let Ok(mut s) = FILES_WITH_APPLIED_UNDERLINE_TAGS.lock() {
        s.remove(file_path);
    }
}

/// Whether we previously applied underline tags for this path (used to skip redundant passes).
pub fn has_applied_diagnostic_underlines_for_path(file_path: &str) -> bool {
    FILES_WITH_APPLIED_UNDERLINE_TAGS
        .lock()
        .ok()
        .map(|s| s.contains(file_path))
        .unwrap_or(false)
}

// Type alias for diagnostics storage
type DiagnosticsMap = Arc<Mutex<HashMap<String, Vec<Diagnostic>>>>;

// Global storage for file diagnostics
// Keys are display-style path strings (see `store_file_diagnostics`) — not `file://` URIs; LSP strips URI prefix before inserting here.
static FILE_DIAGNOSTICS: Lazy<DiagnosticsMap> =
    // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Store diagnostics for a file
pub fn store_file_diagnostics(file_path: &str, diagnostics: Vec<Diagnostic>) {
    // lock() acquires the Mutex lock. It blocks until the lock is available.
    // Keys match `Path::display`-style strings (forward slashes on Unix); keep consistent when calling.
    if let Ok(mut map) = FILE_DIAGNOSTICS.lock() {
        if diagnostics.is_empty() {
            map.remove(file_path); // remove key entirely so `get_file_diagnostics` returns no stale items
        } else {
            map.insert(file_path.to_string(), diagnostics);
        }
    }
}

/// Get diagnostics for a file
pub fn get_file_diagnostics(file_path: &str) -> Vec<Diagnostic> {
    // Returns a clone — safe to mutate/discard; authoritative store stays in `FILE_DIAGNOSTICS`.
    FILE_DIAGNOSTICS
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        .lock()
        .ok()
        .and_then(|map| map.get(file_path).cloned())
        .unwrap_or_default()
}

/// Apply diagnostic underlines to a source buffer
pub fn apply_diagnostic_underlines(buffer: &sourceview5::Buffer, file_path: &str) {
    // `file_path` must match the key used in `store_file_diagnostics` (plain path string — not `file://` URIs used by the panel store).
    // --- Load & short-circuit -------------------------------------------------
    let mut diagnostics = get_file_diagnostics(file_path);

    underline_debug(format!(
        "🖊️  apply_diagnostic_underlines for {}: {} diagnostics",
        file_path,
        diagnostics.len()
    ));

    let tag_table = buffer.tag_table();
    let start_iter = buffer.start_iter();
    let end_iter = buffer.end_iter();

    // --- Empty diagnostics: fast path (avoid O(n) tag removal when nothing was ever drawn) ----------
    if diagnostics.is_empty() {
        let had_underlines = FILES_WITH_APPLIED_UNDERLINE_TAGS
            .lock()
            .ok()
            .map(|s| s.contains(file_path))
            .unwrap_or(false);
        if !had_underlines {
            underline_debug("(skip underline clear: nothing was previously applied for this path)");
            return;
        }
        if let Some(tag) = tag_table.lookup("diagnostic-info-underline") {
            buffer.remove_tag(&tag, &start_iter, &end_iter);
        }
        if let Some(tag) = tag_table.lookup("diagnostic-warning-underline") {
            buffer.remove_tag(&tag, &start_iter, &end_iter);
        }
        if let Some(tag) = tag_table.lookup("diagnostic-error-underline") {
            buffer.remove_tag(&tag, &start_iter, &end_iter);
        }
        if let Ok(mut s) = FILES_WITH_APPLIED_UNDERLINE_TAGS.lock() {
            s.remove(file_path);
        }
        underline_debug("✓ Cleared diagnostic tags (had previous underlines)");
        return;
    }

    // Clear stale diagnostic tags before applying a new non-empty set
    // --- Full clear + re-apply (non-empty set) ----------------------------------------------------
    if let Some(tag) = tag_table.lookup("diagnostic-info-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }
    if let Some(tag) = tag_table.lookup("diagnostic-warning-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }
    if let Some(tag) = tag_table.lookup("diagnostic-error-underline") {
        buffer.remove_tag(&tag, &start_iter, &end_iter);
    }

    // Sort diagnostics by severity (Info > Warning > Error) so more severe ones are applied last and take precedence
    // Later `apply_tag` calls win over earlier overlaps — sort ascending severity so error wins.
    diagnostics.sort_by(|a, b| {
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        let severity_value = |s: &DiagnosticSeverity| match s {
            DiagnosticSeverity::Info => 0,
            DiagnosticSeverity::Warning => 1,
            DiagnosticSeverity::Error => 2,
        };
        severity_value(&a.severity).cmp(&severity_value(&b.severity))
    });

    // Create or get tags for each severity level with proper priorities
    // Gtk `TextTag` objects are created once per process and reused (lookup-or-insert).
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

    let mut applied_any = false;

    // Apply tags for each diagnostic (sorted by severity, so errors override warnings/info)
    for diag in diagnostics {
        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        // One GtkTextTag application per diagnostic — overlapping ranges use last-applied tag color.
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
            applied_any = true;
        }
    }

    if applied_any {
        if let Ok(mut s) = FILES_WITH_APPLIED_UNDERLINE_TAGS.lock() {
            s.insert(file_path.to_string());
        }
    }
}

#[cfg(test)]
#[path = "../../tests/unit/linter/mod_tests.rs"]
mod tests;
