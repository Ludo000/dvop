// User settings and preferences for the text editor
// Handles loading, saving, and accessing user configuration options

use home::home_dir;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Default settings values
// Note: These are fallback values. The application will attempt to detect
// and use the OS default theme on first startup.
const FALLBACK_LIGHT_THEME: &str = "classic";
const FALLBACK_DARK_THEME: &str = "classic-dark";
pub const DEFAULT_FONT_SIZE: u32 = 11;
pub const DEFAULT_TERMINAL_FONT_SIZE: u32 = 11;
const DEFAULT_AUDIO_VOLUME: f64 = 0.8; // 80% volume
const DEFAULT_VIDEO_VOLUME: f64 = 0.8; // 80% volume
const DEFAULT_WINDOW_WIDTH: i32 = 800;
const DEFAULT_WINDOW_HEIGHT: i32 = 600;
const DEFAULT_FILE_PANEL_WIDTH: i32 = 200; // Width of file manager sidebar
const DEFAULT_TERMINAL_HEIGHT: i32 = 320; // Height of terminal section

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
        let _ = settings.load_from_file();

        settings
    }

    /// Sets up default values for all settings
    fn set_defaults(&mut self) {
        // Detect OS default themes on first startup
        let (default_light, default_dark) = detect_os_default_themes();
        self.values.insert("light_theme".to_owned(), default_light);
        self.values.insert("dark_theme".to_owned(), default_dark);
        self.values
            .insert("font_size".to_owned(), DEFAULT_FONT_SIZE.to_string());
        self.values.insert(
            "terminal_font_size".to_owned(),
            DEFAULT_TERMINAL_FONT_SIZE.to_string(),
        );
        self.values
            .insert("audio_volume".to_owned(), DEFAULT_AUDIO_VOLUME.to_string());
        self.values
            .insert("video_volume".to_owned(), DEFAULT_VIDEO_VOLUME.to_string());
        self.values
            .insert("window_width".to_owned(), DEFAULT_WINDOW_WIDTH.to_string());
        self.values.insert(
            "window_height".to_owned(),
            DEFAULT_WINDOW_HEIGHT.to_string(),
        );
        self.values.insert(
            "file_panel_width".to_owned(),
            DEFAULT_FILE_PANEL_WIDTH.to_string(),
        );
        self.values.insert(
            "terminal_height".to_owned(),
            DEFAULT_TERMINAL_HEIGHT.to_string(),
        );
        self.values
            .insert("active_sidebar_tab".to_owned(), "explorer".to_owned());
        self.values
            .insert("sidebar_visible".to_owned(), "true".to_owned());
        self.values
            .insert("terminal_visible".to_owned(), "false".to_owned());
        self.values
            .insert("search_case_sensitive".to_owned(), "false".to_owned());
        self.values
            .insert("search_whole_word".to_owned(), "false".to_owned());
        self.values.insert("search_query".to_owned(), "".to_owned());
        self.values.insert("opened_files".to_owned(), "".to_owned());
        // Default to home directory if not set
        if let Some(home) = home_dir() {
            self.values
                .insert("last_folder".to_owned(), home.to_string_lossy().to_string());
        }
        // Add more default settings here as needed
    }

    /// Loads settings from the config file
    fn load_from_file(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        match crate::file_cache::get_cached_file_content(&self.config_path) {
            Ok(content) => {
                // Parse the content line by line
                for line in content.lines() {
                    if let Some(eq_pos) = line.find('=') {
                        let key = line[..eq_pos].trim();
                        let value = line[eq_pos + 1..].trim();
                        self.values.insert(key.to_owned(), value.to_owned());
                    }
                }
                Ok(())
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // File doesn't exist, use defaults
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
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

        fs::write(&self.config_path, contents)?;

        // Invalidate the file cache so next load reads fresh from disk
        crate::file_cache::invalidate_file_cache(&self.config_path);

        Ok(())
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
        self.get("light_theme")
            .map_or_else(|| detect_os_default_themes().0, |s| s.clone())
    }

    /// Gets the preferred dark theme
    pub fn get_dark_theme(&self) -> String {
        self.get("dark_theme")
            .map_or_else(|| detect_os_default_themes().1, |s| s.clone())
    }

    /// Sets the preferred light theme
    pub fn set_light_theme(&mut self, theme: &str) {
        self.set("light_theme", theme);
    }

    /// Sets the preferred dark theme
    pub fn set_dark_theme(&mut self, theme: &str) {
        self.set("dark_theme", theme);
    }

    /// Gets the font size
    pub fn get_font_size(&self) -> u32 {
        self.get("font_size")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(DEFAULT_FONT_SIZE)
    }

    /// Sets the font size
    pub fn set_font_size(&mut self, size: u32) {
        self.set("font_size", &size.to_string());
    }

    /// Gets the terminal font size
    pub fn get_terminal_font_size(&self) -> u32 {
        self.get("terminal_font_size")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(DEFAULT_TERMINAL_FONT_SIZE)
    }

    /// Sets the terminal font size
    pub fn set_terminal_font_size(&mut self, size: u32) {
        self.set("terminal_font_size", &size.to_string());
    }

    /// Gets the audio volume (0.0 to 1.0)
    pub fn get_audio_volume(&self) -> f64 {
        self.get("audio_volume")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(DEFAULT_AUDIO_VOLUME)
            .max(0.0)
            .min(1.0) // Clamp to valid range
    }

    /// Sets the audio volume (0.0 to 1.0)
    pub fn set_audio_volume(&mut self, volume: f64) {
        let clamped_volume = volume.max(0.0).min(1.0);
        self.set("audio_volume", &clamped_volume.to_string());
    }

    /// Gets the window width
    pub fn get_window_width(&self) -> i32 {
        self.get("window_width")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_WINDOW_WIDTH)
            .max(400) // Minimum width
    }

    /// Sets the window width
    pub fn set_window_width(&mut self, width: i32) {
        let clamped_width = width.max(400); // Minimum width
        self.set("window_width", &clamped_width.to_string());
    }

    /// Gets the window height
    pub fn get_window_height(&self) -> i32 {
        self.get("window_height")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_WINDOW_HEIGHT)
            .max(300) // Minimum height
    }

    /// Sets the window height
    pub fn set_window_height(&mut self, height: i32) {
        let clamped_height = height.max(300); // Minimum height
        self.set("window_height", &clamped_height.to_string());
    }

    /// Sets both window dimensions at once
    pub fn set_window_size(&mut self, width: i32, height: i32) {
        self.set_window_width(width);
        self.set_window_height(height);
    }

    /// Gets the file panel width
    pub fn get_file_panel_width(&self) -> i32 {
        self.get("file_panel_width")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_FILE_PANEL_WIDTH)
            .max(100) // Minimum file panel width
    }

    /// Sets the file panel width
    pub fn set_file_panel_width(&mut self, width: i32) {
        let clamped_width = width.max(100); // Minimum file panel width
        self.set("file_panel_width", &clamped_width.to_string());
    }

    /// Gets the terminal height
    pub fn get_terminal_height(&self) -> i32 {
        self.get("terminal_height")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(DEFAULT_TERMINAL_HEIGHT)
            .max(100) // Minimum terminal height
    }

    /// Sets the terminal height
    pub fn set_terminal_height(&mut self, height: i32) {
        let clamped_height = height.max(100); // Minimum terminal height
        self.set("terminal_height", &clamped_height.to_string());
    }

    /// Sets both pane dimensions at once
    pub fn set_pane_dimensions(&mut self, file_panel_width: i32, terminal_height: i32) {
        self.set_file_panel_width(file_panel_width);
        self.set_terminal_height(terminal_height);
    }

    /// Gets the last used folder path
    pub fn get_last_folder(&self) -> PathBuf {
        self.get("last_folder")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                // Fallback to home directory
                home_dir().unwrap_or_else(|| PathBuf::from("."))
            })
    }

    /// Sets the last used folder path
    pub fn set_last_folder(&mut self, folder: &Path) {
        self.set("last_folder", &folder.to_string_lossy());
    }

    /// Gets the active sidebar tab ("explorer" or "search")
    pub fn get_active_sidebar_tab(&self) -> String {
        self.get("active_sidebar_tab")
            .map_or("explorer".to_string(), |s| s.clone())
    }

    /// Sets the active sidebar tab
    pub fn set_active_sidebar_tab(&mut self, tab: &str) {
        self.set("active_sidebar_tab", tab);
    }

    /// Gets the sidebar visibility state
    pub fn get_sidebar_visible(&self) -> bool {
        self.get("sidebar_visible").is_none_or(|s| s == "true")
    }

    /// Sets the sidebar visibility state
    pub fn set_sidebar_visible(&mut self, visible: bool) {
        self.set("sidebar_visible", if visible { "true" } else { "false" });
    }

    /// Gets the terminal visibility state
    pub fn get_terminal_visible(&self) -> bool {
        self.get("terminal_visible").is_some_and(|s| s == "true")
    }

    /// Sets the terminal visibility state
    pub fn set_terminal_visible(&mut self, visible: bool) {
        self.set("terminal_visible", if visible { "true" } else { "false" });
    }

    /// Gets the search case sensitive setting
    pub fn get_search_case_sensitive(&self) -> bool {
        self.get("search_case_sensitive")
            .is_some_and(|s| s == "true")
    }

    /// Sets the search case sensitive setting
    pub fn set_search_case_sensitive(&mut self, case_sensitive: bool) {
        self.set(
            "search_case_sensitive",
            if case_sensitive { "true" } else { "false" },
        );
    }

    /// Gets the search whole word setting
    pub fn get_search_whole_word(&self) -> bool {
        self.get("search_whole_word").is_some_and(|s| s == "true")
    }

    /// Sets the search whole word setting
    pub fn set_search_whole_word(&mut self, whole_word: bool) {
        self.set(
            "search_whole_word",
            if whole_word { "true" } else { "false" },
        );
    }

    /// Gets the last search query
    pub fn get_search_query(&self) -> String {
        self.get("search_query")
            .map_or(String::new(), |s| s.clone())
    }

    /// Sets the last search query
    pub fn set_search_query(&mut self, query: &str) {
        self.set("search_query", query);
    }

    /// Gets the list of opened files (pipe-separated paths)
    pub fn get_opened_files(&self) -> Vec<PathBuf> {
        self.get("opened_files")
            .map(|s| {
                if s.is_empty() {
                    Vec::new()
                } else {
                    s.split('|')
                        .filter(|p| !p.is_empty())
                        .map(PathBuf::from)
                        .collect()
                }
            })
            .unwrap_or_default()
    }

    /// Sets the list of opened files
    pub fn set_opened_files(&mut self, files: &[PathBuf]) {
        let files_str = files
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect::<Vec<_>>()
            .join("|");
        self.set("opened_files", &files_str);
    }

    /// Gets the configuration directory path
    pub fn config_dir(&self) -> PathBuf {
        get_config_dir()
    }

    /// Gets the saved git commit message
    pub fn get_git_commit_message(&self) -> String {
        self.get("git_commit_message")
            .map_or(String::new(), |s| s.clone())
    }

    /// Sets the git commit message to save
    pub fn set_git_commit_message(&mut self, message: &str) {
        self.set("git_commit_message", message);
    }
}

