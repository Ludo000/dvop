//! # JSON Completion Provider — Data Loader & Cache
//!
//! Reads `completion_data/*.json` files at startup and caches them in memory.
//! Each JSON file follows the `LanguageCompletionData` schema (keywords,
//! snippets, imports). A global `CompletionDataManager` (behind a `Mutex`)
//! holds all loaded providers.
//!
//! ## Adding a New Language
//!
//! 1. Create `completion_data/<lang>.json` following the schema (see
//!    `completion_data/README.md` and existing files for examples).
//! 2. Call `initialize_completion_data()` at startup — it auto-discovers
//!    every `.json` file in the `completion_data/` directory.
//!
//! See FEATURES.md: Feature #111 — Code Completion
//! See FEATURES.md: Feature #112 — Multi-Language Completion Data

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// One keyword entry from a language’s JSON file.
///
/// Example JSON:
/// ```json
/// { "keyword": "fn", "type": "keyword", "description": "Declares a function",
///   "example": "fn main() {}", "category": "Functions" }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeywordData {
    pub keyword: String,
    pub r#type: String,
    pub description: String,
    pub example: String,
    pub category: String,
}

/// Represents a snippet completion item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnippetData {
    pub trigger: String,
    pub description: String,
    pub content: String,
    pub category: String,
}

/// Represents import completion data for a module path
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ImportData {
    pub module_path: String,
    pub items: Vec<ImportItem>,
    pub description: String,
}

/// Represents an importable item from a module
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportItem {
    pub name: String,
    pub item_type: String, // "function", "struct", "enum", "trait", "const", "module"
    pub description: String,
}

/// Represents module hierarchy for import suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleHierarchy {
    pub modules: Vec<ModuleData>,
}

/// Represents a module and its available items
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleData {
    pub path: String,
    pub items: Vec<ImportItem>,
    pub submodules: Vec<String>,
}

/// Represents the complete completion data for a language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCompletionData {
    pub language: String,
    pub description: String,
    pub keywords: Vec<KeywordData>,
    pub snippets: Vec<SnippetData>,
    pub imports: Option<ModuleHierarchy>, // Import data for the language
}

/// JSON-based completion provider
pub struct JsonCompletionProvider {
    language_data: LanguageCompletionData,
    keyword_map: HashMap<String, KeywordData>,
}

impl JsonCompletionProvider {
    /// Load completion data from a JSON file
    pub fn from_file(file_path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let json_content = fs::read_to_string(file_path)?;
        let language_data: LanguageCompletionData = serde_json::from_str(&json_content)?;

        // Create a HashMap for quick keyword lookups
        let mut keyword_map = HashMap::new();
        for keyword_data in &language_data.keywords {
            keyword_map.insert(keyword_data.keyword.clone(), keyword_data.clone());
        }

        Ok(JsonCompletionProvider {
            language_data,
            keyword_map,
        })
    }

    /// Access the underlying language data.
    pub fn language_data(&self) -> &LanguageCompletionData {
        &self.language_data
    }

    /// Get all keywords as string references (compatible with existing trait)
    pub fn keywords(&self) -> Vec<&str> {
        self.language_data
            .keywords
            .iter()
            .map(|k| k.keyword.as_str())
            .collect()
    }

    /// Get all snippets as (trigger, content) tuples (compatible with existing trait)
    pub fn snippets(&self) -> Vec<(&str, &str)> {
        self.language_data
            .snippets
            .iter()
            .map(|s| (s.trigger.as_str(), s.content.as_str()))
            .collect()
    }

    /// Get enhanced documentation for a keyword
    pub fn get_keyword_documentation(&self, keyword: &str) -> String {
        if let Some(keyword_data) = self.keyword_map.get(keyword) {
            format!(
                "{} - {}\n\nCategory: {}\nExample: {}",
                keyword_data.keyword,
                keyword_data.description,
                keyword_data.category,
                keyword_data.example
            )
        } else {
            format!("{} - No documentation available", keyword)
        }
    }

    /// Get enhanced documentation for a snippet
    pub fn get_snippet_documentation(&self, trigger: &str) -> String {
        for snippet in &self.language_data.snippets {
            if snippet.trigger == trigger {
                return format!(
                    "{} (snippet) - {}\n\nCategory: {}",
                    snippet.trigger, snippet.description, snippet.category
                );
            }
        }
        format!("{} (snippet) - No documentation available", trigger)
    }

