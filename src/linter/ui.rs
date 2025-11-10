// UI integration for the linter
// This module handles displaying lint diagnostics in the editor

use sourceview5::{prelude::*, View};
use gtk4::{glib, Label, Box as GtkBox, Orientation, ScrolledWindow, ListBox};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

use super::{Diagnostic, DiagnosticSeverity, lint_file, lint_by_language};

/// Detect if the current directory contains Rust files or is a Rust project
pub fn is_rust_project(dir: &Path) -> bool {
    // Check for Cargo.toml
    if dir.join("Cargo.toml").exists() {
        println!("✓ Found Cargo.toml in {:?}", dir);
        return true;
    }
    
    // Check for any .rs files in the directory or src subdirectory
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "rs") {
                println!("✓ Found .rs file: {:?}", path);
                return true;
            }
            if path.is_dir() && path.file_name().map_or(false, |name| name == "src") {
                if let Ok(src_entries) = std::fs::read_dir(&path) {
                    for src_entry in src_entries.flatten() {
                        if src_entry.path().extension().map_or(false, |ext| ext == "rs") {
                            println!("✓ Found .rs file in src: {:?}", src_entry.path());
                            return true;
                        }
                    }
                }
            }
        }
    }
    
    println!("✗ No Rust project detected in {:?}", dir);
    false
}

// Global LSP manager
lazy_static::lazy_static! {
    static ref RUST_ANALYZER: Arc<Mutex<Option<crate::lsp::rust_analyzer::RustAnalyzerManager>>> = 
        Arc::new(Mutex::new(None));
    
    static ref DIAGNOSTICS_STORE: Arc<Mutex<HashMap<String, Vec<Diagnostic>>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // Track files that have received their initial diagnostics
    static ref INITIAL_DIAGNOSTICS_RECEIVED: Arc<Mutex<HashMap<String, bool>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // Track document versions for each file
    static ref DOCUMENT_VERSIONS: Arc<Mutex<HashMap<String, i32>>> = 
        Arc::new(Mutex::new(HashMap::new()));
    
    // Track when we're expecting diagnostics after a save
    static ref AWAITING_SAVE_DIAGNOSTICS: Arc<Mutex<HashMap<String, bool>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

/// Initialize rust-analyzer if not already running
pub fn initialize_rust_analyzer() {
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if manager_guard.is_none() {
        println!("🚀 Initializing rust-analyzer...");
        *manager_guard = Some(crate::lsp::rust_analyzer::RustAnalyzerManager::new());
    }
}

/// Shutdown rust-analyzer and clear diagnostics
pub fn shutdown_rust_analyzer() {
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if let Some(ref manager) = *manager_guard {
        println!("🛑 Shutting down rust-analyzer...");
        
        // Properly shutdown the rust-analyzer process
        manager.shutdown();
        
        // Clear all diagnostics
        let mut store = DIAGNOSTICS_STORE.lock().unwrap();
        store.clear();
        drop(store);
        
        // Clear tracking maps
        let mut initial = INITIAL_DIAGNOSTICS_RECEIVED.lock().unwrap();
        initial.clear();
        drop(initial);
        
        let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
        versions.clear();
        drop(versions);
        
        let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
        awaiting.clear();
        drop(awaiting);
        
        // Refresh panel to show empty state
        refresh_diagnostics_panel();
        
        println!("✅ rust-analyzer shut down and diagnostics cleared");
    }
    
    // Remove the manager
    *manager_guard = None;
}

/// Setup linting for a source view
/// This connects to buffer changes and runs the linter automatically
pub fn setup_linting(source_view: &View, file_path: Option<&Path>) {
    println!("🔧 setup_linting called with file_path: {:?}", file_path);
    let buffer = source_view.buffer();
    
    // Create a clone for the signal handler
    let source_view_weak = source_view.downgrade();
    let file_path_opt = file_path.map(|p| p.to_path_buf());
    
    // Connect to buffer changes
    buffer.connect_changed(move |buffer| {
        if let Some(source_view) = source_view_weak.upgrade() {
            // Run linter after a short delay to avoid running on every keystroke
            let buffer_clone = buffer.clone();
            let source_view_clone = source_view.clone();
            let file_path_clone = file_path_opt.clone();
            
            glib::timeout_add_local_once(std::time::Duration::from_millis(500), move || {
                run_linter(&source_view_clone, &buffer_clone, file_path_clone.as_deref());
            });
        }
    });
    
    // Run initial lint if we have a file path
    if let Some(path) = file_path {
        println!("📁 File path exists: {:?}", path);
        println!("📎 File extension: {:?}", path.extension());
        
        // Try to setup LSP for Rust files
        if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            println!("🦀 Detected Rust file, initializing rust-analyzer");
            initialize_rust_analyzer();
            setup_lsp_for_file(source_view, path);
        } else {
            println!("❌ Not a Rust file, extension: {:?}", path.extension());
        }
        run_linter(&source_view, &buffer, file_path);
    } else {
        println!("⚠️  No file path provided to setup_linting");
    }
}

