// Syntax highlighting functionality for the text editor
// This module manages syntax highlighting based on file types

use sourceview5::{prelude::*, Buffer, LanguageManager, StyleSchemeManager, View};
use gtk4::ScrolledWindow;
use gtk4::Settings;
use std::path::Path;

/// Determines whether the system is using a dark theme
/// 
/// Checks the GTK settings and environment to determine if the system prefers dark mode
pub fn is_dark_mode_enabled() -> bool {
    
    // Check for desktop environment specific settings FIRST (more reliable than GTK settings)
    let desktop_env = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

    if desktop_env.contains("GNOME") || desktop_env.contains("Unity") {
        // Try GIO Settings first (more reliable than gsettings command)
        use gtk4::gio::prelude::*;
        // GIO Settings new() can panic if schema doesn't exist, so we need to wrap it carefully
        match std::panic::catch_unwind(|| gtk4::gio::Settings::new("org.gnome.desktop.interface")) {
            Ok(gio_settings) => {
                // Check the new color-scheme setting (Ubuntu 22.04+)
                let color_scheme = gio_settings.string("color-scheme");
                if color_scheme.as_str() == "prefer-dark" {
                    return true;
                }
                if color_scheme.as_str() == "prefer-light" || color_scheme.as_str() == "default" {
                    return false; // Explicitly return false for light themes - this is definitive
                }
                
                // Check the gtk-theme setting as fallback
                let gtk_theme = gio_settings.string("gtk-theme");
                let theme_lower = gtk_theme.to_lowercase();
                if theme_lower.contains("dark") {
                    return true;
                } else if theme_lower == "yaru" || theme_lower == "adwaita" || theme_lower.contains("light") {
                    return false; // Explicitly return false for known light themes
                }
            },
            Err(_) => {
                // Schema not available, continue to fallback methods
            }
        }
        
        // Legacy method: Try gsettings command
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("dark") {
                return true;
            } else if output_str.contains("light") || output_str.contains("default") {
                return false; // Explicitly return false for light themes
            }
        }
        
        // Also try the gtk-theme setting which might indicate a dark theme
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout).to_lowercase();
            if output_str.contains("dark") {
                return true;
            } else if output_str.contains("light") || output_str.contains("yaru") || output_str.contains("adwaita") {
                return false; // Explicitly return false for known light themes
            }
        }
    } else if desktop_env.contains("KDE") {
        // Try kreadconfig5 for KDE Plasma
        let output = std::process::Command::new("kreadconfig5")
            .args(["--group", "General", "--key", "ColorScheme", "--file", "kdeglobals"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Dark") || output_str.contains("dark") || output_str.contains("Breeze Dark") {
                return true;
            }
        }
    }
    
    // NOTE: GTK settings check disabled because it can lag behind system theme changes
    // and override correct GSettings detection. We rely on GSettings which is more reliable.
    // 
    // Check GTK settings as a fallback (may not always be accurate with Ubuntu theme switching)
    // if let Some(settings) = Settings::default() {
    //     let gtk_setting = settings.is_gtk_application_prefer_dark_theme();
    //     if gtk_setting {
    //         return true;
    //     }
    //     
    //     // Also check the theme name itself
    //     let theme_name = settings.gtk_theme_name();
    //     if let Some(theme) = theme_name {
    //         if theme.to_lowercase().contains("dark") {
    //             return true;
    //         }
    //     }
    // }
    
    // Check for any other common dark theme indicators
    let typical_dark_themes = [
        "Adwaita-dark", "Breeze-Dark", "Arc-Dark", "Yaru-dark", 
        "Materia-dark", "Pop-dark", "Nordic", "Dracula"
    ];
    
    if let Ok(current_theme) = std::env::var("GTK_THEME") {
        if typical_dark_themes.iter().any(|&theme| current_theme.contains(theme)) {
            return true;
        }
    }
    
    // Default to light theme if all detection methods fail
    false
}

// Track if we're currently getting the preferred style scheme to avoid recursive calls
static mut GETTING_STYLE: bool = false;

