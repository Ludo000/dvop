//! # Extensions Module — Plugin Architecture
//!
//! Dvop supports two kinds of extensions:
//!
//! 1. **Script extensions** — folders under `~/.config/dvop/extensions/` containing
//!    a `manifest.json` (schema: `ExtensionManifest`) plus shell scripts. Scripts
//!    are executed via `runner::run_script()` with a 5-second timeout.
//! 2. **Native extensions** — Rust code compiled into the binary that implements
//!    the `NativeExtension` trait (see `native.rs`). Currently only
//!    `RustDiagnosticsExtension` exists.
//!
//! Both kinds share the same `ExtensionManifest` schema and appear in the
//! Extensions panel where they can be enabled/disabled.
//!
//! ## Submodules
//!
//! | Module | Role |
//! |--------|------|
//! | `manager` | Global `ExtensionManager` — loads, caches, enables/disables extensions |
//! | `runner` | Executes shell scripts with timeout, stdin/stdout piping |
//! | `hooks` | Lifecycle hooks (file open/save/close) + keybindings + context menus |
//! | `native` | Trait + registry for compiled-in extensions |
//! | `rust_diagnostics` | Native extension: rust-analyzer integration |
//! | `ui` | Extensions panel UI (cards, install dialog, detail view) |
//! | `sample` | Stub for bundled sample extension archives |
//!
//! See FEATURES.md: Feature #87–#109 — Extension System

pub mod hooks;
pub mod manager;
pub mod native;
pub mod runner;
pub mod rust_diagnostics;
pub mod sample;
pub mod ui;

use serde::{Deserialize, Serialize};

/// Extension manifest describing an extension's metadata and contributions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub icon: Option<String>,
    /// If true, this extension is a built-in native extension (code compiled in, no scripts).
    #[serde(default)]
    pub is_native: bool,
    #[serde(default)]
    pub contributions: ExtensionContributions,
}

/// What the extension contributes to the editor
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtensionContributions {
    /// Script whose stdout is shown in the status bar.
    /// Called with the current file path as $1.
    #[serde(default)]
    pub status_bar: Option<StatusBarContribution>,

    /// CSS file to inject into the app (overrides default styles).
    #[serde(default)]
    pub css: Option<CssContribution>,

    /// Keyboard shortcuts bound to scripts.
    #[serde(default)]
    pub keybindings: Vec<KeybindingContribution>,

    /// Commands available in the command palette.
    #[serde(default)]
    pub commands: Vec<CommandContribution>,

    /// Context menu entries for editor and file explorer.
    #[serde(default)]
    pub context_menus: Option<ContextMenuContributions>,

    /// Linter scripts invoked per-language.
    #[serde(default)]
    pub linters: Vec<LinterContribution>,

    /// Lifecycle hook scripts (on_file_open, on_file_save, etc.).
    #[serde(default)]
    pub hooks: Option<HooksContribution>,

    /// Text transform commands (receive selection on stdin, output to stdout).
    #[serde(default)]
    pub text_transforms: Vec<TextTransformContribution>,

    /// Custom sidebar panels.
    #[serde(default)]
    pub sidebar_panels: Vec<SidebarPanelContribution>,
}

/// A status bar contribution — runs a script and displays its output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusBarContribution {
    /// Path to script relative to the extension directory
    pub script: String,
}

/// A CSS theme/style contribution — a static CSS file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CssContribution {
    /// Path to CSS file relative to the extension directory
    pub file: String,
}

/// A keyboard shortcut contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeybindingContribution {
    /// Key combo in GTK notation, e.g. "Ctrl+Shift+L"
    pub key: String,
    /// Human-readable title
    pub title: String,
    /// Script to execute. Receives $1=file_path $2=selection. Non-empty stdout replaces selection.
    pub script: String,
}

/// A command palette entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandContribution {
    /// Unique command ID within the extension
    pub id: String,
    /// Display title in the palette
    pub title: String,
    /// Script to execute. Receives $1=file_path $2=selection. Non-empty stdout replaces selection.
    pub script: String,
    /// Search keywords for fuzzy matching
    #[serde(default)]
    pub keywords: Vec<String>,
}

/// Context menu entries for different areas
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContextMenuContributions {
    /// Editor right-click entries
    #[serde(default)]
    pub editor: Vec<EditorContextMenu>,
    /// File explorer right-click entries
    #[serde(default)]
    pub file_explorer: Vec<FileExplorerContextMenu>,
}

/// An editor context menu entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorContextMenu {
    /// Label shown in the menu
    pub label: String,
    /// Script. Receives $1=file_path $2=selection $3=line $4=col. Non-empty stdout replaces selection.
    pub script: String,
}

/// A file explorer context menu entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileExplorerContextMenu {
    /// Label shown in the menu
    pub label: String,
    /// Script. Receives $1=file_path (the right-clicked file).
    pub script: String,
}

/// A linter contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinterContribution {
    /// File extensions this linter applies to (e.g. ["py", "python"])
    pub languages: Vec<String>,
    /// Script. Receives $1=file_path. Must output JSON array of diagnostics.
    pub script: String,
}

/// Lifecycle hook scripts
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HooksContribution {
    /// Runs when a file is opened
    #[serde(default)]
    pub on_file_open: Option<String>,
    /// Runs when a file is saved
    #[serde(default)]
    pub on_file_save: Option<String>,
    /// Runs when a file tab is closed
    #[serde(default)]
    pub on_file_close: Option<String>,
    /// Runs when the app starts
    #[serde(default)]
    pub on_app_start: Option<String>,
}

/// A text transform contribution (stdin/stdout)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextTransformContribution {
    /// Unique transform ID
    pub id: String,
    /// Display title
    pub title: String,
    /// Script. Receives $1=file_path. Selected text on stdin. Outputs replacement on stdout.
    pub script: String,
}

/// A sidebar panel contribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SidebarPanelContribution {
    /// Unique panel ID
    pub id: String,
    /// Panel title
    pub title: String,
    /// Icon name (GTK icon, e.g. "accessories-text-editor-symbolic")
    #[serde(default = "default_panel_icon")]
    pub icon: String,
    /// Script. Receives $1=action ("init"/"refresh") $2=file_path. Output displayed in panel.
    pub script: String,
}

fn default_panel_icon() -> String {
    "application-x-addon-symbolic".to_string()
}

/// Represents a loaded extension
#[derive(Debug, Clone)]
pub struct Extension {
    pub manifest: ExtensionManifest,
    pub path: std::path::PathBuf,
}

impl Extension {
    pub fn new(manifest: ExtensionManifest, path: std::path::PathBuf) -> Self {
        Self { manifest, path }
    }
}
