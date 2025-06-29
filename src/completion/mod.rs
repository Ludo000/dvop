// Completion module for language-specific code completion
// This module provides intelligent code completion for various programming languages

pub mod rust_lang;
pub mod javascript_lang;
pub mod python_lang;
pub mod c_lang;
pub mod cpp_lang;
pub mod java_lang;
pub mod html_lang;
pub mod css_lang;
pub mod ui;

/// Language completion provider trait
pub trait LanguageProvider {
    fn keywords(&self) -> &[&'static str];
    fn snippets(&self) -> &[(&'static str, &'static str)];
    fn get_documentation(&self, keyword: &str) -> String;
}

/// Get the appropriate language provider for a given language
pub fn get_language_provider(language: &str) -> Option<Box<dyn LanguageProvider>> {
    match language {
        "rust" => Some(Box::new(rust_lang::RustProvider)),
        "javascript" => Some(Box::new(javascript_lang::JavaScriptProvider)),
        "python" => Some(Box::new(python_lang::PythonProvider)),
        "c" => Some(Box::new(c_lang::CProvider)),
        "cpp" => Some(Box::new(cpp_lang::CppProvider)),
        "java" => Some(Box::new(java_lang::JavaProvider)),
        "html" => Some(Box::new(html_lang::HtmlProvider)),
        "css" => Some(Box::new(css_lang::CssProvider)),
        _ => None,
    }
}

/// Get all keywords for a language
pub fn get_language_keywords(language: &str) -> Vec<&'static str> {
    if let Some(provider) = get_language_provider(language) {
        provider.keywords().to_vec()
    } else {
        vec![]
    }
}

/// Get all snippets for a language
pub fn get_language_snippets(language: &str) -> Vec<(&'static str, &'static str)> {
    if let Some(provider) = get_language_provider(language) {
        provider.snippets().to_vec()
    } else {
        vec![]
    }
}

/// Get documentation for a specific keyword in a language
pub fn get_keyword_documentation(language: &str, keyword: &str) -> String {
    // Forward to the enhanced documentation system in ui.rs
    ui::get_completion_documentation(keyword, language)
}

/// Get all supported languages
pub fn get_supported_languages() -> Vec<&'static str> {
    vec!["rust", "javascript", "python", "c", "cpp", "java", "html", "css"]
}

// Re-export UI functions for external use
pub use ui::{setup_completion, setup_completion_for_file, setup_completion_shortcuts};
