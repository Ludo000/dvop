//! # Code Completion Extension — Native Extension
//!
//! A **native extension** (compiled into the binary) that provides intelligent
//! code completion via Ctrl+Space / F1. When enabled, it:
//!
//! 1. Registers keyboard shortcuts for manual completion triggers.
//! 2. Loads JSON-based completion data for the active language.
//! 3. Provides fuzzy-matched, context-aware, proximity-scored suggestions.
//!
//! The enable/disable state is persisted in `~/.config/dvop/native_extensions.json`.
//!
//! See FEATURES.md: Feature #111 — Code Completion

use super::native::NativeExtension;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── State ────────────────────────────────────────────────────────

lazy_static::lazy_static! {
    // Mirrors other native extensions: toggle without threading `self` through every completion call site.
    static ref ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

// ── Extension struct ─────────────────────────────────────────────

pub struct CodeCompletionExtension;

// "impl" blocks define methods and behavior for a struct or enum.
impl CodeCompletionExtension {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn new() -> Self {
        let enabled = load_enabled_state();
        ENABLED.store(enabled, Ordering::SeqCst);
        Self
    }
}

// "impl" blocks define methods and behavior for a struct or enum.
impl NativeExtension for CodeCompletionExtension {
    fn id(&self) -> &str {
        "code-completion"
    }

    fn manifest(&self) -> super::ExtensionManifest {
        super::ExtensionManifest {
            id: "code-completion".to_string(),
            name: "Code Completion".to_string(),
            version: "1.0.0".to_string(),
            description: "Intelligent code completion with fuzzy matching, context-aware ranking, and multi-language support. Trigger with Ctrl+Space or F1.".to_string(),
            author: "Built-in".to_string(),
            enabled: self.is_enabled(),
            icon: None,
            is_native: true,
            contributions: super::ExtensionContributions::default(),
        }
    }

    fn is_enabled(&self) -> bool {
        ENABLED.load(Ordering::SeqCst)
    }

    fn set_enabled(&mut self, enabled: bool) {
        ENABLED.store(enabled, Ordering::SeqCst);
        persist_enabled_state(enabled);
    }
}

// ── Public helpers ───────────────────────────────────────────────

/// Register the code completion extension. Call once during app init.
pub fn register() {
    // Gates JSON keyword/snippet providers (`completion/ui` checks `is_enabled`) — independent of `rust-completion` for stdlib/doc-derived Rust lists.
    // Box::new(...) allocates the data on the heap rather than the stack.
    super::native::register(Box::new(CodeCompletionExtension::new()));
}

/// Check if the code completion extension is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
}

// ── Persistence helpers ──────────────────────────────────────────

// Shared `native_extensions.json` with other built-ins — toggles keyed by extension id (`code-completion`).
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
            return *map.get("code-completion").unwrap_or(&true);
        }
    }
    true // enabled by default
}

fn persist_enabled_state(enabled: bool) {
    let path = config_path();
    // One shared JSON map for all built-in natives — parse existing keys so toggling completion doesn’t erase rust-diagnostics / etc.
    let mut map: HashMap<String, bool> = if let Ok(data) = std::fs::read_to_string(&path) {
        serde_json::from_str(&data).unwrap_or_default()
    } else {
        HashMap::new()
    };
    map.insert("code-completion".to_string(), enabled);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&path, json);
    }
}

#[cfg(test)]
#[path = "../../tests/unit/extensions/code_completion_tests.rs"]
mod tests;
