//! # Linter UI — Editor Integration & Status Bar
//!
//! Glues the linter module to the editor UI:
//!
//! - `setup_linting()` — called when a file is opened; runs built-in +
//!   extension linters and applies underline tags to the buffer.
//! - `store_diagnostics_for_uri()` — stores diagnostics from any source
//!   (LSP, extension, built-in) in a global `HashMap<file_uri, Vec<Diagnostic>>`.
//! - `update_diagnostics_count()` — refreshes the error/warning counts in
//!   the status bar and triggers panel updates.
//! - `register_buffer_for_diagnostics()` — tracks open buffers so diagnostic
//!   underlines can be re-applied when diagnostics change.
//!
//! Thread-local statics hold references to GTK widgets (status bar, callback
//! closures) to avoid threading GTK handles everywhere.
//!
//! See FEATURES.md: Feature #47 — Real-Time Diagnostics
//! See FEATURES.md: Feature #48 — Inline Error Highlighting

// UI integration for the linter
// This module handles displaying lint diagnostics in the editor.
// Language-specific linting (e.g. Rust via rust-analyzer) is handled by
// native extensions in extensions/rust_diagnostics.rs.
use gtk4::{glib, Box as GtkBox, Label, Image};
use sourceview5::{prelude::*, View};
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use super::{lint_by_language, Diagnostic, DiagnosticSeverity};

// Type alias for complex callback types
type VisibilityCallback = RefCell<Option<Rc<dyn Fn(bool)>>>;

// Global diagnostics store — used by both local linters and native extensions
lazy_static::lazy_static! {
    static ref DIAGNOSTICS_STORE: Arc<Mutex<HashMap<String, Vec<Diagnostic>>>> =
        Arc::new(Mutex::new(HashMap::new()));
}

