//! # Rust Diagnostics Extension — rust-analyzer Integration
//!
//! A **native extension** (compiled into the binary) that wraps the
//! rust-analyzer language server. When enabled and a Rust project is open,
//! it:
//!
//! 1. Starts rust-analyzer via the LSP client (`src/lsp/`).
//! 2. Sends `textDocument/didOpen` notifications on file open.
//! 3. Sends `textDocument/didSave` notifications on file save.
//! 4. Receives `textDocument/publishDiagnostics` and converts them to
//!    `Diagnostic` structs consumed by the diagnostics panel.
//!
//! The enable/disable state is persisted in `~/.config/dvop/rust_diagnostics.conf`.
//! The enable/disable state is persisted in `~/.config/dvop/native_extensions.json` (key `rust-diagnostics`),
//! alongside other built-in native extensions.
//!
//! See FEATURES.md: Feature #41 — Rust-Analyzer Integration
//! See FEATURES.md: Feature #47 — Real-Time Diagnostics
//!
// Rust Diagnostics Extension — native extension providing Rust language diagnostics
// via rust-analyzer LSP. This was previously hardcoded in linter/ui.rs and is now
// exposed as a toggleable extension through the extension system.
//! ## Threads: LSP worker vs GTK main thread
//!
//! rust-analyzer I/O runs on a **background thread** (`std::thread::spawn` in this module). When
//! diagnostics arrive, we **`glib::MainContext::default().invoke(...)`** to jump back to the GTK main
//! thread before refreshing widgets — same rule as `linter/ui.rs`.
//!
//! ## Session restore and `DEFER_RUST_LSP_OPENS`
//!
//! Restoring dozens of tabs would start dozens of LSP sessions at once. While bulk restore is active,
//! `didOpen` may be **queued** and flushed later so the UI stays responsive.

use super::native::NativeExtension;
use crate::linter::Diagnostic;
use gtk4::glib;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// When true, Rust `didOpen` / LSP setup is queued instead of running (session bulk-restore).
static DEFER_RUST_LSP_OPENS: AtomicBool = AtomicBool::new(false);

/// Queue of Rust paths that still need `didOpen` while startup deferral is active — drained by [`flush_deferred_rust_lsp_opens`].
static PENDING_RUST_LSP_OPENS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

// ── State ────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    /// Successful `textDocument/didOpen` paths — avoids re-running LSP setup when switching tabs
    /// (e.g. session restore already opened all files in [`flush_deferred_rust_lsp_opens`]).
    static ref LSP_DID_OPEN_PATHS: Mutex<HashSet<PathBuf>> = Mutex::new(HashSet::new());
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
    // `None` until the first rust-analyzer client is constructed; `Some` holds the shared manager for all Rust tabs.
    static ref RUST_ANALYZER: Arc<Mutex<Option<crate::lsp::rust_analyzer::RustAnalyzerManager>>> =
        // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
        Arc::new(Mutex::new(None));

    // Arc<Mutex<T>> provides thread-safe shared mutable state. Arc for multiple owners across threads, Mutex for locking.
    static ref INITIAL_DIAGNOSTICS_RECEIVED: Arc<Mutex<HashMap<String, bool>>> =
        // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
        Arc::new(Mutex::new(HashMap::new()));

    // Per-URI document version sent with each full-buffer `didChange` after save — kept for LSP protocol correctness (see `notify_file_saved`).
    // Arc<Mutex<T>> provides thread-safe shared mutable state. Arc for multiple owners across threads, Mutex for locking.
    static ref DOCUMENT_VERSIONS: Arc<Mutex<HashMap<String, i32>>> =
        // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
        Arc::new(Mutex::new(HashMap::new()));

    // Arc<Mutex<T>> provides thread-safe shared mutable state. Arc for multiple owners across threads, Mutex for locking.
    static ref AWAITING_SAVE_DIAGNOSTICS: Arc<Mutex<HashMap<String, bool>>> =
        // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
        Arc::new(Mutex::new(HashMap::new()));

    static ref ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

// ── Extension struct ─────────────────────────────────────────────

pub struct RustDiagnosticsExtension;

