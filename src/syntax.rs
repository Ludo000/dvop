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
    
    // First check GTK settings directly
    if let Some(settings) = Settings::default() {
        let gtk_setting = settings.is_gtk_application_prefer_dark_theme();
        if gtk_setting {
            return true;
        }
    }
    
    // Check for desktop environment specific settings
    let desktop_env = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();


    if desktop_env.contains("GNOME") || desktop_env.contains("Unity") {
        // Try gsettings for GNOME
        let output = std::process::Command::new("gsettings")
            .args(["get", "org.gnome.desktop.interface", "color-scheme"])
            .output()
            .ok();
            
        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("dark") {
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
            if output_str.to_lowercase().contains("dark") {
                return true;
            }
        } else {
            println!("Could not execute gsettings command for gtk-theme");
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
    
    if is_dark_mode_enabled() {
        // Try several dark schemes in order of preference
        for scheme in ["Yaru-dark", "Adwaita-dark", "kate-dark", "oblivion", "cobalt", "monokai", "solarized-dark"] {
            if available_schemes.iter().any(|s| s == scheme) {
                return scheme;
            }
        }
        "classic" // Last fallback if no dark theme is available
    } else {
        // Try several light schemes in order of preference
        for scheme in ["Yaru", "Adwaita", "kate", "classic", "tango", "solarized-light"] {
            if available_schemes.iter().any(|s| s == scheme) {
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
    } else if is_dark_mode_enabled() && available_schemes.iter().any(|s| s.contains("dark")) {
        // Find any scheme with "dark" in the name
        let dark_scheme_name = available_schemes.iter()
            .find(|s| s.contains("dark"))
            .unwrap()
            .clone();
        
        if let Some(dark_scheme) = scheme_manager.scheme(&dark_scheme_name) {
            buffer.set_style_scheme(Some(&dark_scheme));
        }
    } else if let Some(fallback_scheme) = scheme_manager.scheme("classic") {
        // Fallback to classic scheme if the preferred one isn't available
        buffer.set_style_scheme(Some(&fallback_scheme));
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
    
    // Try to apply the preferred scheme
    if let Some(scheme) = scheme_manager.scheme(preferred_scheme) {
        buffer.set_style_scheme(Some(&scheme));
    } else if is_dark_mode_enabled() && available_schemes.iter().any(|s| s.contains("dark")) {
        // Find any scheme with "dark" in the name
        let dark_scheme_name = available_schemes.iter()
            .find(|s| s.contains("dark"))
            .unwrap()
            .clone();
        
        if let Some(dark_scheme) = scheme_manager.scheme(&dark_scheme_name) {
            buffer.set_style_scheme(Some(&dark_scheme));
        }
    } else if let Some(fallback_scheme) = scheme_manager.scheme("classic") {
        // Fallback to classic scheme if the preferred one isn't available
        buffer.set_style_scheme(Some(&fallback_scheme));
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
