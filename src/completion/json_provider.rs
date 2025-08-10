// JSON-based completion data loader
// Allows users to define completion data in JSON files for easy customization

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Represents a keyword completion item
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

/// Represents the complete completion data for a language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageCompletionData {
    pub language: String,
    pub description: String,
    pub keywords: Vec<KeywordData>,
    pub snippets: Vec<SnippetData>,
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
    
    /// Load completion data from JSON string
    pub fn from_json(json_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let language_data: LanguageCompletionData = serde_json::from_str(json_content)?;
        
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
    
    /// Get all keywords as string references (compatible with existing trait)
    pub fn keywords(&self) -> Vec<&str> {
        self.language_data.keywords.iter()
            .map(|k| k.keyword.as_str())
            .collect()
    }
    
    /// Get all snippets as (trigger, content) tuples (compatible with existing trait)
    pub fn snippets(&self) -> Vec<(&str, &str)> {
        self.language_data.snippets.iter()
            .map(|s| (s.trigger.as_str(), s.content.as_str()))
            .collect()
    }
    
    /// Get enhanced documentation for a keyword
    pub fn get_keyword_documentation(&self, keyword: &str) -> String {
        if let Some(keyword_data) = self.keyword_map.get(keyword) {
            format!("{} - {}\n\nCategory: {}\nExample: {}",
                keyword_data.keyword,
                keyword_data.description,
                keyword_data.category,
                keyword_data.example)
        } else {
            format!("{} - No documentation available", keyword)
        }
    }
    
    /// Get enhanced documentation for a snippet
    pub fn get_snippet_documentation(&self, trigger: &str) -> String {
        for snippet in &self.language_data.snippets {
            if snippet.trigger == trigger {
                return format!("{} (snippet) - {}\n\nCategory: {}",
                    snippet.trigger,
                    snippet.description,
                    snippet.category);
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
    
    /// Get language name
    pub fn get_language(&self) -> &str {
        &self.language_data.language
    }
    
    /// Get language description
    pub fn get_description(&self) -> &str {
        &self.language_data.description
    }
    
    /// Get keywords by category
    pub fn get_keywords_by_category(&self, category: &str) -> Vec<&KeywordData> {
        self.language_data.keywords.iter()
            .filter(|k| k.category == category)
            .collect()
    }
    
    /// Get snippets by category
    pub fn get_snippets_by_category(&self, category: &str) -> Vec<&SnippetData> {
        self.language_data.snippets.iter()
            .filter(|s| s.category == category)
            .collect()
    }
    
    /// Get all available categories for keywords
    pub fn get_keyword_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.language_data.keywords.iter()
            .map(|k| k.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }
    
    /// Get all available categories for snippets
    pub fn get_snippet_categories(&self) -> Vec<String> {
        let mut categories: Vec<String> = self.language_data.snippets.iter()
            .map(|s| s.category.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();
        categories.sort();
        categories
    }
}

/// Manager for loading and caching multiple language completion providers
pub struct CompletionDataManager {
    providers: HashMap<String, JsonCompletionProvider>,
    data_directory: String,
}

impl CompletionDataManager {
    /// Create a new manager with a data directory
    pub fn new(data_directory: impl Into<String>) -> Self {
        CompletionDataManager {
            providers: HashMap::new(),
            data_directory: data_directory.into(),
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
    
    /// Get provider for a language, loading it if necessary
    pub fn get_provider(&mut self, language: &str) -> Option<&JsonCompletionProvider> {
        // Try to load if not already loaded
        if !self.providers.contains_key(language) {
            if let Err(e) = self.load_language(language) {
                println!("Failed to load completion data for {}: {}", language, e);
                return None;
            }
        }
        
        self.providers.get(language)
    }
    
    /// Get mutable provider for a language
    pub fn get_provider_mut(&mut self, language: &str) -> Option<&mut JsonCompletionProvider> {
        // Try to load if not already loaded
        if !self.providers.contains_key(language) {
            if let Err(e) = self.load_language(language) {
                println!("Failed to load completion data for {}: {}", language, e);
                return None;
            }
        }
        
        self.providers.get_mut(language)
    }
    
    /// Check if language data is available
    pub fn has_language(&self, language: &str) -> bool {
        self.providers.contains_key(language)
    }
    
    /// Get list of loaded languages
    pub fn get_loaded_languages(&self) -> Vec<&str> {
        self.providers.keys().map(|s| s.as_str()).collect()
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
                        },
                        Err(e) => {
                            println!("Warning: Failed to load {}: {}", filename, e);
                        }
                    }
                }
            }
        }
        
        Ok(loaded_languages)
    }
    
    /// Reload a specific language from disk
    pub fn reload_language(&mut self, language: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.providers.remove(language);
        self.load_language(language)
    }
    
    /// Add custom language data from JSON string (for testing or dynamic data)
    pub fn add_language_from_json(&mut self, language: &str, json_content: &str) -> Result<(), Box<dyn std::error::Error>> {
        let provider = JsonCompletionProvider::from_json(json_content)?;
        self.providers.insert(language.to_string(), provider);
        Ok(())
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
pub fn get_json_keywords(language: &str) -> Vec<String> {
    let mut manager = get_completion_manager();
    
    if let Some(provider) = manager.get_provider(language) {
        provider.keywords().into_iter().map(|s| s.to_string()).collect()
    } else {
        Vec::new()
    }
}

/// Convenience function to get snippets for a language using JSON data
pub fn get_json_snippets(language: &str) -> Vec<(String, String)> {
    let mut manager = get_completion_manager();
    
    if let Some(provider) = manager.get_provider(language) {
        provider.snippets().into_iter()
            .map(|(trigger, content)| (trigger.to_string(), content.to_string()))
            .collect()
    } else {
        Vec::new()
    }
}

/// Convenience function to get keyword documentation using JSON data
pub fn get_json_keyword_documentation(language: &str, keyword: &str) -> String {
    let mut manager = get_completion_manager();
    
    if let Some(provider) = manager.get_provider(language) {
        provider.get_keyword_documentation(keyword)
    } else {
        format!("{} - No documentation available (language data not loaded)", keyword)
    }
}

/// Convenience function to get snippet documentation using JSON data
pub fn get_json_snippet_documentation(language: &str, trigger: &str) -> String {
    let mut manager = get_completion_manager();
    
    if let Some(provider) = manager.get_provider(language) {
        provider.get_snippet_documentation(trigger)
    } else {
        format!("{} (snippet) - No documentation available (language data not loaded)", trigger)
    }
}

/// Initialize and load all available completion data
pub fn initialize_completion_data() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut manager = get_completion_manager();
    manager.load_all_languages()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_json_provider_creation() {
        let json_data = r#"
        {
            "language": "test",
            "description": "Test language",
            "keywords": [
                {
                    "keyword": "test_keyword",
                    "type": "keyword",
                    "description": "A test keyword",
                    "example": "test_keyword value",
                    "category": "test"
                }
            ],
            "snippets": [
                {
                    "trigger": "test_snippet",
                    "description": "A test snippet",
                    "content": "test ${1:placeholder}",
                    "category": "test"
                }
            ]
        }
        "#;
        
        let provider = JsonCompletionProvider::from_json(json_data).unwrap();
        
        assert_eq!(provider.get_language(), "test");
        assert_eq!(provider.keywords().len(), 1);
        assert_eq!(provider.snippets().len(), 1);
        assert_eq!(provider.keywords()[0], "test_keyword");
    }
    
    #[test]
    fn test_manager_functionality() {
        let mut manager = CompletionDataManager::new("test_data");
        
        let json_data = r#"
        {
            "language": "test_lang",
            "description": "Test language for manager",
            "keywords": [
                {
                    "keyword": "manager_test",
                    "type": "keyword", 
                    "description": "Manager test keyword",
                    "example": "manager_test example",
                    "category": "test"
                }
            ],
            "snippets": []
        }
        "#;
        
        manager.add_language_from_json("test_lang", json_data).unwrap();
        
        assert!(manager.has_language("test_lang"));
        
        let provider = manager.get_provider("test_lang").unwrap();
        assert_eq!(provider.get_language(), "test_lang");
        assert_eq!(provider.keywords().len(), 1);
    }
}