/// Helper function to save the current folder to settings
/// This should be called whenever the current directory changes
#[allow(dead_code)]
pub fn save_current_folder(folder: &Path) {
    let mut settings = get_settings_mut();
    settings.set_last_folder(folder);
    // Don't save immediately to avoid too many disk writes
    // The folder will be saved on app close
}

/// Returns the configuration directory path
fn get_config_dir() -> PathBuf {
    // First try to use XDG_CONFIG_HOME
    if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        let path = Path::new(&xdg_config).join("dvop");
        return path;
    }

    // Then fall back to ~/.config/dvop
    if let Some(home) = home::home_dir() {
        return home.join(".config").join("dvop");
    }

    // Last resort: use the current directory
    PathBuf::from("./config")
}

/// Returns the configuration directory path (public function)
pub fn get_config_dir_public() -> PathBuf {
    get_config_dir()
}

use once_cell::sync::Lazy;
use std::sync::{Mutex, Once};

// Global settings instance using thread-safe patterns
static SETTINGS_INSTANCE: Lazy<Mutex<EditorSettings>> =
    Lazy::new(|| Mutex::new(EditorSettings::new()));
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

use std::cell::Cell;

// Used to prevent recursive calls to refresh_settings
thread_local! {
    static REFRESHING: Cell<bool> = const { Cell::new(false) };
}

