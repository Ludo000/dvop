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
#[derive(Clone)]
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
    pub fn get_light_theme(&self) -> String {
        self.get("light_theme").map_or(DEFAULT_LIGHT_THEME.to_string(), |s| s.clone())
    }

    /// Gets the preferred dark theme
    pub fn get_dark_theme(&self) -> String {
        self.get("dark_theme").map_or(DEFAULT_DARK_THEME.to_string(), |s| s.clone())
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
        let path = Path::new(&xdg_config).join("basado-text-editor");
        return path;
    }
    
    // Then fall back to ~/.config/basado-text-editor
    if let Some(home) = home_dir() {
        return home.join(".config").join("basado-text-editor");
    }
    
    // Last resort: use the current directory
    PathBuf::from("./config")
}

use std::sync::{Mutex, Once};
use once_cell::sync::Lazy;

// Global settings instance using thread-safe patterns
static SETTINGS_INSTANCE: Lazy<Mutex<EditorSettings>> = Lazy::new(|| {
    Mutex::new(EditorSettings::new())
});
static INIT: Once = Once::new();

/// Initializes global settings
pub fn initialize_settings() {
    // This ensures initialization happens only once
    INIT.call_once(|| {
        // The initialization happens in the Lazy::new above
        // We just need to ensure it's called
        let _ = &SETTINGS_INSTANCE;
    });
}

/// Gets the settings or a temporary clone of the settings for read operations
///
/// This creates a fresh copy of the settings each time to ensure we get the latest values.
/// Any changes made through get_settings_mut() will be reflected in subsequent get_settings() calls.
pub fn get_settings() -> EditorSettings {
    // Ensure settings are initialized
    initialize_settings();
    
    // Get a fresh clone of the settings
    SETTINGS_INSTANCE.lock().unwrap().clone()
}

/// Updates and returns the mutable settings
/// 
/// This function locks the mutex to perform changes and returns a mutable
/// reference to the settings. Call save() afterwards to persist changes.
pub fn get_settings_mut() -> std::sync::MutexGuard<'static, EditorSettings> {
    initialize_settings();
    SETTINGS_INSTANCE.lock().unwrap()
}

// Used to prevent recursive calls to refresh_settings
static mut REFRESHING: bool = false;

/// Forces a reload of settings and triggers updates
/// 
/// This function should be called after settings have been changed and saved
pub fn refresh_settings() {
    // Prevent recursive calls
    unsafe {
        if REFRESHING {
            return;
        }
        REFRESHING = true;
    }
    
    // Lock the settings instance
    let mut settings = SETTINGS_INSTANCE.lock().unwrap();
    
    // Reload settings from disk
    settings.load();
    
    // Print some debugging info about the current themes
    println!("Settings refreshed:");
    println!("  Light theme: {}", settings.get_light_theme());
    println!("  Dark theme: {}", settings.get_dark_theme());
    
    // Reset the refreshing flag
    unsafe {
        REFRESHING = false;
    }
}
