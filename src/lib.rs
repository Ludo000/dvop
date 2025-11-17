// Library interface for Dvop - exposes modules for testing
// This allows integration and E2E tests to access internal functionality

// Re-export main modules
pub mod handlers;
pub mod syntax;
pub mod utils;
pub mod settings;
pub mod search;
pub mod status_log;
pub mod file_cache;
pub mod audio;
pub mod video;
pub mod completion;
pub mod linter;
pub mod lsp;
pub mod ui;

// Re-export specific functions from main that are used by modules
// Note: In a refactor, these should be moved to appropriate modules
use gtk4::prelude::*;

pub fn update_all_buffer_themes(window: &impl IsA<gtk4::Widget>) {
    // This is a stub for library compilation
    // The actual implementation is in main.rs and only used in the binary
    // For testing purposes, we provide a no-op version
    let _ = window;
}

// Common imports that tests might need
pub use gtk4;
pub use sourceview5;
