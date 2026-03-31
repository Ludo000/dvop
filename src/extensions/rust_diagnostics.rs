// Rust Diagnostics Extension — native extension providing Rust language diagnostics
// via rust-analyzer LSP. This was previously hardcoded in linter/ui.rs and is now
// exposed as a toggleable extension through the extension system.

use super::native::NativeExtension;
use crate::linter::Diagnostic;
use gtk4::glib;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

// ── State ────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    static ref RUST_ANALYZER: Arc<Mutex<Option<crate::lsp::rust_analyzer::RustAnalyzerManager>>> =
        Arc::new(Mutex::new(None));

    static ref INITIAL_DIAGNOSTICS_RECEIVED: Arc<Mutex<HashMap<String, bool>>> =
        Arc::new(Mutex::new(HashMap::new()));

    static ref DOCUMENT_VERSIONS: Arc<Mutex<HashMap<String, i32>>> =
        Arc::new(Mutex::new(HashMap::new()));

    static ref AWAITING_SAVE_DIAGNOSTICS: Arc<Mutex<HashMap<String, bool>>> =
        Arc::new(Mutex::new(HashMap::new()));

    static ref ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

// ── Extension struct ─────────────────────────────────────────────

pub struct RustDiagnosticsExtension;

impl RustDiagnosticsExtension {
    pub fn new() -> Self {
        // Restore persisted enabled state
        let enabled = load_enabled_state();
        ENABLED.store(enabled, Ordering::SeqCst);
        Self
    }
}

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
        initialize_rust_analyzer();
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
        if file_path.extension().and_then(|e| e.to_str()) == Some("rs") {
            println!("🦀 [Rust Diagnostics Extension] Detected Rust file, initializing rust-analyzer");
            initialize_rust_analyzer();
            setup_lsp_for_file(file_path);
        }
    }

    fn on_file_save(&self, file_path: &Path) {
        if !ENABLED.load(Ordering::SeqCst) {
            return;
        }
        notify_file_saved(file_path);
    }

    fn on_file_close(&self, _file_path: &Path) {
        // rust-analyzer handles file tracking internally
    }

    fn shutdown(&self) {
        shutdown_rust_analyzer();
    }
}

// ── Public helpers (used by main.rs / handlers.rs) ──────────────

/// Register the Rust diagnostics extension. Call once during app init.
pub fn register() {
    super::native::register(Box::new(RustDiagnosticsExtension::new()));
}

/// Check if the Rust diagnostics extension is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

// ── Rust-analyzer lifecycle ─────────────────────────────────────

/// Detect if the current directory contains Rust files or is a Rust project
pub fn is_rust_project(dir: &Path) -> bool {
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
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }

    let has_rust = is_rust_project(dir);

    glib::idle_add_once({
        move || {
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
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if manager_guard.is_none() {
        *manager_guard = Some(crate::lsp::rust_analyzer::RustAnalyzerManager::new());
        crate::linter::ui::update_linter_status("Initializing...");
    }
}

/// Shutdown rust-analyzer and clear diagnostics
fn shutdown_rust_analyzer() {
    let mut manager_guard = RUST_ANALYZER.lock().unwrap();
    if let Some(ref manager) = *manager_guard {
        manager.shutdown();

        crate::linter::ui::clear_all_diagnostics_store();

        let mut initial = INITIAL_DIAGNOSTICS_RECEIVED.lock().unwrap();
        initial.clear();
        drop(initial);

        let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
        versions.clear();
        drop(versions);

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
    let workspace_root = find_workspace_root(file_path);

    println!("=== LSP SETUP START ===");
    println!("Setting up LSP for file: {:?}", file_path);
    println!("Workspace root: {:?}", workspace_root);

    let file_path_buf = file_path.to_path_buf();

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
                    crate::linter::ui::update_linter_status("Ready");

                    let initial_received = INITIAL_DIAGNOSTICS_RECEIVED.clone();
                    println!("Setting diagnostic callback...");
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
                        let file_path = uri_str.strip_prefix("file://").unwrap_or(&uri_str);
                        crate::linter::store_file_diagnostics(file_path, diagnostics.clone());

                        // Mark as received
                        let mut initial_map = initial_received.lock().unwrap();
                        initial_map.insert(uri_str.clone(), true);
                        drop(initial_map);

                        println!("📊 Refreshing diagnostics panel");
                        let uri_for_underlines = uri_str.clone();
                        glib::source::idle_add(move || {
                            crate::linter::ui::refresh_diagnostics_panel();
                            crate::linter::ui::update_diagnostics_count();
                            crate::linter::ui::show_diagnostics_panel_on_main_thread();

                            // Reapply underlines to currently visible buffer
                            crate::linter::ui::reapply_diagnostic_underlines(&uri_for_underlines);

                            glib::ControlFlow::Break
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

/// Notify LSP that a file was saved — sends didChange + didSave
fn notify_file_saved(file_path: &Path) {
    if !ENABLED.load(Ordering::SeqCst) {
        return;
    }
    // Only handle Rust files
    if file_path.extension().and_then(|e| e.to_str()) != Some("rs") {
        return;
    }

    let workspace_root = find_workspace_root(file_path);
    let file_path_buf = file_path.to_path_buf();

    std::thread::spawn(move || {
        let manager_guard = RUST_ANALYZER.lock().unwrap();
        if let Some(ref manager) = *manager_guard {
            if let Ok(client) = manager.get_client(workspace_root) {
                if let Ok(url) = url::Url::from_file_path(&file_path_buf) {
                    if let Ok(uri) = url.as_str().parse::<lsp_types::Uri>() {
                        if let Ok(content) = std::fs::read_to_string(&file_path_buf) {
                            let uri_str = uri.to_string();

                            {
                                let mut awaiting = AWAITING_SAVE_DIAGNOSTICS.lock().unwrap();
                                awaiting.insert(uri_str.clone(), true);
                            }

                            let version = {
                                let mut versions = DOCUMENT_VERSIONS.lock().unwrap();
                                let v = versions.entry(uri_str.clone()).or_insert(0);
                                *v += 1;
                                *v
                            };

                            if let Err(e) = client.did_change(uri.clone(), version, content.clone())
                            {
                                println!("❌ Failed to send didChange: {}", e);
                            } else {
                                println!(
                                    "✓ Sent didChange for file: {:?} (version {})",
                                    file_path_buf, version
                                );
                            }

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