// Thread-local callback for diagnostics panel visibility
thread_local! {
    static DIAGNOSTICS_PANEL_CALLBACK: VisibilityCallback = RefCell::new(None);
    static LINTER_STATUS_BOX: RefCell<Option<GtkBox>> = const { RefCell::new(None) };
    static LINTER_STATUS_VISIBILITY_CALLBACK: VisibilityCallback = RefCell::new(None);
    
    // Track open buffers by file URI for reapplying diagnostics (thread-local since GTK is single-threaded)
    static BUFFER_REGISTRY: RefCell<HashMap<String, glib::WeakRef<sourceview5::Buffer>>> = 
        RefCell::new(HashMap::new());
    
    // Track source views by file URI for forcing redraw after diagnostic updates
    static VIEW_REGISTRY: RefCell<HashMap<String, glib::WeakRef<sourceview5::View>>> = 
        RefCell::new(HashMap::new());
    
    // Track the currently active view and its file path for refresh button
    static ACTIVE_VIEW_INFO: RefCell<Option<(glib::WeakRef<sourceview5::View>, std::path::PathBuf)>> = 
        RefCell::new(None);
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
pub fn update_linter_status_with_icons(_rust_icon: bool, errors: usize, warnings: usize) {
    glib::idle_add_once(move || {
        LINTER_STATUS_BOX.with(|cell| {
            if let Some(ref status_box) = *cell.borrow() {
                // Clear existing children
                while let Some(child) = status_box.first_child() {
                    status_box.remove(&child);
                }
                
                // Show diagnostics counts directly without "Rust:" prefix
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

/// Show the diagnostics panel (only if diagnostics extension is enabled and there are diagnostics).
/// Schedules via idle_add_once — safe to call from any thread.
pub fn show_diagnostics_panel() {
    glib::idle_add_once(|| {
        if !crate::extensions::rust_diagnostics::is_enabled() {
            return;
        }
        if !has_any_diagnostics() {
            return;
        }
        DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
            if let Some(ref callback) = *cell.borrow() {
                callback(true);
            }
        });
    });
}

/// Show the diagnostics panel immediately — must be called on the GTK main thread
/// (e.g. from within a glib::idle_add or glib::idle_add_once callback).
/// Skips the has_any_diagnostics check since the caller is expected to have just stored diagnostics.
pub fn show_diagnostics_panel_on_main_thread() {
    if !crate::extensions::rust_diagnostics::is_enabled() {
        return;
    }
    DIAGNOSTICS_PANEL_CALLBACK.with(|cell| {
        if let Some(ref callback) = *cell.borrow() {
            callback(true);
        }
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

/// Check if there are any diagnostics in the store
pub fn has_any_diagnostics() -> bool {
    DIAGNOSTICS_STORE
        .lock()
        .ok()
        .map(|store| !store.is_empty())
        .unwrap_or(false)
}

/// Show or hide the linter status bar widget
pub fn show_linter_status_visibility(show: bool) {
    LINTER_STATUS_VISIBILITY_CALLBACK.with(|cell| {
        if let Some(ref callback) = *cell.borrow() {
            callback(show);
        }
    });
}

/// Store diagnostics for a file URI (used by native extensions and local linters).
/// Pass an empty vec to clear diagnostics for the URI.
pub fn store_diagnostics_for_uri(file_uri: &str, diagnostics: Vec<Diagnostic>) {
    if let Ok(mut store) = DIAGNOSTICS_STORE.lock() {
        if diagnostics.is_empty() {
            store.remove(file_uri);
        } else {
            store.insert(file_uri.to_string(), diagnostics);
        }
    }
}

/// Clear all diagnostics from the store (used during native extension shutdown).
pub fn clear_all_diagnostics_store() {
    if let Ok(mut store) = DIAGNOSTICS_STORE.lock() {
        store.clear();
    }
}

/// Update diagnostics count in the linter status
pub fn update_diagnostics_count() {
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

/// Setup linting for a source view
/// This connects to buffer changes and runs the linter automatically.
/// Language-specific setup (e.g. Rust LSP) is handled by native extensions
/// via on_file_open hooks.
pub fn setup_linting(source_view: &View, file_path: Option<&Path>) {
    println!("🔧 setup_linting called with file_path: {:?}", file_path);
    let buffer = source_view.buffer();

    // Create a clone for the signal handler
    let source_view_weak = source_view.downgrade();
    let file_path_opt = file_path.map(|p| p.to_path_buf());

    // Debounce timer — cancel previous timer on each keystroke so the linter
    // only runs once after the user *stops* typing for 800ms.
    let pending_source_id: Rc<RefCell<Option<glib::SourceId>>> = Rc::new(RefCell::new(None));

    // Connect to buffer changes
    buffer.connect_changed(move |buffer| {
        if let Some(source_view) = source_view_weak.upgrade() {
            // Cancel any previously scheduled linter run (debounce)
            if let Some(old_id) = pending_source_id.borrow_mut().take() {
                old_id.remove();
            }

            let buffer_clone = buffer.clone();
            let source_view_clone = source_view.clone();
            let file_path_clone = file_path_opt.clone();
            let pending_clone = pending_source_id.clone();

            let source_id = glib::timeout_add_local_once(std::time::Duration::from_millis(800), move || {
                // Clear the stored source ID since we're now executing
                pending_clone.borrow_mut().take();
                run_linter(
                    &source_view_clone,
                    &buffer_clone,
                    file_path_clone.as_deref(),
                );
            });
            *pending_source_id.borrow_mut() = Some(source_id);
        }
    });

    // Run initial lint if we have a file path
    if let Some(path) = file_path {
        println!("📁 File path exists: {:?}", path);
        println!("📎 File extension: {:?}", path.extension());

        // Fire native extension hooks (e.g. Rust diagnostics LSP init)
        crate::extensions::native::fire_on_file_open(path);

        // Run linter and show diagnostics panel for all supported file types
        run_linter(source_view, &buffer, file_path);

        // Show diagnostics panel if we have diagnostics
        show_diagnostics_panel();
    } else {
        println!("⚠️  No file path provided to setup_linting");
    }
}

/// Run the linter and display diagnostics.
/// Built-in linters run synchronously (fast); extension linters run in a
/// background thread to avoid blocking the GTK main loop.
fn run_linter(_source_view: &View, buffer: &impl IsA<gtk4::TextBuffer>, file_path: Option<&Path>) {
    // Get buffer content
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    let content = buffer.text(&start, &end, true).to_string();

    // ── Phase 1: fast built-in linters (main thread) ──────────────
    let builtin_diagnostics = if let Some(path) = file_path {
        crate::linter::lint_file_builtin(path, &content)
    } else {
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

    // Apply built-in diagnostics immediately
    if let Some(path) = file_path {
        apply_diagnostics_to_ui(buffer, path, &builtin_diagnostics);
    }

    // ── Phase 2: extension linters (background thread) ────────────
    if let Some(path) = file_path {
        let path_buf = path.to_path_buf();

        std::thread::spawn(move || {
            let ext_diags = crate::extensions::hooks::run_extension_linters(&path_buf);
            if ext_diags.is_empty() {
                return;
            }
            // Marshal results back to the GTK main thread
            glib::idle_add_local_once(move || {
                let file_uri = format!("file://{}", path_buf.display());
                let file_path_str = path_buf.to_string_lossy().to_string();

                if let Ok(mut store) = DIAGNOSTICS_STORE.lock() {
                    let entry = store.entry(file_uri.clone()).or_insert_with(Vec::new);
                    entry.extend(ext_diags.clone());
                }

                crate::linter::store_file_diagnostics(&file_path_str, ext_diags);

                // Re-apply underlines using the buffer registry
                reapply_diagnostic_underlines(&file_uri);

                // Refresh UI
                update_diagnostics_count();
                refresh_diagnostics_panel();
            });
        });
    }
}

/// Helper: store diagnostics and update the UI widgets (main-thread only).
fn apply_diagnostics_to_ui(
    buffer: &impl IsA<gtk4::TextBuffer>,
    path: &Path,
    diagnostics: &[Diagnostic],
) {
    let file_uri = format!("file://{}", path.display());

    // Display diagnostics in console
    if !diagnostics.is_empty() {
        println!("🔍 Linter found {} diagnostic(s)", diagnostics.len());
        for diag in diagnostics {
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
    } else {
        println!("✅ Linter found no issues");
    }

    // Store in DIAGNOSTICS_STORE
    let has_any_diagnostics = if let Ok(mut store) = DIAGNOSTICS_STORE.lock() {
        if diagnostics.is_empty() {
            store.remove(&file_uri);
        } else {
            store.insert(file_uri.clone(), diagnostics.to_vec());
        }
        !store.is_empty()
    } else {
        false
    };

    // Also store in the global file diagnostics (for underlines)
    crate::linter::store_file_diagnostics(&path.to_string_lossy(), diagnostics.to_vec());

    // Apply underlines if we have a source buffer
    if let Some(source_buffer) = buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
        crate::linter::apply_diagnostic_underlines(source_buffer, &path.to_string_lossy());
    }

    // Show linter status widget if we have any diagnostics
    // and the rust diagnostics extension is enabled
    if has_any_diagnostics && crate::extensions::rust_diagnostics::is_enabled() {
        LINTER_STATUS_VISIBILITY_CALLBACK.with(|cell| {
            if let Some(ref callback) = *cell.borrow() {
                callback(true);
            }
        });
    }

    // Refresh the diagnostics panel
    glib::idle_add_local_once(|| {
        refresh_diagnostics_panel();
    });
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

/// Notify native extensions that a file was saved.
/// This replaces the old direct rust-analyzer notification.
pub fn notify_file_saved(file_path: &Path) {
    crate::extensions::native::fire_on_file_save(file_path);
}

/// Reapply diagnostic underlines to the currently open file
pub fn reapply_diagnostic_underlines(file_uri: &str) {
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
                
                // Force the view to redraw to show changes immediately
                VIEW_REGISTRY.with(|view_registry| {
                    if let Some(weak_view) = view_registry.borrow().get(file_uri) {
                        if let Some(view) = weak_view.upgrade() {
                            view.queue_draw();
                            println!("✓ Queued redraw of source view");
                        }
                    }
                });
            } else {
                println!("⚠️  Buffer reference is no longer valid for {}", file_uri);
            }
        } else {
            println!("⚠️  No buffer registered for {}", file_uri);
        }
    });
}

/// Register a buffer and view for diagnostic underline updates
pub fn register_buffer_for_diagnostics(file_path: &Path, buffer: &sourceview5::Buffer, view: &sourceview5::View) {
    if let Ok(url) = url::Url::from_file_path(file_path) {
        let uri = url.to_string();
        println!("📝 Registering buffer and view for diagnostics: {}", uri);
        
        BUFFER_REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.insert(uri.clone(), buffer.downgrade());
        });
        
        VIEW_REGISTRY.with(|registry| {
            let mut registry = registry.borrow_mut();
            registry.insert(uri, view.downgrade());
        });
    }
}

/// Iterate over all registered file URIs (must be called on the GTK main thread).
pub fn for_each_registered_file<F: FnMut(&str)>(mut f: F) {
    BUFFER_REGISTRY.with(|registry| {
        let registry = registry.borrow();
        for uri in registry.keys() {
            f(uri);
        }
    });
}

/// Set the currently active view for the refresh button
/// This should be called whenever the user switches to a different file
pub fn set_active_view_for_refresh(file_path: Option<&Path>, view: Option<&sourceview5::View>) {
    ACTIVE_VIEW_INFO.with(|cell| {
        if let (Some(path), Some(v)) = (file_path, view) {
            *cell.borrow_mut() = Some((v.downgrade(), path.to_path_buf()));
        } else {
            *cell.borrow_mut() = None;
        }
    });
}

/// Trigger a refresh of diagnostics for the currently active file
/// This is called by the refresh button in the diagnostics panel
pub fn trigger_lint_refresh() {
    ACTIVE_VIEW_INFO.with(|cell| {
        if let Some((weak_view, file_path)) = &*cell.borrow() {
            if let Some(view) = weak_view.upgrade() {
                println!("🔄 Refreshing lint for: {:?}", file_path);
                let buffer = view.buffer();
                run_linter(&view, &buffer, Some(&file_path));
                show_diagnostics_panel();
            }
        }
    });
}