/// Forces a reload of settings and triggers updates
///
/// This function should be called after settings have been changed and saved
pub fn refresh_settings() {
    // Prevent recursive calls
    if REFRESHING.with(|flag| flag.get()) {
        return;
    }

    REFRESHING.with(|flag| flag.set(true));

    // Lock the settings instance
    let mut settings = SETTINGS_INSTANCE.lock().unwrap();

    // Reload settings from disk
    let _ = settings.load_from_file();

    // Print some debugging info about the current themes
    println!("Settings refreshed:");
    println!("  Light theme: {}", settings.get_light_theme());
    println!("  Dark theme: {}", settings.get_dark_theme());

    // Reset the refreshing flag
    REFRESHING.with(|flag| flag.set(false));
}

/// Detects the OS default theme based on the current GTK theme
///
/// Returns a tuple of (light_theme, dark_theme) based on what's available
/// and what the OS is currently using. Prioritizes OS-native themes.
fn detect_os_default_themes() -> (String, String) {
    // Try to detect the current GTK theme name
    let gtk_theme = std::env::var("GTK_THEME")
        .or_else(|_| {
            // Try to get from GIO settings for GNOME
            use gtk4::gio::prelude::*;
            match std::panic::catch_unwind(|| {
                gtk4::gio::Settings::new("org.gnome.desktop.interface")
            }) {
                Ok(gio_settings) => {
                    if let Some(schema) = gio_settings.settings_schema() {
                        if schema.has_key("gtk-theme") {
                            return Ok(gio_settings.string("gtk-theme").to_string());
                        }
                    }
                    Err(())
                }
                Err(_) => Err(()),
            }
        })
        .unwrap_or_else(|_| String::from(""));

    let theme_lower = gtk_theme.to_lowercase();

    // Determine the appropriate light/dark theme pair based on the OS theme
    let (light, dark) = if theme_lower.contains("yaru") {
        ("Yaru", "Yaru-dark")
    } else if theme_lower.contains("adwaita") {
        ("Adwaita", "Adwaita-dark")
    } else {
        // Check desktop environment for better defaults
        let desktop_env = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

        if desktop_env.contains("GNOME") || desktop_env.contains("Ubuntu") {
            // Ubuntu/GNOME: prefer Yaru on Ubuntu, Adwaita on GNOME
            if desktop_env.contains("Ubuntu") {
                ("Yaru", "Yaru-dark")
            } else {
                ("Adwaita", "Adwaita-dark")
            }
        } else if desktop_env.contains("KDE") {
            ("kate", "kate-dark")
        } else {
            // Generic fallback to classic themes (available everywhere)
            (FALLBACK_LIGHT_THEME, FALLBACK_DARK_THEME)
        }
    };

    println!(
        "Detected OS default themes: Light='{}', Dark='{}'",
        light, dark
    );
    (light.to_string(), dark.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_settings_creation() {
        let settings = EditorSettings::new();
        assert!(settings.get_font_size() >= 8);
        assert!(settings.get_window_width() > 0);
        assert!(settings.get_window_height() > 0);
    }

    #[test]
    fn test_font_size_validation() {
        let mut settings = EditorSettings::new();
        
        // Valid font size
        settings.set_font_size(14);
        assert_eq!(settings.get_font_size(), 14);
        
        // Can set any value
        settings.set_font_size(4);
        assert_eq!(settings.get_font_size(), 4);
        
        settings.set_font_size(100);
        assert_eq!(settings.get_font_size(), 100);
    }

    #[test]
    fn test_terminal_font_size() {
        let mut settings = EditorSettings::new();
        
        settings.set_terminal_font_size(12);
        assert_eq!(settings.get_terminal_font_size(), 12);
        
        settings.set_terminal_font_size(8);
        assert_eq!(settings.get_terminal_font_size(), 8);
    }

    #[test]
    fn test_audio_volume() {
        let mut settings = EditorSettings::new();
        
        settings.set_audio_volume(0.5);
        assert_eq!(settings.get_audio_volume(), 0.5);
        
        // Test clamping to 0.0-1.0 range
        settings.set_audio_volume(-0.5);
        assert_eq!(settings.get_audio_volume(), 0.0);
        
        settings.set_audio_volume(2.0);
        assert_eq!(settings.get_audio_volume(), 1.0);
    }

    #[test]
    fn test_window_dimensions() {
        let mut settings = EditorSettings::new();
        
        settings.set_window_size(1024, 768);
        assert_eq!(settings.get_window_width(), 1024);
        assert_eq!(settings.get_window_height(), 768);
        
        // Individual setters
        settings.set_window_width(1920);
        assert_eq!(settings.get_window_width(), 1920);
        
        settings.set_window_height(1080);
        assert_eq!(settings.get_window_height(), 1080);
    }

    #[test]
    fn test_pane_dimensions() {
        let mut settings = EditorSettings::new();
        
        settings.set_pane_dimensions(250, 200);
        assert_eq!(settings.get_file_panel_width(), 250);
        assert_eq!(settings.get_terminal_height(), 200);
        
        // Individual setters
        settings.set_file_panel_width(300);
        assert_eq!(settings.get_file_panel_width(), 300);
        
        settings.set_terminal_height(150);
        assert_eq!(settings.get_terminal_height(), 150);
    }

    #[test]
    fn test_theme_settings() {
        let mut settings = EditorSettings::new();
        
        settings.set_light_theme("solarized-light");
        assert_eq!(settings.get_light_theme(), "solarized-light");
        
        settings.set_dark_theme("solarized-dark");
        assert_eq!(settings.get_dark_theme(), "solarized-dark");
    }

    #[test]
    fn test_last_folder() {
        let mut settings = EditorSettings::new();
        let temp_dir = TempDir::new().unwrap();
        
        settings.set_last_folder(temp_dir.path());
        assert_eq!(settings.get_last_folder(), temp_dir.path());
    }

    #[test]
    fn test_opened_files_management() {
        let mut settings = EditorSettings::new();
        
        let file1 = PathBuf::from("/path/to/file1.rs");
        let file2 = PathBuf::from("/path/to/file2.rs");
        
        settings.set_opened_files(&[file1.clone(), file2.clone()]);
        
        let opened = settings.get_opened_files();
        assert_eq!(opened.len(), 2);
        assert!(opened.contains(&file1));
        assert!(opened.contains(&file2));
        
        // Remove one file
        settings.set_opened_files(&[file2.clone()]);
        let opened = settings.get_opened_files();
        assert_eq!(opened.len(), 1);
        assert!(!opened.contains(&file1));
        assert!(opened.contains(&file2));
    }

    #[test]
    fn test_config_dir_creation() {
        let config_dir = get_config_dir_public();
        assert!(config_dir.is_absolute());
    }

    #[test]
    fn test_default_themes_detection() {
        let (light, dark) = detect_os_default_themes();
        assert!(!light.is_empty());
        assert!(!dark.is_empty());
    }
}
