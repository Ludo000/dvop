// Native extension support for built-in extensions compiled into the binary.
// These extensions use the same manifest system as script extensions but their
// implementation is in Rust code rather than shell scripts.

use std::path::Path;
use std::sync::Mutex;
use once_cell::sync::Lazy;

/// Trait for built-in extensions compiled into the binary.
/// These appear in the extension UI alongside script extensions and can be enabled/disabled.
pub trait NativeExtension: Send + Sync {
    /// Extension ID (must match manifest.id)
    fn id(&self) -> &str;

    /// Extension manifest for UI display and enable/disable state
    fn manifest(&self) -> super::ExtensionManifest;

    /// Whether the extension is currently enabled
    fn is_enabled(&self) -> bool;

    /// Enable or disable the extension. Implementations should persist state.
    fn set_enabled(&mut self, enabled: bool);

    /// Called when the application starts
    fn on_app_start(&self) {}

    /// Called when a directory is opened/changed in the explorer
    fn on_directory_open(&self, _dir: &Path) {}

    /// Called when a file is opened in the editor
    fn on_file_open(&self, _file_path: &Path) {}

    /// Called when a file is saved
    fn on_file_save(&self, _file_path: &Path) {}

    /// Called when a file tab is closed
    fn on_file_close(&self, _file_path: &Path) {}

    /// Called on application shutdown
    fn shutdown(&self) {}
}

// Global registry of native extensions
static NATIVE_REGISTRY: Lazy<Mutex<Vec<Box<dyn NativeExtension>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

/// Register a native extension. Call during app initialization.
pub fn register(ext: Box<dyn NativeExtension>) {
    if let Ok(mut registry) = NATIVE_REGISTRY.lock() {
        println!("Registered native extension: {}", ext.id());
        registry.push(ext);
    }
}

/// Get manifests for all native extensions (for UI display).
pub fn get_native_manifests() -> Vec<super::ExtensionManifest> {
    NATIVE_REGISTRY
        .lock()
        .ok()
        .map(|registry| registry.iter().map(|e| e.manifest()).collect())
        .unwrap_or_default()
}

/// Check if a given extension ID is a native extension.
pub fn is_native_extension(id: &str) -> bool {
    NATIVE_REGISTRY
        .lock()
        .ok()
        .map(|registry| registry.iter().any(|e| e.id() == id))
        .unwrap_or(false)
}

/// Set enabled state for a native extension by ID.
pub fn set_native_enabled(id: &str, enabled: bool) {
    if let Ok(mut registry) = NATIVE_REGISTRY.lock() {
        if let Some(ext) = registry.iter_mut().find(|e| e.id() == id) {
            ext.set_enabled(enabled);
        }
    }
}

/// Fire on_app_start for all enabled native extensions.
pub fn fire_on_app_start() {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            if ext.is_enabled() {
                ext.on_app_start();
            }
        }
    }
}

/// Fire on_directory_open for all enabled native extensions.
pub fn fire_on_directory_open(dir: &Path) {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            if ext.is_enabled() {
                ext.on_directory_open(dir);
            }
        }
    }
}

/// Fire on_file_open for all enabled native extensions.
pub fn fire_on_file_open(file_path: &Path) {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            if ext.is_enabled() {
                ext.on_file_open(file_path);
            }
        }
    }
}

/// Fire on_file_save for all enabled native extensions.
pub fn fire_on_file_save(file_path: &Path) {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            if ext.is_enabled() {
                ext.on_file_save(file_path);
            }
        }
    }
}

/// Fire on_file_close for all enabled native extensions.
pub fn fire_on_file_close(file_path: &Path) {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            if ext.is_enabled() {
                ext.on_file_close(file_path);
            }
        }
    }
}

/// Shutdown all native extensions.
pub fn shutdown_all() {
    if let Ok(registry) = NATIVE_REGISTRY.lock() {
        for ext in registry.iter() {
            ext.shutdown();
        }
    }
}