// "impl" blocks define methods and behavior for a struct or enum.
impl RustDiagnosticsExtension {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn new() -> Self {
        // Restore persisted enabled state
        let enabled = load_enabled_state();
        ENABLED.store(enabled, Ordering::SeqCst);
        Self
    }
}

// "impl" blocks define methods and behavior for a struct or enum.
impl NativeExtension for RustDiagnosticsExtension {
    fn id(&self) -> &str {
        "rust-diagnostics"
    }

    fn manifest(&self) -> super::ExtensionManifest {
        super::ExtensionManifest {
            id: "rust-diagnostics".to_string(),
            name: "Rust Diagnostics".to_string(),
            version: "1.0.0".to_string(),
            description: "Real-time Rust diagnostics via rust-analyzer LSP. Provides error and warning underlines, a diagnostics panel, and status bar counts for Rust projects.".to_string(),
            author: "Built-in".to_string(),
            enabled: self.is_enabled(),
            icon: None,
            is_native: true,
            contributions: super::ExtensionContributions {
                linters: vec![super::LinterContribution {
                    languages: vec!["rs".to_string()],
                    script: String::new(), // native — no script
                }],
                ..Default::default()
            },
        }
    }

    fn is_enabled(&self) -> bool {
        ENABLED.load(Ordering::SeqCst)
    }

    fn set_enabled(&mut self, enabled: bool) {
        let was_enabled = ENABLED.swap(enabled, Ordering::SeqCst);
        persist_enabled_state(enabled);

        if was_enabled && !enabled {
            // Shutting down — hide diagnostics tab and linter status
            shutdown_rust_analyzer();
            crate::linter::ui::hide_diagnostics_panel();
            glib::idle_add_once(|| {
                crate::linter::ui::show_linter_status_visibility(false);
            });
        } else if !was_enabled && enabled {
            // Starting up — reinitialize rust-analyzer and restore UI
            initialize_rust_analyzer();

            // Restore diagnostics UI for the current directory
            let dir = crate::settings::get_settings().get_last_folder();
            check_and_update_rust_ui(&dir);

            // Re-open LSP for any Rust files already open in the editor
            crate::linter::ui::for_each_registered_file(|file_uri| {
                let path_str = file_uri.strip_prefix("file://").unwrap_or(file_uri);
                let path = std::path::Path::new(path_str);
                if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    setup_lsp_for_file(path);
                }
            });
        }
    }

    fn on_app_start(&self) {
        if !self.is_enabled() {
            return;
        }
        // Don't block first frame; manager is created before first real LSP work.
        glib::idle_add_local_once(|| {
            initialize_rust_analyzer();
        });
    }

    fn on_directory_open(&self, dir: &Path) {
        if !ENABLED.load(Ordering::SeqCst) {
            return;
        }
        check_and_update_rust_ui(dir);
    }

    fn on_file_open(&self, file_path: &Path) {
        if !ENABLED.load(Ordering::SeqCst) {
            return;
        }
        // Non-Rust files never talk to rust-analyzer.
        if file_path.extension().and_then(|e| e.to_str()) != Some("rs") {
            return;
        }
        if DEFER_RUST_LSP_OPENS.load(Ordering::SeqCst) {
            if let Ok(mut q) = PENDING_RUST_LSP_OPENS.lock() {
                // Deduplicate: session restore may mention the same path more than once.
                if !q.iter().any(|p| p == file_path) {
                    q.push(file_path.to_path_buf());
                }
            }
            return;
        }
        initialize_rust_analyzer(); // no-op if already started
        setup_lsp_for_file(file_path);
    }

    fn on_file_save(&self, file_path: &Path) {
        if !ENABLED.load(Ordering::SeqCst) {
            return;
        }
        notify_file_saved(file_path);
    }

    fn shutdown(&self) {
        shutdown_rust_analyzer();
    }
}

// ── Public helpers (used by main.rs / handlers.rs) ──────────────

/// Register the Rust diagnostics extension. Call once during app init.
pub fn register() {
    // Box::new(...) allocates the data on the heap rather than the stack.
    super::native::register(Box::new(RustDiagnosticsExtension::new()));
}

/// Check if the Rust diagnostics extension is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

