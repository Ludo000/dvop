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
    static ref ENABLED: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
}

// ── Extension struct ─────────────────────────────────────────────

pub struct CodeCompletionExtension;

impl CodeCompletionExtension {
    pub fn new() -> Self {
        let enabled = load_enabled_state();
        ENABLED.store(enabled, Ordering::SeqCst);
        Self
    }
}

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
    super::native::register(Box::new(CodeCompletionExtension::new()));
}

/// Check if the code completion extension is currently enabled.
pub fn is_enabled() -> bool {
    ENABLED.load(Ordering::SeqCst)
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
            return *map.get("code-completion").unwrap_or(&true);
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
    map.insert("code-completion".to_string(), enabled);

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(&map) {
        let _ = std::fs::write(&path, json);
    }
}