/// Setup LSP for a Rust file
fn setup_lsp_for_file(source_view: &View, file_path: &Path) {
    // Find workspace root (directory containing Cargo.toml)
    let workspace_root = find_workspace_root(file_path);
    
    println!("=== LSP SETUP START ===");
    println!("Setting up LSP for file: {:?}", file_path);
    println!("Workspace root: {:?}", workspace_root);
    
    let file_path_buf = file_path.to_path_buf();

    
    // Spawn LSP initialization in a separate thread
    std::thread::spawn(move || {
        println!("LSP thread started");
        let manager_guard = RUST_ANALYZER.lock().unwrap();
        println!("Acquired RUST_ANALYZER lock");
        if let Some(ref manager) = *manager_guard {
            println!("Manager exists, getting client...");
            match manager.get_client(workspace_root.clone()) {
                Ok(client) => {
                    println!("✓ Got rust-analyzer client for workspace: {:?}", workspace_root);
                    
                    // Setup diagnostic callback
                    let diagnostics_store = DIAGNOSTICS_STORE.clone();
                    let initial_received = INITIAL_DIAGNOSTICS_RECEIVED.clone();
                    println!("Setting diagnostic callback...");
                    client.set_diagnostic_callback(move |uri, lsp_diagnostics| {
                        let uri_str = uri.to_string();
                        println!("🔔 Received diagnostics for {}: {} items", uri_str, lsp_diagnostics.len());
                        
                        // Convert LSP diagnostics to our format
                        let diagnostics: Vec<Diagnostic> = lsp_diagnostics.iter()
                            .map(|d| crate::lsp::convert_lsp_diagnostic(d))
                            .collect();
                        
                        // Store diagnostics
                        let mut store = diagnostics_store.lock().unwrap();
                        store.insert(uri_str.clone(), diagnostics.clone());
                        drop(store);
                        
                        // Mark as received
                        let mut initial_map = initial_received.lock().unwrap();
                        initial_map.insert(uri_str.clone(), true);
                        drop(initial_map);
                        
                        // Always refresh panel when diagnostics change
                        println!("📊 Refreshing diagnostics panel");
                        glib::idle_add_once(|| {
                            refresh_diagnostics_panel();
                        });
                        
                        println!("✅ Stored {} diagnostics for {}", diagnostics.len(), uri_str);
                    });
                    
                    println!("Diagnostic callback set, now sending didOpen...");
                    
                    // Send didOpen notification
                    if let Ok(url) = url::Url::from_file_path(&file_path_buf) {
                        println!("Created URL: {}", url);
                        if let Ok(uri) = url.as_str().parse::<lsp_types::Uri>() {
                            println!("Parsed URI: {:?}", uri);
                            // Read file content
                            if let Ok(content) = std::fs::read_to_string(&file_path_buf) {
                                println!("Read file content: {} bytes", content.len());
                                if let Err(e) = client.did_open(uri, "rust".to_string(), 1, content) {
                                    println!("❌ Failed to send didOpen: {}", e);
                                } else {
                                    println!("✓ Sent didOpen for file: {:?}", file_path_buf);
                                }
                            } else {
                                println!("❌ Failed to read file content");
                            }
                        } else {
                            println!("❌ Failed to parse URI");
                        }
                    } else {
                        println!("❌ Failed to create URL from file path");
                    }
                    println!("=== LSP SETUP COMPLETE ===");
                }
                Err(e) => {
                    println!("❌ Failed to get rust-analyzer client: {}", e);
                    println!("Make sure rust-analyzer is installed: rustup component add rust-analyzer");
                }
            }
        } else {
            println!("❌ Manager is None!");
        }
    });
}

/// Find the workspace root by looking for Cargo.toml
fn find_workspace_root(file_path: &Path) -> PathBuf {
    let mut current = file_path.parent();
    
    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    
    // Fallback to file's directory
    file_path.parent().unwrap_or(file_path).to_path_buf()
}

/// Run the linter and display diagnostics
fn run_linter(_source_view: &View, buffer: &impl IsA<gtk4::TextBuffer>, file_path: Option<&Path>) {
    // Get buffer content
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let content = buffer.text(&start, &end, true).to_string();
    
    // Run linter
    let diagnostics = if let Some(path) = file_path {
        lint_file(path, &content)
    } else {
        // Try to detect language from buffer
        if let Some(source_buffer) = buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
            if let Some(language) = source_buffer.language() {
                let lang_id = language.id().to_string();
                lint_by_language(&lang_id, &content)
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        }
    };
    
    // Display diagnostics
    if !diagnostics.is_empty() {
        println!("Linter found {} diagnostic(s)", diagnostics.len());
        for diag in &diagnostics {
            let severity_str = match diag.severity {
                DiagnosticSeverity::Error => "ERROR",
                DiagnosticSeverity::Warning => "WARNING",
                DiagnosticSeverity::Info => "INFO",
            };
            println!("  [{}] Line {}: {} ({})", severity_str, diag.line, diag.message, diag.rule);
        }
        
        // TODO: Display diagnostics in the UI (underlines, margin icons, etc.)
        // For now, we're just logging them
    }
}

