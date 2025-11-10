// Completion module for language-specific code completion
// This module provides intelligent code completion for various programming languages using JSON data

pub mod json_provider;
pub mod ui;

use json_provider::{
    find_matching_modules, get_import_suggestions, get_json_keyword_documentation,
    get_json_keywords, get_json_snippet_documentation, get_json_snippets, get_submodules,
    initialize_completion_data, ImportItem,
};

/// Initialize completion system - loads JSON data and sets up providers
pub fn initialize_completion() {
    println!("Initializing JSON-based completion system...");

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
pub fn get_language_keywords_owned(language: &str) -> Vec<String> {
    get_json_keywords(language)
}

/// Get snippets as owned strings from JSON data
pub fn get_language_snippets_owned(language: &str) -> Vec<(String, String)> {
    get_json_snippets(language)
}

/// Get documentation for a specific keyword using JSON data
pub fn get_keyword_documentation(language: &str, keyword: &str) -> String {
    get_json_keyword_documentation(language, keyword)
}

/// Get documentation for a specific snippet using JSON data
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

/// Get import suggestions for a specific module path
pub fn get_import_completions(language: &str, module_path: &str) -> Vec<ImportItem> {
    get_import_suggestions(language, module_path)
}

/// Get available submodules for a module path
pub fn get_available_submodules(language: &str, module_path: &str) -> Vec<String> {
    get_submodules(language, module_path)
}

/// Find modules that match a partial import path
pub fn find_modules_by_prefix(language: &str, partial_path: &str) -> Vec<String> {
    find_matching_modules(language, partial_path)
}

// Re-export functions for external use
pub use ui::{setup_completion, setup_completion_for_file, setup_completion_shortcuts};