/// Gets the appropriate style scheme name based on user preferences
/// 
/// Returns user-configured theme for light or dark mode, without fallback logic
pub fn get_preferred_style_scheme() -> String {
    // Prevent recursive calls when refresh_settings calls back into this function
    unsafe {
        if GETTING_STYLE {
            return if is_dark_mode_enabled() {
                "solarized-dark".to_string()
            } else {
                "solarized-light".to_string()
            };
        }
        GETTING_STYLE = true;
    }
    
    // Get a fresh copy of settings
    let settings = crate::settings::get_settings();
    
    // Return the user's configured theme based on dark/light mode without fallbacks
    let theme = if is_dark_mode_enabled() {
        let theme = settings.get_dark_theme();
        println!("Using dark theme: {}", theme);
        theme
    } else {
        let theme = settings.get_light_theme();
        println!("Using light theme: {}", theme);
        theme
    };
    
    // Reset the flag
    unsafe {
        GETTING_STYLE = false;
    }
    
    theme
}

/// Creates a sourceview with syntax highlighting instead of a regular TextView
/// 
/// This function replaces the standard TextView with SourceView from the sourceview5 library,
/// which provides syntax highlighting capabilities based on file extensions.
pub fn create_source_view() -> (View, Buffer) {
    // Create the buffer first with syntax highlighting
    let buffer = Buffer::new(None);
    
    // Set up syntax highlighting with a style scheme based on user preferences
    let scheme_manager = StyleSchemeManager::new();
    let preferred_scheme = get_preferred_style_scheme();
    
    println!("Creating new source view with theme: {}", preferred_scheme);
    
    // Apply the user's preferred theme directly
    if let Some(scheme) = scheme_manager.scheme(&preferred_scheme) {
        println!("Successfully applied theme '{}' to new buffer", preferred_scheme);
        buffer.set_style_scheme(Some(&scheme));
    } else {
        println!("WARNING: Failed to find theme '{}' for new buffer", preferred_scheme);
    }

    // Create the view with the buffer
    let source_view = View::with_buffer(&buffer);
    
    // Configure standard options for the source view
    source_view.set_monospace(true);
    source_view.set_editable(true);
    source_view.set_cursor_visible(true);
    source_view.set_show_line_numbers(true);
    source_view.set_highlight_current_line(true);
    source_view.set_tab_width(4);
    source_view.set_auto_indent(true);
    
    (source_view, buffer)
}

/// Updates the style scheme of an existing buffer based on user theme preference
/// 
/// This function can be called when the system theme changes to update
/// the syntax highlighting style scheme accordingly
pub fn update_buffer_style_scheme(buffer: &Buffer) {
    // Force refresh of settings to pick up any theme changes
    crate::settings::refresh_settings();
    
    let scheme_manager = StyleSchemeManager::new();
    let preferred_scheme = get_preferred_style_scheme();
    
    println!("Updating buffer style scheme to: {}", preferred_scheme);
    
    // Simply apply the user's preferred theme without fallbacks
    if let Some(scheme) = scheme_manager.scheme(&preferred_scheme) {
        println!("Successfully found and applied theme: {}", preferred_scheme);
        buffer.set_style_scheme(Some(&scheme));
    } else {
        println!("WARNING: Theme '{}' not found in available schemes!", preferred_scheme);
        
        // List available schemes for debugging
        let available_schemes: Vec<String> = scheme_manager.scheme_ids()
            .iter()
            .map(|s| s.to_string())
            .collect();
        println!("Available schemes: {:?}", available_schemes);
    }

    // Note: We don't emit the "changed" signal here as it would mark clean files as dirty.
    // The set_style_scheme() call above is sufficient to update the visual appearance.
}

