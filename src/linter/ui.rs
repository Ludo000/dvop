// UI integration for the linter
// This module handles displaying lint diagnostics in the editor
use gtk4::{glib, Box as GtkBox, Label, ListBox, Orientation, ScrolledWindow, Image};
use sourceview5::{prelude::*, View};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::{lint_by_language, lint_file, Diagnostic, DiagnosticSeverity};

/// Detect if the current directory contains Rust files or is a Rust project
pub fn is_rust_project(dir: &Path) -> bool {
    if dir.join("Cargo.toml").exists() {
        return true;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "rs") {
                return true;
            }
            if path.is_dir() && path.file_name().map_or(false, |name| name == "src") {
                if let Ok(src_entries) = std::fs::read_dir(&path) {
                    for src_entry in src_entries.flatten() {
                        if src_entry
                            .path()
                            .extension()
                            .map_or(false, |ext| ext == "rs")
                        {
                            return true;
                        }
                    }
                }
            }
        }
    }

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

// Thread-local callback for diagnostics panel visibility
thread_local! {
    static DIAGNOSTICS_PANEL_CALLBACK: RefCell<Option<Rc<dyn Fn(bool)>>> = RefCell::new(None);
    static LINTER_STATUS_BOX: RefCell<Option<GtkBox>> = RefCell::new(None);
    static LINTER_STATUS_VISIBILITY_CALLBACK: RefCell<Option<Rc<dyn Fn(bool)>>> = RefCell::new(None);
    
    // Track open buffers by file URI for reapplying diagnostics (thread-local since GTK is single-threaded)
    static BUFFER_REGISTRY: RefCell<HashMap<String, glib::WeakRef<sourceview5::Buffer>>> = 
        RefCell::new(HashMap::new());
}

/// Set the callback for showing/hiding the diagnostics panel
pub fn set_diagnostics_panel_callback<F>(callback: F)
where
    F: Fn(bool) + 'static,
{
    DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(Rc::new(callback));
    });
}

/// Set the callback for updating the linter status label
pub fn set_linter_status_callback(status_box: GtkBox) {
    LINTER_STATUS_BOX.with(|cell| {
        *cell.borrow_mut() = Some(status_box);
    });
}

/// Set the callback for showing/hiding the linter status widget
pub fn set_linter_status_visibility_callback<F>(callback: F)
where
    F: Fn(bool) + 'static,
{
    LINTER_STATUS_VISIBILITY_CALLBACK.with(|cell| {
        *cell.borrow_mut() = Some(Rc::new(callback));
    });
}

/// Update the linter status label
pub fn update_linter_status(status: &str) {
    glib::idle_add_once({
        let status = status.to_string();
        move || {
            LINTER_STATUS_BOX.with(|cell| {
                if let Some(ref status_box) = *cell.borrow() {
                    // Clear existing children
                    while let Some(child) = status_box.first_child() {
                        status_box.remove(&child);
                    }
                    
                    // Add label with status text
                    let label = Label::new(Some(&status));
                    status_box.append(&label);
                }
            });
        }
    });
}

/// Update the linter status with icons
pub fn update_linter_status_with_icons(rust_icon: bool, errors: usize, warnings: usize) {
    glib::idle_add_once(move || {
        LINTER_STATUS_BOX.with(|cell| {
            if let Some(ref status_box) = *cell.borrow() {
                // Clear existing children
                while let Some(child) = status_box.first_child() {
                    status_box.remove(&child);
                }
                
                // Add Rust icon if requested
                if rust_icon {
                    let icon = Image::from_icon_name("application-x-executable-symbolic");
                    icon.set_pixel_size(16);
                    status_box.append(&icon);
                    
                    let rust_label = Label::new(Some("Rust:"));
                    status_box.append(&rust_label);
                }
                
                if errors > 0 || warnings > 0 {
                    // Add error icon and count
                    if errors > 0 {
                        let error_icon = Image::from_icon_name("dialog-error-symbolic");
                        error_icon.set_pixel_size(16);
                        status_box.append(&error_icon);
                        
                        let error_label = Label::new(Some(&format!("{}", errors)));
                        status_box.append(&error_label);
                    }
                    
                    // Add warning icon and count
                    if warnings > 0 {
                        let warning_icon = Image::from_icon_name("dialog-warning-symbolic");
                        warning_icon.set_pixel_size(16);
                        status_box.append(&warning_icon);
                        
                        let warning_label = Label::new(Some(&format!("{}", warnings)));
                        status_box.append(&warning_label);
                    }
                } else {
                    // All good - show checkmark
                    let check_icon = Image::from_icon_name("emblem-ok-symbolic");
                    check_icon.set_pixel_size(16);
                    status_box.append(&check_icon);
                }
            }
        });
    });
}

