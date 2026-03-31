//! # Dvop Library Crate — Public Module Re-exports for Testing
//!
//! This file exposes all of Dvop's internal modules as public (`pub mod`) so that
//! integration tests (in the `tests/` directory) and E2E tests can import and exercise
//! internal functionality.
//!
//! In Rust, a project can have both a **binary** (`src/main.rs`) and a **library** (`src/lib.rs`).
//! The binary is what runs when you type `cargo run`. The library is what tests import with
//! `use dvop::handlers;` etc. Without this file, tests couldn't access any of the internal modules.
//!
//! ## Why `update_all_buffer_themes` is a stub
//!
//! The real implementation lives in `src/main.rs` and needs the full GTK window context.
//! This stub exists solely so that the library crate compiles without pulling in the
//! binary-only code. Tests that need theme updates should test `syntax::update_buffer_style_scheme`
//! directly instead.

// ──────────────────────────────────────────────────────────────────────────────
// Public module re-exports — each one mirrors a `mod` declaration in main.rs
// but with `pub` visibility so external crates (tests) can access them.
// ──────────────────────────────────────────────────────────────────────────────
pub mod handlers;    // Tab management, file operations, event handling
pub mod syntax;      // Syntax highlighting and dark mode detection
pub mod utils;       // File browser, path navigation, MIME detection, keyboard shortcuts
pub mod settings;    // User preferences persistence
pub mod search;      // In-file find and replace
pub mod status_log;  // Status bar logging with severity levels
pub mod file_cache;  // File content caching with TTL expiration
pub mod audio;       // Audio playback with waveform/spectrogram visualization
pub mod video;       // Video playback with GStreamer
pub mod completion;  // Code completion (keywords, snippets, imports)
pub mod linter;      // Code diagnostics and GTK UI linting
pub mod lsp;         // Language Server Protocol client (rust-analyzer)
pub mod extensions;  // Extension system (script-based and native)
pub mod ui;          // GTK4 UI components and templates

// Re-export specific functions from main that are used by modules
// Note: In a refactor, these should be moved to appropriate modules
use gtk4::prelude::*;

/// Stub implementation of `update_all_buffer_themes` for the library crate.
///
/// The real implementation is in `src/main.rs` and recursively updates all editor buffers'
/// syntax highlighting schemes when the system theme changes. This stub is a no-op because
/// the library crate doesn't have access to the running GTK window.
///
/// This pattern — having a stub in `lib.rs` and the real function in `main.rs` — is common
/// in Rust projects where the binary has functionality that the library doesn't need.
pub fn update_all_buffer_themes(window: &impl IsA<gtk4::Widget>) {
    let _ = window;
}

// Common imports that tests might need
pub use gtk4;
pub use sourceview5;
