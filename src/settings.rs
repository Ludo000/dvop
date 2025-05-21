// User settings and preferences for the text editor
// Handles loading, saving, and accessing user configuration options

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use home::home_dir;

// Default settings values
const DEFAULT_LIGHT_THEME: &str = "solarized-light";
const DEFAULT_DARK_THEME: &str = "solarized-dark";

/// Represents user-configurable settings for the application
pub struct EditorSettings {
    // Store settings in a simple HashMap for flexibility
    values: HashMap<String, String>,
    // Path to the settings file
    config_path: PathBuf,
}

impl EditorSettings {
    /// Creates a new settings instance, loading from file if available
    pub fn new() -> Self {
        let config_dir = get_config_dir();
        let config_path = config_dir.join("settings.conf");

        // Create the config directory if it doesn't exist
        if !config_dir.exists() {
            if let Err(e) = fs::create_dir_all(&config_dir) {
                eprintln!("Failed to create config directory: {}", e);
            }
        }

        let mut settings = Self {
            values: HashMap::new(),
            config_path,
        };

        // Initialize with default values
        settings.set_defaults();
        
        // Try to load existing settings
        settings.load();
        
        settings
    }

    /// Sets up default values for all settings
    fn set_defaults(&mut self) {
        self.values.insert("light_theme".to_string(), DEFAULT_LIGHT_THEME.to_string());
        self.values.insert("dark_theme".to_string(), DEFAULT_DARK_THEME.to_string());
        // Add more default settings here as needed
    }

    /// Loads settings from the config file
    pub fn load(&mut self) {
        if !self.config_path.exists() {
            // No config file yet, just use defaults
            return;
        }

        // Simple line-based config file format: key=value
        match fs::read_to_string(&self.config_path) {
            Ok(contents) => {
                for line in contents.lines() {
                    if line.trim().starts_with('#') || line.trim().is_empty() {
                        continue; // Skip comments and empty lines
                    }

                    if let Some((key, value)) = line.split_once('=') {
                        self.values.insert(key.trim().to_string(), value.trim().to_string());
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to load settings: {}", e);
            }
        }
    }

    /// Saves current settings to the config file
    pub fn save(&self) -> Result<(), std::io::Error> {
        let mut contents = String::new();
        contents.push_str("# Text Editor Settings\n");
        contents.push_str("# Automatically generated - you can edit manually\n\n");

        for (key, value) in &self.values {
            contents.push_str(&format!("{}={}\n", key, value));
        }

        fs::write(&self.config_path, contents)
    }

    /// Gets a setting value as a string
    pub fn get(&self, key: &str) -> Option<&String> {
        self.values.get(key)
    }

    /// Sets a setting value
    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Gets the preferred light theme
    pub fn get_light_theme(&self) -> &str {
        self.get("light_theme").map_or(DEFAULT_LIGHT_THEME, |s| s.as_str())
    }

    /// Gets the preferred dark theme
    pub fn get_dark_theme(&self) -> &str {
        self.get("dark_theme").map_or(DEFAULT_DARK_THEME, |s| s.as_str())
    }

    /// Sets the preferred light theme
    pub fn set_light_theme(&mut self, theme: &str) {
        self.set("light_theme", theme);
    }

    /// Sets the preferred dark theme
    pub fn set_dark_theme(&mut self, theme: &str) {
        self.set("dark_theme", theme);
    }
}

/// Returns the configuration directory path
fn get_config_dir() -> PathBuf {
    // First try to use XDG_CONFIG_HOME
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = Path::new(&xdg_config).join("text-editor");
        return path;
    }
    
    // Then fall back to ~/.config/text-editor
    if let Some(home) = home_dir() {
        return home.join(".config").join("text-editor");
    }
    
    // Last resort: use the current directory
    PathBuf::from("./config")
}

// Global settings instance
static mut SETTINGS_INSTANCE: Option<EditorSettings> = None;

/// Initializes global settings
pub fn initialize_settings() {
    unsafe {
        SETTINGS_INSTANCE = Some(EditorSettings::new());
    }
}

/// Gets a reference to global settings
/// 
/// # Safety
/// This is unsafe because it accesses a static mutable variable.
/// It should only be called after initialize_settings() and in a context 
/// where there are no other references to SETTINGS_INSTANCE.
pub fn get_settings() -> &'static EditorSettings {
    unsafe {
        SETTINGS_INSTANCE.as_ref().expect("Settings not initialized")
    }
}

/// Gets a mutable reference to global settings
/// 
/// # Safety
/// This is unsafe for the same reasons as get_settings()
pub fn get_settings_mut() -> &'static mut EditorSettings {
    unsafe {
        SETTINGS_INSTANCE.as_mut().expect("Settings not initialized")
    }
}