    /// Get all keyword data for advanced functionality
    pub fn get_keyword_data(&self) -> &[KeywordData] {
        &self.language_data.keywords
    }

    /// Get all snippet data for advanced functionality
    pub fn get_snippet_data(&self) -> &[SnippetData] {
        &self.language_data.snippets
    }

    /// Get import suggestions for a given module path
    pub fn get_import_suggestions(&self, module_path: &str) -> Vec<ImportItem> {
        if let Some(imports) = &self.language_data.imports {
            for module in &imports.modules {
                if module.path == module_path {
                    return module.items.clone();
                }
            }
        }
        Vec::new()
    }

    /// Get available submodules for a given module path
    pub fn get_submodules(&self, module_path: &str) -> Vec<String> {
        if let Some(imports) = &self.language_data.imports {
            for module in &imports.modules {
                if module.path == module_path {
                    return module.submodules.clone();
                }
            }
        }
        Vec::new()
    }

    /// Find best matching module path for partial import
    pub fn find_matching_modules(&self, partial_path: &str) -> Vec<String> {
        if let Some(imports) = &self.language_data.imports {
            return imports
                .modules
                .iter()
                .map(|module| &module.path)
                .filter(|path| path.starts_with(partial_path)).cloned()
                .collect();
        }
        Vec::new()
    }
}

/// Manages all loaded `JsonCompletionProvider` instances, keyed by language name.
///
/// Held behind a global `Lazy<Mutex<...>>` so any thread can look up completion
/// data. Languages are loaded lazily on first access via `ensure_language_loaded()`.
pub struct CompletionDataManager {
    providers: HashMap<String, JsonCompletionProvider>,
    data_directory: String,
    /// Languages explicitly removed by an extension disable; prevents auto-reload from disk.
    blocked: HashSet<String>,
}

impl CompletionDataManager {
    /// Create a new manager with a data directory
    pub fn new(data_directory: impl Into<String>) -> Self {
        CompletionDataManager {
            providers: HashMap::new(),
            data_directory: data_directory.into(),
            blocked: HashSet::new(),
        }
    }

    /// Load completion data for a language
    pub fn load_language(&mut self, language: &str) -> Result<(), Box<dyn std::error::Error>> {
        let file_path = Path::new(&self.data_directory).join(format!("{}.json", language));

        if file_path.exists() {
            let provider = JsonCompletionProvider::from_file(&file_path)?;
            self.providers.insert(language.to_string(), provider);
            println!("Loaded JSON completion data for language: {}", language);
        } else {
            return Err(format!("Completion data file not found: {:?}", file_path).into());
        }

        Ok(())
    }

    /// Get provider for a language, loading it if necessary.
    /// Returns `None` for languages that were explicitly blocked via `remove_provider`.
    pub fn get_provider(&mut self, language: &str) -> Option<&JsonCompletionProvider> {
        // A blocked language was explicitly removed — do not auto-load from disk.
        if self.blocked.contains(language) {
            return None;
        }
        // Try to load if not already loaded
        if !self.providers.contains_key(language) {
            if let Err(_) = self.load_language(language) {
                return None;
            }
        }

        self.providers.get(language)
    }

    /// Load all available language files in the data directory
    pub fn load_all_languages(&mut self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let data_dir = Path::new(&self.data_directory);
        let mut loaded_languages = Vec::new();

        if !data_dir.exists() {
            return Err(format!("Data directory does not exist: {:?}", data_dir).into());
        }

        for entry in fs::read_dir(data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(filename) = path.file_stem().and_then(|s| s.to_str()) {
                    match self.load_language(filename) {
                        Ok(_) => {
                            loaded_languages.push(filename.to_string());
                        }
                        Err(e) => {
                            println!("Warning: Failed to load {}: {}", filename, e);
                        }
                    }
                }
            }
        }