/// Defer rust-analyzer `didOpen` until [`flush_deferred_rust_lsp_opens`] runs (e.g. session restore).
pub fn set_defer_rust_lsp_opens(defer: bool) {
    // Turn on/off alongside `handlers::set_bulk_session_restore` while opening many tabs — both reduce startup storms.
    DEFER_RUST_LSP_OPENS.store(defer, Ordering::SeqCst);
}

/// Run queued LSP opens on the GTK main thread after the window has had a chance to load.
pub fn flush_deferred_rust_lsp_opens() {
    // Pairs with session restore: paths queued while `DEFER_RUST_LSP_OPENS` was true — drains here so rust-analyzer sees tabs before spamming `didOpen`.
    if !ENABLED.load(Ordering::SeqCst) {
        if let Ok(mut q) = PENDING_RUST_LSP_OPENS.lock() {
            q.clear();
        }
        return;
    }

    let paths: Vec<PathBuf> = {
        let Ok(mut q) = PENDING_RUST_LSP_OPENS.lock() else {
            return;
        };
        // `take` empties the queue and gives us ownership of the vec — avoids cloning `PathBuf`s.
        std::mem::take(&mut *q)
    };

    if paths.is_empty() {
        return;
    }

    println!(
        "📎 rust-analyzer: opening {} restored Rust file(s) (deferred until after UI startup)",
        paths.len()
    );
    initialize_rust_analyzer();
    for path in paths {
        setup_lsp_for_file(&path);
    }
}

// ── Rust-analyzer lifecycle ─────────────────────────────────────

/// Detect if the current directory contains Rust files or is a Rust project
pub fn is_rust_project(dir: &Path) -> bool {
    // Lightweight scan for sidebar/status chrome — not a substitute for `cargo metadata` or workspace detection.
    if dir.join("Cargo.toml").exists() {
        return true;
    }

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "rs") {
                return true;
            }
            if path.is_dir() && path.file_name().is_some_and(|name| name == "src") {
                if let Ok(src_entries) = std::fs::read_dir(&path) {
                    for src_entry in src_entries.flatten() {
                        if src_entry.path().extension().is_some_and(|ext| ext == "rs") {
                            return true;
                        }
                    }
                }
            }
        }
    }

    false
}

/// Check if a directory contains Rust files and update UI accordingly
pub fn check_and_update_rust_ui(dir: &Path) {
    // Explorer `current_dir` changes call this — toggles Rust-specific status/linter affordances when the folder looks like a project.
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }

    let has_rust = is_rust_project(dir);

    glib::idle_add_once({
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        move || {
            // Status strip stays visible if **either** Rust tooling applies or another linter already produced hits.
            let has_diagnostics = crate::linter::ui::has_any_diagnostics();

            let should_show_status = has_rust || has_diagnostics;

            crate::linter::ui::show_linter_status_visibility(should_show_status);

            if has_rust {
                crate::linter::ui::refresh_diagnostics_panel();
            }

            // If there are already diagnostics (e.g. after re-enabling), show the panel
            if has_diagnostics {
                crate::linter::ui::show_diagnostics_panel_on_main_thread();
            }
        }
    });
}

/// Initialize rust-analyzer if not already running
fn initialize_rust_analyzer() {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if manager_guard.is_none() {
        *manager_guard = Some(crate::lsp::rust_analyzer::RustAnalyzerManager::new());
        crate::linter::ui::update_linter_status("Initializing...");
    }
}

/// Shutdown rust-analyzer and clear diagnostics
fn shutdown_rust_analyzer() {
    // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if let Some(ref manager) = *manager_guard {
        manager.shutdown();

        if let Ok(mut open) = LSP_DID_OPEN_PATHS.lock() {
            open.clear();
        }

        crate::linter::ui::clear_all_diagnostics_store();

        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let mut initial = INITIAL_DIAGNOSTICS_RECEIVED.lock().unwrap();
        initial.clear();
        drop(initial);

        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
        versions.clear();
        drop(versions);

        // lock() acquires the Mutex lock. It blocks until the lock is available.
        let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
        awaiting.clear();
        drop(awaiting);

        crate::linter::ui::refresh_diagnostics_panel();
        crate::linter::ui::update_linter_status("");
    }

    *manager_guard = None;
}

