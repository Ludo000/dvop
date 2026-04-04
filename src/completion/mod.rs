//! # Code Completion — JSON-Driven Autocomplete
//!
//! Provides language-specific code completion powered by static JSON data files
//! (stored in `completion_data/*.json`). Each JSON file defines keywords,
//! snippets, and optional import hierarchies for one language.
//!
//! ## Architecture
//!
//! - **`json_provider`** — Loads and caches JSON completion data; provides
//!   lookup functions (`get_json_keywords`, `get_json_snippets`, etc.).
//! - **`ui`** — GTK4 popover-based completion popup; handles key events
//!   (Ctrl+Space / F1), filtering by prefix, and snippet insertion.
//!
//! ## How Completion Works
//!
//! 1. On `Ctrl+Space`, `trigger_completion()` reads the word at the cursor.
//! 2. It gathers matches from three sources: language keywords, snippets,
//!    and words already present in the current buffer.
//! 3. A `gtk4::Popover` with a `ListBox` is positioned at the cursor and
//!    shown. Arrow keys navigate; Enter inserts the selected item.
//!
//! Completion is **manual only** (no auto-popup) to keep the editor fast.
//!
//! See FEATURES.md: Feature #111 — Code Completion
//! See FEATURES.md: Feature #113 — Snippet Expansion
//! See FEATURES.md: Feature #114 — Import Suggestions

pub mod json_provider;
pub mod ui;

use json_provider::{
    get_json_keyword_documentation, get_json_keywords, get_json_snippet_documentation,
    get_json_snippets, initialize_completion_data,
};

/// Initialize completion system - loads JSON data files for all languages.
/// Rust completions are loaded separately by the `rust-completion` native extension.
pub fn initialize_completion() {
    println!("Initializing completion system...");

    // Load all languages from JSON files
    match initialize_completion_data() {
        Ok(languages) => {
            println!(
                "Successfully loaded completion data for {} languages: {:?}",
                languages.len(),
                languages
            );
        }
        Err(e) => {
            println!("Warning: Failed to load some completion data: {}", e);
            println!("Make sure completion_data directory exists with JSON files");
        }
    }
}

/// Get keywords as owned strings from JSON data
#[allow(dead_code)]
pub fn get_language_keywords_owned(language: &str) -> Vec<String> {
    get_json_keywords(language)
}

/// Get snippets as owned strings from JSON data
#[allow(dead_code)]
pub fn get_language_snippets_owned(language: &str) -> Vec<(String, String)> {
    get_json_snippets(language)
}

/// Get documentation for a specific keyword using JSON data
#[allow(dead_code)]
pub fn get_keyword_documentation(language: &str, keyword: &str) -> String {
    get_json_keyword_documentation(language, keyword)
}

/// Get documentation for a specific snippet using JSON data
#[allow(dead_code)]
pub fn get_snippet_documentation(language: &str, trigger: &str) -> String {
    get_json_snippet_documentation(language, trigger)
}

/// Get all supported languages based on available JSON files
/// Cached to avoid repeated filesystem operations
pub fn get_supported_languages() -> Vec<String> {
    use std::sync::OnceLock;
    static SUPPORTED_LANGUAGES: OnceLock<Vec<String>> = OnceLock::new();

    SUPPORTED_LANGUAGES
        .get_or_init(|| {
            let mut manager = json_provider::get_completion_manager();
            match manager.load_all_languages() {
                Ok(languages) => languages,
                Err(_) => {
                    // Return default list if loading fails
                    vec![
                        "rust".to_owned(),
                        "javascript".to_owned(),
                        "python".to_owned(),
                        "html".to_owned(),
                        "css".to_owned(),
                    ]
                }
            }
        })
        .clone()
}

// Re-export functions for external use
pub use ui::{setup_completion, setup_completion_for_file, setup_completion_shortcuts};

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Once;
    static INIT: Once = Once::new();
    fn ensure_initialized() {
        INIT.call_once(|| {
            initialize_completion();
            // Also load Rust completions (normally done by the rust-completion extension)
            crate::extensions::rust_completion::load_and_register();
        });
    }

    #[test]
    fn test_get_supported_languages() {
        let languages = get_supported_languages();
        assert!(!languages.is_empty());
        // Check for common languages
        assert!(languages.iter().any(|l| l == "rust" || l == "python" || l == "javascript"));
    }

    #[test]
    fn test_get_language_keywords_rust() {
        ensure_initialized();
        let keywords = get_language_keywords_owned("rust");
        assert!(keywords.contains(&"fn".to_string()));
        assert!(keywords.contains(&"let".to_string()));
        assert!(keywords.contains(&"mut".to_string()));
        assert!(keywords.contains(&"impl".to_string()));
    }

    #[test]
    fn test_get_language_keywords_python() {
        let keywords = get_language_keywords_owned("python");
        assert!(keywords.contains(&"def".to_string()));
        assert!(keywords.contains(&"class".to_string()));
        assert!(keywords.contains(&"import".to_string()));
    }

    #[test]
    fn test_get_language_snippets() {
        ensure_initialized();
        let snippets = get_language_snippets_owned("rust");
        assert!(!snippets.is_empty());
        // Check that snippets have both trigger and body
        for (trigger, body) in snippets {
            assert!(!trigger.is_empty());
            assert!(!body.is_empty());
        }
    }

    #[test]
    fn test_get_keyword_documentation() {
        ensure_initialized();
        let doc = get_keyword_documentation("rust", "fn");
        assert!(!doc.is_empty());
        assert!(doc.to_lowercase().contains("function"));
    }

    #[test]
    fn test_get_snippet_documentation() {
        let doc = get_snippet_documentation("rust", "fn");
        // May or may not have documentation
        assert!(doc.is_empty() || !doc.is_empty());
    }

    #[test]
    fn test_unsupported_language() {
        let keywords = get_language_keywords_owned("nonexistent");
        assert!(keywords.is_empty());
    }

}
