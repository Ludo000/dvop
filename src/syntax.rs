// Syntax highlighting functionality for the text editor
// This module manages syntax highlighting based on file types

use sourceview5::{prelude::*, Buffer, LanguageManager, StyleSchemeManager, View};
use gtk4::prelude::*;
use gtk4::ScrolledWindow;
use gtk4::Settings;
use std::path::Path;

/// Determines whether the system is using a dark theme
/// 
/// Checks the GTK settings and environment to determine if the system prefers dark mode
pub fn is_dark_mode_enabled() -> bool {
    // Print environment variables to help diagnose dark mode detection
    println!("⭐⭐⭐ DEBUGGING DARK MODE DETECTION ⭐⭐⭐");
    
    // First check GTK settings directly
    if let Some(settings) = Settings::default() {
        let gtk_setting = settings.is_gtk_application_prefer_dark_theme();
        println!("GTK setting prefer_dark: {}", gtk_setting);
        if gtk_setting {
            return true;
        }
    }
    
    // Check environment variables that might indicate dark theme
    if let Ok(var) = std::env::var("GTK_THEME") {
        println!("GTK_THEME: {}", var);
        if var.contains("dark") {
            println!("Dark mode detected via GTK_THEME");
            return true;
        }
    } else {
        println!("GTK_THEME not set");
    }
    
    if let Ok(var) = std::env::var("GTK_THEME_PREFER_DARK") {
        println!("GTK_THEME_PREFER_DARK: {}", var);
        if var == "1" || var.to_lowercase() == "true" {
            println!("Dark mode detected via GTK_THEME_PREFER_DARK");
            return true;
        }
    } else {
        println!("GTK_THEME_PREFER_DARK not set");
    }

    if let Ok(var) = std::env::var("GTK_APPLICATION_PREFER_DARK_THEME") {
        println!("GTK_APPLICATION_PREFER_DARK_THEME: {}", var);
        if var == "1" || var.to_lowercase() == "true" {
            println!("Dark mode detected via GTK_APPLICATION_PREFER_DARK_THEME");
            return true;
        }
    } else {
        println!("GTK_APPLICATION_PREFER_DARK_THEME not set");
    }
    
    // Check for desktop environment specific settings
    let desktop_env = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    println!("XDG_CURRENT_DESKTOP: {}", desktop_env);
    
    if desktop_env.contains("GNOME") {
        println!("Detected GNOME desktop environment");
        // Try gsettings for GNOME
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("GNOME color-scheme: {}", output_str.trim());
            if output_str.contains("dark") {
                println!("Dark mode detected via GNOME color-scheme");
                return true;
            }
        } else {
            println!("Could not execute gsettings command for color-scheme");
        }
        
        // Also try the gtk-theme setting which might indicate a dark theme
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "gtk-theme"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("GNOME gtk-theme: {}", output_str.trim());
            if output_str.to_lowercase().contains("dark") {
                println!("Dark mode detected via GNOME gtk-theme");
                return true;
            }
        } else {
            println!("Could not execute gsettings command for gtk-theme");
        }
    } else if desktop_env.contains("KDE") {
        println!("Detected KDE desktop environment");
        // Try kreadconfig5 for KDE Plasma
        let output = std::process::Command::new("kreadconfig5")
            .args(["--group", "General", "--key", "ColorScheme", "--file", "kdeglobals"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            println!("KDE ColorScheme: {}", output_str.trim());
            if output_str.contains("Dark") || output_str.contains("dark") || output_str.contains("Breeze Dark") {
                println!("Dark mode detected via KDE ColorScheme");
                return true;
            }
        } else {
            println!("Could not execute kreadconfig5 command for ColorScheme");
        }
    }
    
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
    
    // For testing - If needed, you can uncomment the next two lines to force dark mode
    // println!("⭐⭐⭐ FORCING DARK MODE FOR TESTING ⭐⭐⭐");
    // return true;
    
    // Default to light theme if all detection methods fail
    false
}

