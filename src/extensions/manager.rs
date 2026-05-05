//! # Extension Manager — Loading, Caching & Lifecycle
//!
//! Owns the list of loaded `Extension` instances and exposes methods to
//! install, remove, enable, disable, and query extensions. A single global
//! instance lives in `EXTENSION_MANAGER` (`Lazy<Mutex<ExtensionManager>>`).
//!
//! At startup, `init()` scans `~/.config/dvop/extensions/` for directories
//! containing a `manifest.json`, parses it, and builds an `Extension` object.
//!
//! Also manages `EXTENSION_STATUS_TEXT` — a cached string produced by running
//! all extensions’ `status_bar` scripts for the current file.
//!
//! See FEATURES.md: Feature #87 — Extension System
//! See FEATURES.md: Feature #88 — Extension Install from Archive

use super::{Extension, ExtensionManifest};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

static EXTENSION_MANAGER: once_cell::sync::Lazy<Mutex<ExtensionManager>> =
    // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
    once_cell::sync::Lazy::new(|| Mutex::new(ExtensionManager::new()));

/// Cached status bar text produced by extension scripts
static EXTENSION_STATUS_TEXT: once_cell::sync::Lazy<Mutex<String>> =
    // Mutex ensures only one thread can access the inner data at a time to prevent race conditions.
    once_cell::sync::Lazy::new(|| Mutex::new(String::new()));

/// Manages the lifecycle of all extensions
pub struct ExtensionManager {
    extensions: Vec<Extension>,
    extensions_dir: PathBuf,
}

// "impl" blocks define methods and behavior for a struct or enum.
impl ExtensionManager {
    fn new() -> Self {
        let extensions_dir = get_extensions_dir();
        Self {
            extensions: Vec::new(),
            extensions_dir,
        }
    }