/// Show the diagnostics panel
pub fn show_diagnostics_panel() {
    glib::idle_add_once(|| {
        DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
            if let Some(ref callback) = *cell.borrow() {
                callback(true);
            }
        });
    });
}

/// Hide the diagnostics panel
#[allow(dead_code)]
pub fn hide_diagnostics_panel() {
    glib::idle_add_once(|| {
        DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
            if let Some(ref callback) = *cell.borrow() {
                callback(false);
            }
        });
    });
}

/// Check if there are any Rust files with diagnostics
#[allow(dead_code)]
pub fn has_rust_diagnostics() -> bool {
    if let Ok(guard) = DIAGNOSTICS_STORE.lock() {
        !guard.is_empty()
    } else {
        false
    }
}

/// Check if a directory contains Rust files and update UI accordingly
pub fn check_and_update_rust_ui(dir: &Path) {
    let has_rust = is_rust_project(dir);

    glib::idle_add_once({
        let has_rust = has_rust;
        move || {
            LINTER_STATUS_VISIBILITY_CALLBACK.with(|cell| {
                if let Some(ref callback) = *cell.borrow() {
                    callback(has_rust);
                }
            });

            DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
                if let Some(ref callback) = *cell.borrow() {
                    callback(has_rust);
                }
            });

            if has_rust {
                refresh_diagnostics_panel();
            }
        }
    });
}

/// Update diagnostics count in the linter status
fn update_diagnostics_count() {
    if let Ok(store) = DIAGNOSTICS_STORE.lock() {
        let mut errors = 0;
        let mut warnings = 0;

        for diagnostics in store.values() {
            for diag in diagnostics {
                match diag.severity {
                    DiagnosticSeverity::Error => errors += 1,
                    DiagnosticSeverity::Warning => warnings += 1,
                    _ => {}
                }
            }
        }

        update_linter_status_with_icons(true, errors, warnings);
    }
}

/// Initialize rust-analyzer if not already running
pub fn initialize_rust_analyzer() {
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if manager_guard.is_none() {
        *manager_guard = Some(crate::lsp::rust_analyzer::RustAnalyzerManager::new());
        update_linter_status("Rust: Initializing...");
    }
}