/// Gets the appropriate style scheme name based on system theme
/// 
/// Returns "classic" for light theme and "cobalt" or other dark scheme for dark theme
pub fn get_preferred_style_scheme() -> &'static str {
    // Get the StyleSchemeManager to check available schemes
    let scheme_manager = StyleSchemeManager::new();
    let available_schemes: Vec<String> = scheme_manager.scheme_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    println!("Available style schemes: {:?}", available_schemes);
    
    if is_dark_mode_enabled() {
        // Try several dark schemes in order of preference
        for scheme in ["cobalt", "oblivion", "monokai", "kate-dark", "solarized-dark"] {
            if available_schemes.iter().any(|s| s == scheme) {
                println!("Selected dark scheme: {}", scheme);
                return scheme;
            }
        }
        // Fallback to any available dark scheme
        if available_schemes.iter().any(|s| s.contains("dark")) {
            let dark_scheme = available_schemes.iter()
                .find(|s| s.contains("dark"))
                .unwrap();
            println!("Selected fallback dark scheme: {}", dark_scheme);
            return "classic"; // We can't return a String reference from this function
                              // so we'll handle this special case in create_source_view
        }
        "classic" // Last fallback if no dark theme is available
    } else {
        // Try several light schemes in order of preference
        for scheme in ["classic", "kate", "tango", "solarized-light"] {
            if available_schemes.iter().any(|s| s == scheme) {
                println!("Selected light scheme: {}", scheme);
                return scheme;
            }
        }
        "classic" // Default light theme
    }
}

/// Creates a sourceview with syntax highlighting instead of a regular TextView
/// 
/// This function replaces the standard TextView with SourceView from the sourceview5 library,
/// which provides syntax highlighting capabilities based on file extensions.
pub fn create_source_view() -> (View, Buffer) {
    // Create the buffer first with syntax highlighting
    let buffer = Buffer::new(None);
    
    // Set up syntax highlighting with a style scheme based on system theme
    let scheme_manager = StyleSchemeManager::new();
    let preferred_scheme = get_preferred_style_scheme();
    
    // Special handling for the case where we found a dark scheme by searching
    let available_schemes: Vec<String> = scheme_manager.scheme_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    if let Some(scheme) = scheme_manager.scheme(preferred_scheme) {
        buffer.set_style_scheme(Some(&scheme));
        println!("Applied style scheme: {}", preferred_scheme);
    } else if is_dark_mode_enabled() && available_schemes.iter().any(|s| s.contains("dark")) {
        // Find any scheme with "dark" in the name
        let dark_scheme_name = available_schemes.iter()
            .find(|s| s.contains("dark"))
            .unwrap()
            .clone();
        
        if let Some(dark_scheme) = scheme_manager.scheme(&dark_scheme_name) {
            buffer.set_style_scheme(Some(&dark_scheme));
            println!("Applied dark style scheme: {}", dark_scheme_name);
        }
    } else if let Some(fallback_scheme) = scheme_manager.scheme("classic") {
        // Fallback to classic scheme if the preferred one isn't available
        buffer.set_style_scheme(Some(&fallback_scheme));
        println!("Applied fallback style scheme: classic");
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

/// Updates the style scheme of an existing buffer based on system theme
/// 
/// This function can be called when the system theme changes to update
/// the syntax highlighting style scheme accordingly
pub fn update_buffer_style_scheme(buffer: &Buffer) {
    let scheme_manager = StyleSchemeManager::new();
    let preferred_scheme = get_preferred_style_scheme();
    
    // Special handling for the case where we found a dark scheme by searching
    let available_schemes: Vec<String> = scheme_manager.scheme_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    println!("Updating buffer style to match current theme: {}", 
             if is_dark_mode_enabled() { "dark" } else { "light" });
    
    // Try to apply the preferred scheme
    if let Some(scheme) = scheme_manager.scheme(preferred_scheme) {
        buffer.set_style_scheme(Some(&scheme));
        println!("Applied style scheme to existing buffer: {}", preferred_scheme);
    } else if is_dark_mode_enabled() && available_schemes.iter().any(|s| s.contains("dark")) {
        // Find any scheme with "dark" in the name
        let dark_scheme_name = available_schemes.iter()
            .find(|s| s.contains("dark"))
            .unwrap()
            .clone();
        
        if let Some(dark_scheme) = scheme_manager.scheme(&dark_scheme_name) {
            buffer.set_style_scheme(Some(&dark_scheme));
            println!("Applied dark style scheme to existing buffer: {}", dark_scheme_name);
        }
    } else if let Some(fallback_scheme) = scheme_manager.scheme("classic") {
        // Fallback to classic scheme if the preferred one isn't available
        buffer.set_style_scheme(Some(&fallback_scheme));
        println!("Applied fallback style scheme to existing buffer: classic");
    }
    
    // Force the buffer to redraw with the "changed" signal 
    // This ensures the theme change is immediately visible in existing views
    buffer.emit_by_name::<()>("changed", &[]);
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

/// Toggle dark mode on or off
/// 
/// This function toggles the GTK dark mode preference and returns the new state
pub fn toggle_dark_mode() -> bool {
    if let Some(settings) = gtk4::Settings::default() {
        let current = settings.is_gtk_application_prefer_dark_theme();
        let new_value = !current;
        settings.set_gtk_application_prefer_dark_theme(new_value);
        new_value
    } else {
        false
    }
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