/// Find the workspace root by looking for Cargo.toml
fn find_workspace_root(file_path: &Path) -> PathBuf {
    // Ascend parents until `Cargo.toml` — if none (single stray `.rs`), fall back to immediate parent directory below.
    let mut current = file_path.parent();

    while let Some(dir) = current {
        if dir.join("Cargo.toml").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }

    file_path.parent().unwrap_or(file_path).to_path_buf()
}

/// Setup LSP for a Rust file
fn setup_lsp_for_file(file_path: &Path) {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    if DEFER_RUST_LSP_OPENS.load(Ordering::SeqCst) {
        if let Ok(mut q) = PENDING_RUST_LSP_OPENS.lock() {
            if !q.iter().any(|p| p == file_path) {
                q.push(file_path.to_path_buf());
            }
        }
        return;
    }

    let canonical_path =
        std::fs::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
    {
        let open = LSP_DID_OPEN_PATHS.lock().unwrap();
        if open.contains(&canonical_path) {
            return;
        }
    }

    let workspace_root = find_workspace_root(file_path);

    let file_path_buf = file_path.to_path_buf();
    let dedupe_key = canonical_path.clone();

    // The "move" keyword forces the closure to take ownership of the variables it uses.
    // Blocking RA handshake + file slurp happens here — GTK thread only receives `glib::MainContext::invoke` callbacks afterward.
    std::thread::spawn(move || {
        println!("LSP thread started");
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        let manager_guard = RUST_ANALYZER.lock().unwrap();
        println!("Acquired RUST_ANALYZER lock");
        if let Some(ref manager) = *manager_guard {
            println!("Manager exists, getting client...");
            // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
            match manager.get_client(workspace_root.clone()) {
                Ok(client) => {
                    println!(
                        "✓ Got rust-analyzer client for workspace: {:?}",
                        workspace_root
                    );
                    crate::linter::ui::update_linter_status("Ready");

                    let initial_received = INITIAL_DIAGNOSTICS_RECEIVED.clone();
                    println!("Setting diagnostic callback...");
                    // The "move" keyword forces the closure to take ownership of the variables it uses.
                    // `move` so this closure can be stored and called later from the LSP thread (owns `initial_received`).
                    client.set_diagnostic_callback(move |uri, lsp_diagnostics| {
                        // Ignore diagnostics if the extension was disabled
                        if !ENABLED.load(Ordering::SeqCst) {
                            return;
                        }

                        let uri_str = uri.to_string();
                        println!(
                            "🔔 Received diagnostics for {}: {} items",
                            uri_str,
                            lsp_diagnostics.len()
                        );

                        let diagnostics: Vec<Diagnostic> = lsp_diagnostics
                            .iter()
                            .map(crate::lsp::convert_lsp_diagnostic)
                            .collect();

                        // Store diagnostics via core API
                        crate::linter::ui::store_diagnostics_for_uri(&uri_str, diagnostics.clone());

                        // Also update FILE_DIAGNOSTICS for underline rendering
                        // Strip `file://` so the path matches what GtkSourceView / `PathBuf::display` use for tags.
                        let file_path = uri_str.strip_prefix("file://").unwrap_or(&uri_str);
                        crate::linter::store_file_diagnostics(file_path, diagnostics.clone());

                        // Mark as received
                        let mut initial_map = initial_received.lock().unwrap();
                        initial_map.insert(uri_str.clone(), true);
                        drop(initial_map);

                        println!("📊 Refreshing diagnostics panel");
                        let uri_for_underlines = uri_str.clone();
                        // Must marshal to the UI thread without acquiring the main context from this
                        // worker thread (`idle_add` / `idle_add_local` panic cross-thread).
                        // `invoke` hops to the GTK main thread — required before touching widgets / Gtk buffers.
                        glib::MainContext::default().invoke(move || {
                            crate::linter::ui::refresh_diagnostics_panel();
                            crate::linter::ui::update_diagnostics_count();
                            crate::linter::ui::show_diagnostics_panel_on_main_thread();
                            crate::linter::ui::reapply_diagnostic_underlines(&uri_for_underlines);
                        });

                        println!(
                            "✅ Stored {} diagnostics for {}",
                            diagnostics.len(),
                            uri_str
                        );
                    });

                    println!("Diagnostic callback set, now sending didOpen...");

                    if let Ok(url) = url::Url::from_file_path(&file_path_buf) {
                        println!("Created URL: {}", url);
                        if let Ok(uri) = url.as_str().parse::<lsp_types::Uri>() {
                            println!("Parsed URI: {:?}", uri);
                            if let Ok(content) = std::fs::read_to_string(&file_path_buf) {
                                println!("Read file content: {} bytes", content.len());
                                // Initial `TextDocumentItem.version` — first save path increments `DOCUMENT_VERSIONS` before emitting `didChange`.
                                if let Err(e) = client.did_open(uri, "rust".to_string(), 1, content)
                                {
                                    println!("❌ Failed to send didOpen: {}", e);
                                } else {
                                    println!("✓ Sent didOpen for file: {:?}", file_path_buf);
                                    if let Ok(mut open) = LSP_DID_OPEN_PATHS.lock() {
                                        open.insert(dedupe_key);
                                    }
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

/// Notify LSP that a file was saved — sends didChange + didSave
fn notify_file_saved(file_path: &Path) {
    // Implements save pipeline as full-buffer `didChange` (version bump per URI) followed by `didSave` with disk snapshot.
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    // Only handle Rust files
    if file_path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return;
    }

    let workspace_root = find_workspace_root(file_path);
    let file_path_buf = file_path.to_path_buf();

    // thread::spawn creates a new background thread to run operations without blocking the main UI.
    std::thread::spawn(move || {
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        let manager_guard = RUST_ANALYZER.lock().unwrap();
        if let Some(ref manager) = *manager_guard {
            if let Ok(client) = manager.get_client(workspace_root) {
                if let Ok(url) = url::Url::from_file_path(&file_path_buf) {
                    if let Ok(uri) = url.as_str().parse::<lsp_types::Uri>() {
                        if let Ok(content) = std::fs::read_to_string(&file_path_buf) {
                            let uri_str = uri.to_string();

                            {
                                // lock() acquires the Mutex lock. It blocks until the lock is available.
                                let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
                                awaiting.insert(uri_str.clone(), true);
                            }

                            let version = {
                                let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
                                let v = versions.entry(uri_str.clone()).or_insert(0);
                                *v += 1;
                                *v
                            };
                            // Per-URI monotonic counter for `VersionedTextDocumentIdentifier` on full-buffer `didChange` after save.

                            if let Err(e) = client.did_change(uri.clone(), version, content.clone())
                            {
                                println!("❌ Failed to send didChange: {}", e);
                            } else {
                                println!(
                                    "✓ Sent didChange for file: {:?} (version {})",
                                    file_path_buf, version
                                );
                            }

                            // Tiny delay before `didSave` — gives rust-analyzer time to reconcile incremental state (heuristic, not spec-required).
                            std::thread::sleep(std::time::Duration::from_millis(100));

                            if let Err(e) = client.did_save(uri, Some(content)) {
                                println!("❌ Failed to send didSave: {}", e);
                            } else {
                                println!("✓ Sent didSave for file: {:?}", file_path_buf);
                            }
                        }
                    }
                }
            }
        }
    });
}

// ── Persistence helpers ──────────────────────────────────────────

// Same `native_extensions.json` map other native extensions use — boolean flag keyed by extension id (see also rust-completion).
fn config_path() -> PathBuf {
    if let Some(home) = home::home_dir() {
        home.join(".config").join("dvop").join("native_extensions.json")
    } else {
        PathBuf::from(".config/dvop/native_extensions.json")
    }
}

fn load_enabled_state() -> bool {
    let path = config_path();
    if let Ok(data) = std::fs::read_to_string(&path) {
        if let Ok(map) = serde_json::from_str::<HashMap<String, bool>>(&data) {
            return *map.get("rust-diagnostics").unwrap_or(&true);
        }
    }
    true // enabled by default
}

fn persist_enabled_state(enabled: bool) {
    let path = config_path();
    // Same merge strategy as `code_completion::persist_enabled_state` — keeps sibling extension flags in `native_extensions.json`.
    let mut map: HashMap<String, bool> = if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    };
    map.insert("rust-diagnostics".to_string(), enabled);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&path, json);
    }
}