/// Create a diagnostics panel widget to display lint results
pub fn create_diagnostics_panel(diagnostics: &[Diagnostic]) -> GtkBox {
    let panel = GtkBox::new(Orientation::Vertical, 4);
    panel.set_margin_start(8);
    panel.set_margin_end(8);
    panel.set_margin_top(8);
    panel.set_margin_bottom(8);
    
    // Title
    let title = Label::new(Some(&format!("Diagnostics ({})", diagnostics.len())));
    title.add_css_class("title-4");
    title.set_halign(gtk4::Align::Start);
    title.set_margin_bottom(8);
    panel.append(&title);
    
    // List of diagnostics
    let list = ListBox::new();
    list.add_css_class("boxed-list");
    
    for diag in diagnostics {
        let row_box = GtkBox::new(Orientation::Vertical, 2);
        row_box.set_margin_start(4);
        row_box.set_margin_end(4);
        row_box.set_margin_top(4);
        row_box.set_margin_bottom(4);
        
        // Severity and message
        let severity_icon = match diag.severity {
            DiagnosticSeverity::Error => "❌",
            DiagnosticSeverity::Warning => "⚠️",
            DiagnosticSeverity::Info => "ℹ️",
        };
        
        let message_label = Label::new(Some(&format!("{} {}", severity_icon, diag.message)));
        message_label.set_halign(gtk4::Align::Start);
        message_label.set_wrap(true);
        row_box.append(&message_label);
        
        // Location and rule
        let detail_label = Label::new(Some(&format!(
            "Line {}, Column {} • {}",
            diag.line, diag.column, diag.rule
        )));
        detail_label.set_halign(gtk4::Align::Start);
        detail_label.add_css_class("dim-label");
        detail_label.add_css_class("caption");
        row_box.append(&detail_label);
        
        list.append(&row_box);
    }
    
    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_child(Some(&list));
    panel.append(&scrolled);
    
    panel
}

/// Refresh the diagnostics panel with all stored diagnostics
/// This should be called when switching tabs or opening files
pub fn refresh_diagnostics_panel() {
    println!("🔄 Refreshing diagnostics panel...");
    
    // Clear the panel first
    crate::linter::diagnostics_panel::clear_diagnostics();
    
    // Get all stored diagnostics
    let store = DIAGNOSTICS_STORE.lock().unwrap();
    
    // Display each file's diagnostics
    for (file_uri, diagnostics) in store.iter() {
        crate::linter::diagnostics_panel::display_file_diagnostics(file_uri, diagnostics);
    }
    
    println!("✅ Diagnostics panel refreshed with {} files", store.len());
}

/// Notify LSP that a file was saved
/// This should be called after successfully saving a Rust file
pub fn notify_file_saved(file_path: &Path) {
    // Only handle Rust files
    if file_path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return;
    }
    
    let workspace_root = find_workspace_root(file_path);
    let file_path_buf = file_path.to_path_buf();
    
    // Spawn notification in a separate thread
    std::thread::spawn(move || {
        let manager_guard = RUST_ANALYZER.lock().unwrap();
        if let Some(ref manager) = *manager_guard {
            if let Ok(client) = manager.get_client(workspace_root) {
                if let Ok(url) = url::Url::from_file_path(&file_path_buf) {
                    if let Ok(uri) = url.as_str().parse::<lsp_types::Uri>() {
                        // Read file content
                        if let Ok(content) = std::fs::read_to_string(&file_path_buf) {
                            let uri_str = uri.to_string();
                            
                            // Mark that we're awaiting diagnostics for this file
                            {
                                let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
                                awaiting.insert(uri_str.clone(), true);
                            }
                            
                            // Get and increment the version
                            let version = {
                                let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
                                let v = versions.entry(uri_str.clone()).or_insert(0);
                                *v += 1;
                                *v
                            };
                            
                            // First send didChange to update the content
                            if let Err(e) = client.did_change(uri.clone(), version, content.clone()) {
                                println!("❌ Failed to send didChange: {}", e);
                            } else {
                                println!("✓ Sent didChange for file: {:?} (version {})", file_path_buf, version);
                            }
                            
                            // Small delay between didChange and didSave
                            std::thread::sleep(std::time::Duration::from_millis(100));
                            
                            // Then send didSave to trigger analysis
                            if let Err(e) = client.did_save(uri, Some(content)) {
                                println!("❌ Failed to send didSave: {}", e);
                            } else {
                                println!("✓ Sent didSave for file: {:?}", file_path_buf);
                                // The callback will handle refreshing when diagnostics arrive
                            }
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_create_diagnostics_panel() {
        gtk4::init().ok();
        
        let diagnostics = vec![
            Diagnostic::new(
                DiagnosticSeverity::Warning,
                "Test warning".to_string(),
                10,
                5,
                "test_rule".to_string(),
            ),
        ];
        
        let panel = create_diagnostics_panel(&diagnostics);
        assert!(panel.is_visible());
    }
}