/// Shutdown rust-analyzer and clear diagnostics
pub fn shutdown_rust_analyzer() {
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if let Some(ref manager) = *manager_guard {
        manager.shutdown();

        let mut store = DIAGNOSTICS_STORE.lock().unwrap();
        store.clear();
        drop(store);

        let mut initial = INITIAL_DIAGNOSTICS_RECEIVED.lock().unwrap();
        initial.clear();
        drop(initial);

        let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
        versions.clear();
        drop(versions);

        let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
        awaiting.clear();
        drop(awaiting);

        refresh_diagnostics_panel();
        update_linter_status("");
    }

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
                run_linter(
                    &source_view_clone,
                    &buffer_clone,
                    file_path_clone.as_deref(),
                );
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
            show_diagnostics_panel();
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
fn setup_lsp_for_file(_source_view: &View, file_path: &Path) {
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
                    println!(
                        "✓ Got rust-analyzer client for workspace: {:?}",
                        workspace_root
                    );
                    update_linter_status("Rust: Ready");

                    // Setup diagnostic callback
                    let diagnostics_store = DIAGNOSTICS_STORE.clone();
                    let initial_received = INITIAL_DIAGNOSTICS_RECEIVED.clone();
                    println!("Setting diagnostic callback...");
                    client.set_diagnostic_callback(move |uri, lsp_diagnostics| {
                        let uri_str = uri.to_string();
                        println!(
                            "🔔 Received diagnostics for {}: {} items",
                            uri_str,
                            lsp_diagnostics.len()
                        );

                        // Convert LSP diagnostics to our format
                        let diagnostics: Vec<Diagnostic> = lsp_diagnostics
                            .iter()
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
                        let uri_for_underlines = uri_str.clone();
                        glib::source::idle_add(move || {
                            refresh_diagnostics_panel();
                            update_diagnostics_count();
                            
                            // Reapply underlines to currently visible buffer
                            reapply_diagnostic_underlines(&uri_for_underlines);
                            
                            glib::ControlFlow::Break
                        });

                        println!(
                            "✅ Stored {} diagnostics for {}",
                            diagnostics.len(),
                            uri_str
                        );
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
                                if let Err(e) = client.did_open(uri, "rust".to_string(), 1, content)
                                {
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
                    println!(
                        "Make sure rust-analyzer is installed: rustup component add rust-analyzer"
                    );
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
            println!(
                "  [{}] Line {}: {} ({})",
                severity_str, diag.line, diag.message, diag.rule
            );
        }

        // TODO: Display diagnostics in the UI (underlines, margin icons, etc.)
        // For now, we're just logging them
    }
}

/// Create a diagnostics panel widget to display lint results
#[allow(dead_code)]
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

    // Calculate total counts across all files
    let mut total_errors = 0;
    let mut total_warnings = 0;
    let mut total_infos = 0;

    // Display each file's diagnostics and count totals
    for (file_uri, diagnostics) in store.iter() {
        for diag in diagnostics {
            match diag.severity {
                crate::linter::DiagnosticSeverity::Error => total_errors += 1,
                crate::linter::DiagnosticSeverity::Warning => total_warnings += 1,
                crate::linter::DiagnosticSeverity::Info => total_infos += 1,
            }
        }
        crate::linter::diagnostics_panel::display_file_diagnostics(file_uri, diagnostics);
    }

    // Update the summary header
    crate::linter::diagnostics_panel::update_summary(total_errors, total_warnings, total_infos);

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
                            if let Err(e) = client.did_change(uri.clone(), version, content.clone())
                            {
                                println!("❌ Failed to send didChange: {}", e);
                            } else {
                                println!(
                                    "✓ Sent didChange for file: {:?} (version {})",
                                    file_path_buf, version
                                );
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

/// Reapply diagnostic underlines to the currently open file
fn reapply_diagnostic_underlines(file_uri: &str) {
    println!("🎨 Attempting to reapply diagnostic underlines for {}", file_uri);
    
    // Extract file path from URI
    let file_path = file_uri.strip_prefix("file://").unwrap_or(file_uri);
    
    // Try to get the buffer from the thread-local registry
    BUFFER_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        println!("📋 Registry has {} entries", registry.len());
        
        // Debug: print all registered URIs
        for (uri, _) in registry.iter() {
            println!("  - Registered: {}", uri);
        }
        
        if let Some(weak_buffer) = registry.get(file_uri) {
            if let Some(buffer) = weak_buffer.upgrade() {
                println!("✓ Found buffer for {}, reapplying underlines", file_uri);
                crate::linter::apply_diagnostic_underlines(&buffer, file_path);
            } else {
                println!("⚠️  Buffer reference is no longer valid for {}", file_uri);
            }
        } else {
            println!("⚠️  No buffer registered for {}", file_uri);
        }
    });
}

/// Register a buffer for diagnostic underline updates
pub fn register_buffer_for_diagnostics(file_path: &Path, buffer: &sourceview5::Buffer) {
    if let Ok(url) = url::Url::from_file_path(file_path) {
        let uri = url.to_string();
        println!("📝 Registering buffer for diagnostics: {}", uri);
        
        BUFFER_REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.insert(uri, buffer.downgrade());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_diagnostics_panel() {
        gtk4::init().ok();

        let diagnostics = vec![Diagnostic::new(
            DiagnosticSeverity::Warning,
            "Test warning".to_string(),
            10,
            5,
            "test_rule".to_string(),
        )];

        let panel = create_diagnostics_panel(&diagnostics);
        assert!(panel.is_visible());
    }
}