    /// Load all extensions from the extensions directory
    pub fn load_extensions(&mut self) {
        self.extensions.clear();

        if !self.extensions_dir.exists() {
            if let Err(e) = std::fs::create_dir_all(&self.extensions_dir) {
                eprintln!("Failed to create extensions directory: {}", e);
                return;
            }
        }

        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        let entries = match std::fs::read_dir(&self.extensions_dir) {
            Ok(entries) => entries,
            Err(e) => {
                eprintln!("Failed to read extensions directory: {}", e);
                return;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let manifest_path = path.join("manifest.json");
            if !manifest_path.exists() {
                continue;
            }

            // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
            match std::fs::read_to_string(&manifest_path) {
                // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
                Ok(content) => match serde_json::from_str::<ExtensionManifest>(&content) {
                    Ok(manifest) => {
                        println!("Loaded extension: {} v{}", manifest.name, manifest.version);
                        self.extensions.push(Extension::new(manifest, path));
                    }
                    Err(e) => {
                        eprintln!("Failed to parse manifest at {:?}: {}", manifest_path, e);
                    }
                },
                Err(e) => {
                    eprintln!("Failed to read manifest at {:?}: {}", manifest_path, e);
                }
            }
        }

        println!(
            "Extension manager loaded {} extension(s)",
            self.extensions.len()
        );
    }

    /// Get all loaded extensions (script-based only)
    pub fn get_extensions(&self) -> &[Extension] {
        &self.extensions
    }

    /// Get all extensions including native built-in extensions.
    /// Native extensions are appended as virtual Extension objects.
    pub fn get_all_extensions(&self) -> Vec<Extension> {
        let mut all: Vec<Extension> = self.extensions.clone();
        for manifest in super::native::get_native_manifests() {
            all.push(Extension::new(manifest, std::path::PathBuf::new()));
        }
        all
    }

    /// Enable or disable an extension by ID
    pub fn set_enabled(&mut self, id: &str, enabled: bool) {
        // Check if this is a native extension first
        if super::native::is_native_extension(id) {
            super::native::set_native_enabled(id, enabled);
            println!(
                "Native extension '{}' {}",
                id,
                if enabled { "enabled" } else { "disabled" }
            );
            return;
        }

        for ext in &mut self.extensions {
            if ext.manifest.id == id {
                ext.manifest.enabled = enabled;
                // Write updated manifest back to disk
                let manifest_path = ext.path.join("manifest.json");
                if let Ok(json) = serde_json::to_string_pretty(&ext.manifest) {
                    if let Err(e) = std::fs::write(&manifest_path, json) {
                        eprintln!("Failed to save manifest: {}", e);
                    }
                }
                println!(
                    "Extension '{}' {}",
                    ext.manifest.name,
                    if enabled { "enabled" } else { "disabled" }
                );
                break;
            }
        }
    }

    /// Remove an extension by ID (deletes from disk)
    pub fn remove_extension(&mut self, id: &str) -> Result<String, String> {
        if let Some(pos) = self.extensions.iter().position(|e| e.manifest.id == id) {
            let ext = self.extensions.remove(pos);
            let name = ext.manifest.name.clone();
            if let Err(e) = std::fs::remove_dir_all(&ext.path) {
                return Err(format!("Failed to remove extension directory: {}", e));
            }
            Ok(name)
        } else {
            Err("Extension not found".to_string())
        }
    }

    /// Run all enabled extensions' status_bar scripts for the given file.
    /// Returns concatenated output.
    pub fn run_status_bar_scripts(&self, file_path: &Path) -> String {
        let mut parts = Vec::new();

        for ext in &self.extensions {
            if !ext.manifest.enabled {
                continue;
            }
            if let Some(ref contrib) = ext.manifest.contributions.status_bar {
                let script_path = ext.path.join(&contrib.script);
                if !script_path.exists() {
                    continue;
                }

                match std::process::Command::new("bash")
                    .arg(&script_path)
                    .arg(file_path)
                    .output()
                {
                    Ok(output) => {
                        if output.status.success() {
                            let text = String::from_utf8_lossy(&output.stdout)
                                .trim()
                                .to_string();
                            if !text.is_empty() {
                                parts.push(text);
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to run status script for '{}': {}",
                            ext.manifest.name, e
                        );
                    }
                }
            }
        }

        parts.join(" | ")
    }

    /// Get all CSS file paths from enabled extensions (absolute paths).
    pub fn get_extension_css_paths(&self) -> Vec<std::path::PathBuf> {
        let mut paths = Vec::new();
        for ext in &self.extensions {
            if !ext.manifest.enabled {
                continue;
            }
            if let Some(ref css) = ext.manifest.contributions.css {
                let css_path = ext.path.join(&css.file);
                if css_path.exists() {
                    paths.push(css_path);
                }
            }
        }
        paths
    }

    /// Get all commands from enabled extensions.
    /// Returns (ext_path, command) tuples.
    pub fn get_extension_commands(&self) -> Vec<(std::path::PathBuf, super::CommandContribution)> {
        let mut cmds = Vec::new();
        for ext in &self.extensions {
            if !ext.manifest.enabled {
                continue;
            }
            for cmd in &ext.manifest.contributions.commands {
                cmds.push((ext.path.clone(), cmd.clone()));
            }
        }
        cmds
    }

    /// Get all text transforms from enabled extensions.
    /// Returns (ext_path, transform) tuples.
    pub fn get_extension_transforms(&self) -> Vec<(std::path::PathBuf, super::TextTransformContribution)> {
        let mut transforms = Vec::new();
        for ext in &self.extensions {
            if !ext.manifest.enabled {
                continue;
            }
            for t in &ext.manifest.contributions.text_transforms {
                transforms.push((ext.path.clone(), t.clone()));
            }
        }
        transforms
    }

    /// Get all sidebar panel contributions from ALL extensions (not just enabled).
    /// Returns (ext_id, panel, enabled) tuples.
    pub fn get_extension_sidebar_panels(&self) -> Vec<(String, super::SidebarPanelContribution, bool)> {
        let mut panels = Vec::new();
        for ext in &self.extensions {
            for p in &ext.manifest.contributions.sidebar_panels {
                panels.push((ext.manifest.id.clone(), p.clone(), ext.manifest.enabled));
            }
        }
        panels
    }

    /// Get all file explorer context menu entries from enabled extensions.
    /// Returns (ext_path, entry) tuples.
    pub fn get_file_explorer_context_menu_entries(&self) -> Vec<(std::path::PathBuf, super::FileExplorerContextMenu)> {
        let mut entries = Vec::new();
        for ext in &self.extensions {
            if !ext.manifest.enabled {
                continue;
            }
            if let Some(ref ctx) = ext.manifest.contributions.context_menus {
                for entry in &ctx.file_explorer {
                    entries.push((ext.path.clone(), entry.clone()));
                }
            }
        }
        entries
    }
}

/// Install an extension from a .tar.gz archive.
/// Extracts into the extensions directory and reloads.
/// Returns the name of the installed extension on success.
pub fn install_from_archive(archive_path: &Path) -> Result<String, String> {
    let extensions_dir = get_extensions_dir();
    if let Err(e) = std::fs::create_dir_all(&extensions_dir) {
        return Err(format!("Failed to create extensions directory: {}", e));
    }

    // Extract tar.gz using system tar command
    let output = std::process::Command::new("tar")
        .arg("xzf")
        .arg(archive_path)
        .arg("-C")
        .arg(&extensions_dir)
        .output()
        .map_err(|e| format!("Failed to run tar: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("tar extraction failed: {}", stderr));
    }

    // Reload extensions to pick up the new one
    let mut mgr = get_manager();
    let old_count = mgr.get_extensions().len();
    mgr.load_extensions();
    let new_count = mgr.get_extensions().len();

    if new_count > old_count {
        // Find the newly added extension
        if let Some(ext) = mgr.get_extensions().last() {
            return Ok(ext.manifest.name.clone());
        }
    }

    Ok("Extension".to_string())
}

/// Run status bar scripts for the given file and cache the result
pub fn update_status_bar_text(file_path: &Path) {
    let mgr = get_manager();
    let text = mgr.run_status_bar_scripts(file_path);
    drop(mgr);
    // lock() acquires the Mutex lock. It blocks until the lock is available.
    if let Ok(mut cached) = EXTENSION_STATUS_TEXT.lock() {
        *cached = text;
    }
}

/// Get the cached extension status bar text
pub fn get_status_bar_text() -> String {
    EXTENSION_STATUS_TEXT
        // lock() acquires the Mutex lock. It blocks until the lock is available.
        .lock()
        .map(|s| s.clone())
        .unwrap_or_default()
}

/// Get the extensions directory path (~/.config/dvop/extensions/)
pub fn get_extensions_dir() -> PathBuf {
    if let Some(home) = home::home_dir() {
        home.join(".config").join("dvop").join("extensions")
    } else {
        PathBuf::from(".config/dvop/extensions")
    }
}

/// Access the global extension manager (locked)
pub fn get_manager() -> std::sync::MutexGuard<'static, ExtensionManager> {
    // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
    EXTENSION_MANAGER.lock().unwrap()
}

/// Initialize the extension system: load all extensions from disk
pub fn init() {
    let mut mgr = get_manager();
    mgr.load_extensions();
}