/// Sets the language for syntax highlighting based on file extension
///
/// This function identifies the programming language from a file's extension
/// and applies appropriate syntax highlighting to the buffer.
pub fn set_language_for_file(buffer: &Buffer, file_path: &Path) -> bool {
    let language_manager = LanguageManager::new();
    
    // Get the file extension
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    // Try to get language from file path directly
    if let Some(language) = language_manager.guess_language(
        Some(file_path.to_str().unwrap_or("")), 
        None
    ) {
        buffer.set_language(Some(&language));
        return true;
    } 
    
    // If that fails, try to map the extension to a language ourselves
    let language_id = match extension.to_lowercase().as_str() {
        "rs" => "rust",
        "py" => "python",
        "js" => "javascript",
        "ts" => "typescript",
        "html" => "html",
        "css" => "css",
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "h" | "hpp" | "hxx" => "cpp",
        "java" => "java",
        "sh" => "sh",
        "rb" => "ruby",
        "php" => "php",
        "xml" => "xml",
        "json" => "json",
        "md" => "markdown",
        "txt" => "text",
        "go" => "go",
        "swift" => "swift",
        "sql" => "sql",
        "yaml" | "yml" => "yaml",
        "toml" => "toml",
        "dart" => "dart",
        "kt" | "kts" => "kotlin",
        _ => "",
    };
    
    if !language_id.is_empty() {
        if let Some(language) = language_manager.language(language_id) {
            buffer.set_language(Some(&language));
            return true;
        }
    }
    
    // If no language was set, default to plain text (no highlighting)
    buffer.set_language(None);
    false
}

/// Wraps a SourceView in a ScrolledWindow
///
/// This function creates a scrollable container for the sourceview,
/// similar to how the regular TextView is wrapped.
pub fn create_source_view_scrolled(source_view: &View) -> ScrolledWindow {
    gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(source_view)
        .build()
}

/// Debug function to print current theme detection status
/// Useful for troubleshooting theme switching issues
pub fn debug_theme_detection() {
    println!("=== Theme Detection Debug ===");
    
    // Check GTK settings
    if let Some(settings) = Settings::default() {
        let gtk_setting = settings.is_gtk_application_prefer_dark_theme();
        println!("GTK dark theme preference: {}", gtk_setting);
        
        let theme_name = settings.gtk_theme_name();
        println!("Current GTK theme name: {:?}", theme_name);
        
        let icon_theme = settings.gtk_icon_theme_name();
        println!("Current icon theme: {:?}", icon_theme);
    } else {
        println!("GTK settings not available");
    }
    
    // Check GIO settings for GNOME
    use gtk4::gio::prelude::*;
    match std::panic::catch_unwind(|| gtk4::gio::Settings::new("org.gnome.desktop.interface")) {
        Ok(gio_settings) => {
            let color_scheme = gio_settings.string("color-scheme");
            println!("GNOME color-scheme: {}", color_scheme);
            
            let gtk_theme = gio_settings.string("gtk-theme");
            println!("GNOME gtk-theme: {}", gtk_theme);
        },
        Err(_) => {
            println!("GNOME desktop interface settings not available");
        }
    }
    
    // Check environment variables
    if let Ok(desktop_env) = std::env::var("XDG_CURRENT_DESKTOP") {
        println!("Desktop environment: {}", desktop_env);
    }
    
    if let Ok(gtk_theme) = std::env::var("GTK_THEME") {
        println!("GTK_THEME environment variable: {}", gtk_theme);
    }
    
    // Final detection result
    let is_dark = is_dark_mode_enabled();
    println!("Final dark mode detection: {}", is_dark);
    println!("=============================");
}

/// Forces GTK settings to sync with the detected system theme
/// This helps ensure GTK applications properly reflect system theme changes
pub fn sync_gtk_with_system_theme() {
    if let Some(settings) = Settings::default() {
        let detected_dark_mode = is_dark_mode_enabled();
        let current_gtk_setting = settings.is_gtk_application_prefer_dark_theme();
        
        if detected_dark_mode != current_gtk_setting {
            println!("Syncing GTK setting: detected={}, current={}", detected_dark_mode, current_gtk_setting);
            settings.set_gtk_application_prefer_dark_theme(detected_dark_mode);
            
            // Force refresh of user settings to pick up theme change
            crate::settings::refresh_settings();
        }
    }
}