        Ok(loaded_languages)
    }

    /// Merge additional language data into an existing provider.
    /// New keywords, snippets, and import modules are appended without
    /// overwriting entries that already exist.
    pub fn merge_language_data(&mut self, language: &str, extra: LanguageCompletionData) {
        if let Some(provider) = self.providers.get_mut(language) {
            // Merge keywords (skip duplicates by keyword name)
            for kw in extra.keywords {
                if !provider.keyword_map.contains_key(&kw.keyword) {
                    provider.keyword_map.insert(kw.keyword.clone(), kw.clone());
                    provider.language_data.keywords.push(kw);
                }
            }
            // Merge snippets (skip duplicates by trigger)
            let existing_triggers: std::collections::HashSet<String> = provider
                .language_data
                .snippets
                .iter()
                .map(|s| s.trigger.clone())
                .collect();
            for snippet in extra.snippets {
                if !existing_triggers.contains(&snippet.trigger) {
                    provider.language_data.snippets.push(snippet);
                }
            }
            // Merge import modules
            if let Some(extra_hierarchy) = extra.imports {
                if let Some(ref mut existing) = provider.language_data.imports {
                    let existing_paths: std::collections::HashSet<String> = existing
                        .modules
                        .iter()
                        .map(|m| m.path.clone())
                        .collect();
                    for module in extra_hierarchy.modules {
                        if !existing_paths.contains(&module.path) {
                            existing.modules.push(module);
                        }
                    }
                } else {
                    provider.language_data.imports = Some(extra_hierarchy);
                }
            }
        } else {
            // No existing provider — just add it directly
            self.add_language_data(language, extra);
        }
    }

    /// Remove a language provider entirely and block auto-reload from disk.
    /// Use `add_language_data` to re-add and unblock.
    pub fn remove_provider(&mut self, language: &str) {
        self.providers.remove(language);
        self.blocked.insert(language.to_string());
    }

    /// Add pre-built language data directly (used by rust_completion extension).
    /// Also unblocks the language if it was previously removed.
    pub fn add_language_data(&mut self, language: &str, data: LanguageCompletionData) {
        self.blocked.remove(language);
        let mut keyword_map = HashMap::new();
        for keyword_data in &data.keywords {
            keyword_map.insert(keyword_data.keyword.clone(), keyword_data.clone());
        }
        let provider = JsonCompletionProvider {
            language_data: data,
            keyword_map,
        };
        self.providers.insert(language.to_string(), provider);
    }
}

// Global completion data manager instance
lazy_static::lazy_static! {
    static ref COMPLETION_MANAGER: std::sync::Mutex<CompletionDataManager> = {
        // Try to find the completion_data directory relative to the executable or current directory
        let possible_paths = [
            "completion_data",
            "../completion_data",
            "../../completion_data",
            "./src/completion_data",
            "./completion_data",
        ];

        let data_dir = possible_paths.iter()
            .find(|path| Path::new(path).exists())
            .unwrap_or(&"completion_data")
            .to_string();

        println!("Using completion data directory: {}", data_dir);
        std::sync::Mutex::new(CompletionDataManager::new(data_dir))
    };
}

/// Get global completion data manager instance
pub fn get_completion_manager() -> std::sync::MutexGuard<'static, CompletionDataManager> {
    COMPLETION_MANAGER.lock().unwrap()
}

/// Convenience function to get keywords for a language using JSON data
#[allow(dead_code)]
pub fn get_json_keywords(language: &str) -> Vec<String> {
    let mut manager = get_completion_manager();

    if let Some(provider) = manager.get_provider(language) {
        provider
            .keywords()
            .into_iter()
            .map(|s| s.to_string())
            .collect()
    } else {
        Vec::new()
    }
}

/// Convenience function to get snippets for a language using JSON data
#[allow(dead_code)]
pub fn get_json_snippets(language: &str) -> Vec<(String, String)> {
    let mut manager = get_completion_manager();

    if let Some(provider) = manager.get_provider(language) {
        provider
            .snippets()
            .into_iter()
            .map(|(trigger, content)| (trigger.to_string(), content.to_string()))
            .collect()
    } else {
        Vec::new()
    }
}

/// Convenience function to get keyword documentation using JSON data
#[allow(dead_code)]
pub fn get_json_keyword_documentation(language: &str, keyword: &str) -> String {
    let mut manager = get_completion_manager();

    if let Some(provider) = manager.get_provider(language) {
        provider.get_keyword_documentation(keyword)
    } else {
        format!(
            "{} - No documentation available (language data not loaded)",
            keyword
        )
    }
}

/// Convenience function to get snippet documentation using JSON data
#[allow(dead_code)]
pub fn get_json_snippet_documentation(language: &str, trigger: &str) -> String {
    let mut manager = get_completion_manager();

    if let Some(provider) = manager.get_provider(language) {
        provider.get_snippet_documentation(trigger)
    } else {
        format!(
            "{} (snippet) - No documentation available (language data not loaded)",
            trigger
        )
    }
}

/// Initialize and load all available completion data
pub fn initialize_completion_data() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut manager = get_completion_manager();
    manager.load_all_languages()
}


