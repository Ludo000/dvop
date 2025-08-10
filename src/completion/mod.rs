// Completion module for language-specific code completion
// This module provides intelligent code completion for various programming languages using JSON data

pub mod ui;
pub mod json_provider;

use json_provider::{get_json_keywords, get_json_snippets, get_json_keyword_documentation, get_json_snippet_documentation, initialize_completion_data};

/// Initialize completion system - loads JSON data and sets up providers
pub fn initialize_completion() {
    println!("Initializing JSON-based completion system...");
    
    match initialize_completion_data() {
        Ok(languages) => {
            println!("Successfully loaded completion data for {} languages: {:?}", 
                     languages.len(), languages);
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
pub fn get_supported_languages() -> Vec<String> {
    let mut manager = json_provider::get_completion_manager();
    match manager.load_all_languages() {
        Ok(languages) => languages,
        Err(_) => {
            // Return default list if loading fails
            vec![
                "rust".to_string(),
                "javascript".to_string(), 
                "python".to_string(),
                "html".to_string(),
                "css".to_string(),
            ]
        }
    }
}

// Re-export UI functions for external use
pub use ui::{setup_completion, setup_completion_for_file, setup_completion_shortcuts};
