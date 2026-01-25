// Syntax highlighting functionality for the text editor
// This module manages syntax highlighting based on file types

use gtk4::gdk::RGBA;
use gtk4::ScrolledWindow;
use gtk4::Settings;
use sourceview5::{prelude::*, Buffer, LanguageManager, MarkAttributes, StyleSchemeManager, View};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;

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
                // First check if the key exists to avoid crashes on older GNOME versions
                if let Some(schema) = gio_settings.settings_schema() {
                    if schema.has_key("color-scheme") {
                        let color_scheme = gio_settings.string("color-scheme");
                        if color_scheme.as_str() == "prefer-dark" {
                            return true;
                        }
                        if color_scheme.as_str() == "prefer-light"
                            || color_scheme.as_str() == "default"
                        {
                            return false; // Explicitly return false for light themes - this is definitive
                        }
                    }
                }

                // Check the gtk-theme setting as fallback
                let gtk_theme = gio_settings.string("gtk-theme");
                let theme_lower = gtk_theme.to_lowercase();
                if theme_lower.contains("dark") {
                    return true;
                } else if theme_lower == "yaru"
                    || theme_lower == "adwaita"
                    || theme_lower.contains("light")
                {
                    return false; // Explicitly return false for known light themes
                }
            }
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
            } else if output_str.contains("light")
                || output_str.contains("yaru")
                || output_str.contains("adwaita")
            {
                return false; // Explicitly return false for known light themes
            }
        }
    } else if desktop_env.contains("KDE") {
        // Try kreadconfig5 for KDE Plasma
        let output = std::process::Command::new("kreadconfig5")
            .args([
                "--group",
                "General",
                "--key",
                "ColorScheme",
                "--file",
                "kdeglobals",
            ])
            .output()
            .ok();

        if let Some(output) = output {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if output_str.contains("Dark")
                || output_str.contains("dark")
                || output_str.contains("Breeze Dark")
            {
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
        "Adwaita-dark",
        "Breeze-Dark",
        "Arc-Dark",
        "Yaru-dark",
        "Materia-dark",
        "Pop-dark",
        "Nordic",
        "Dracula",
    ];

    if let Ok(current_theme) = std::env::var("GTK_THEME") {
        if typical_dark_themes
            .iter()
            .any(|&theme| current_theme.contains(theme))
        {
            return true;
        }
    }

    // Default to light theme if all detection methods fail
    false
}

use std::cell::Cell;

// Track if we're currently getting the preferred style scheme to avoid recursive calls
thread_local! {
    static GETTING_STYLE: Cell<bool> = const { Cell::new(false) };
}

/// Gets the appropriate style scheme name based on user preferences
///
/// Returns user-configured theme for light or dark mode, without fallback logic
pub fn get_preferred_style_scheme() -> String {
    // Prevent recursive calls when refresh_settings calls back into this function
    if GETTING_STYLE.with(|flag| flag.get()) {
        // Use generic fallback themes that are always available
        return if is_dark_mode_enabled() {
            "classic-dark".to_string()
        } else {
            "classic".to_string()
        };
    }

    GETTING_STYLE.with(|flag| flag.set(true));

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
    GETTING_STYLE.with(|flag| flag.set(false));

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
        println!(
            "Successfully applied theme '{}' to new buffer",
            preferred_scheme
        );
        buffer.set_style_scheme(Some(&scheme));
    } else {
        println!(
            "WARNING: Failed to find theme '{}' for new buffer",
            preferred_scheme
        );
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
    
    // Enable line marks in the gutter (for breakpoints)
    source_view.set_show_line_marks(true);

    // Apply user's font size setting
    let settings = crate::settings::get_settings();
    let font_size = settings.get_font_size();
    apply_font_size_to_view(&source_view, font_size);

    // Enable code completion
    crate::completion::setup_completion(&source_view);

    // Setup keyboard shortcuts for completion (Ctrl+Space)
    crate::completion::setup_completion_shortcuts(&source_view);

    // Note: setup_linting is called later when the file path is known
    // See handlers.rs open_or_focus_tab() which calls setup_linting with the actual file path

    (source_view, buffer)
}

/// The category name used for breakpoint marks in the gutter
pub const BREAKPOINT_CATEGORY: &str = "breakpoint";

/// Thread-local callback for refreshing the debugger panel's breakpoint list
/// This is set by the main application and called when breakpoints change from the editor
thread_local! {
    static BREAKPOINT_CHANGE_CALLBACK: RefCell<Option<Box<dyn Fn()>>> = const { RefCell::new(None) };
}

/// Sets the callback function that will be called when breakpoints are added or removed
/// from the editor gutter. This should be called once during application setup.
pub fn set_breakpoint_change_callback<F: Fn() + 'static>(callback: F) {
    BREAKPOINT_CHANGE_CALLBACK.with(|cb| {
        *cb.borrow_mut() = Some(Box::new(callback));
    });
}

/// Calls the breakpoint change callback if one is registered
fn notify_breakpoint_change() {
    BREAKPOINT_CHANGE_CALLBACK.with(|cb| {
        if let Some(ref callback) = *cb.borrow() {
            callback();
        }
    });
}

/// Sets up breakpoint support for a source view
/// 
/// This configures the mark attributes for breakpoints (red circle icon)
/// and connects the line_mark_activated signal to toggle breakpoints when clicking on the gutter.
/// 
/// # Arguments
/// * `source_view` - The source view to set up breakpoint support for
/// * `file_path` - The file path associated with this view (used for debugger integration)
/// * `_debugger` - Deprecated parameter, kept for API compatibility. The global debugger is used instead.
pub fn setup_breakpoint_support(
    source_view: &View,
    file_path: Rc<RefCell<Option<PathBuf>>>,
    _debugger: Option<Arc<crate::debugger::RustDebugger>>,
) {
    // Create mark attributes for breakpoints - red circle
    let breakpoint_attrs = MarkAttributes::new();
    
    // Set a red background color for the breakpoint indicator
    let red = RGBA::new(0.9, 0.2, 0.2, 1.0);
    breakpoint_attrs.set_background(&red);
    
    // Set the icon name to use the standard breakpoint icon
    breakpoint_attrs.set_icon_name("media-record"); // Red circle icon
    
    // Register the mark attributes with the source view
    // Priority 1 means breakpoints show above other marks
    source_view.set_mark_attributes(BREAKPOINT_CATEGORY, &breakpoint_attrs, 1);
    
    // Get the buffer to work with marks
    let buffer = source_view
        .buffer()
        .downcast::<Buffer>()
        .expect("SourceView should have a SourceBuffer");
    
    // Clone what we need for the closure
    let buffer_for_signal = buffer.clone();
    let file_path_for_signal = file_path.clone();
    
    // The line_mark_activated signal fires when clicking in the mark gutter area
    // This works once there are marks, and also fires on the line number gutter
    source_view.connect_line_mark_activated(move |_view, iter, button, _modifiers, _n_presses| {
        // Only respond to primary (left) mouse button
        if button != 1 {
            return;
        }
        
        let line = iter.line() as u32 + 1; // Convert to 1-based line number
        
        // Get the file path
        let file_path_opt = file_path_for_signal.borrow().clone();
        let file = match file_path_opt {
            Some(path) => path,
            None => {
                // Can't set breakpoint on unsaved file
                return;
            }
        };
        
        // Get the global debugger instance
        let debugger = crate::debugger::ui::get_debugger_instance();
        
        // Check if there's already a breakpoint on this line
        let has_breakpoint = has_breakpoint_at_line(&buffer_for_signal, line);
        
        if has_breakpoint {
            // Remove the breakpoint
            println!("[Breakpoint] Removing breakpoint at {}:{}", file.display(), line);
            remove_breakpoint_mark(&buffer_for_signal, line);
            
            // Notify debugger if available - use remove_breakpoint_at for direct file:line removal
            if let Some(ref dbg) = debugger {
                let _ = dbg.remove_breakpoint_at(&file, line);
            }
            
            // Notify the debugger panel to refresh its list
            notify_breakpoint_change();
        } else {
            // Add a new breakpoint
            println!("[Breakpoint] Adding breakpoint at {}:{}", file.display(), line);
            add_breakpoint_mark(&buffer_for_signal, line);
            
            // Notify debugger if available
            if let Some(ref dbg) = debugger {
                let _ = dbg.add_breakpoint(file.clone(), line);
            }
            
            // Notify the debugger panel to refresh its list
            notify_breakpoint_change();
        }
    });
}

/// Checks if there's a breakpoint at the given line (1-based)
fn has_breakpoint_at_line(buffer: &Buffer, line: u32) -> bool {
    let line_index = (line - 1) as i32;
    
    // Get marks at this line
    let marks = buffer.source_marks_at_line(line_index, Some(BREAKPOINT_CATEGORY));
    !marks.is_empty()
}

/// Adds a breakpoint mark at the given line (1-based)
fn add_breakpoint_mark(buffer: &Buffer, line: u32) {
    let line_index = (line - 1) as i32;
    if let Some(iter) = buffer.iter_at_line(line_index) {
        // Create a unique name for this mark
        let mark_name = format!("breakpoint-{}", line);
        buffer.create_source_mark(Some(&mark_name), BREAKPOINT_CATEGORY, &iter);
    }
}

/// Removes a breakpoint mark at the given line (1-based)
fn remove_breakpoint_mark(buffer: &Buffer, line: u32) {
    let line_index = (line - 1) as i32;
    let marks = buffer.source_marks_at_line(line_index, Some(BREAKPOINT_CATEGORY));
    
    for mark in marks {
        buffer.delete_mark(&mark);
    }
}

/// Adds a breakpoint visually at the specified line without notifying the debugger
/// Used for syncing breakpoints from the debugger to the UI
#[allow(dead_code)]
pub fn add_breakpoint_visual(buffer: &Buffer, line: u32) {
    if !has_breakpoint_at_line(buffer, line) {
        add_breakpoint_mark(buffer, line);
    }
}

/// Removes a breakpoint visually at the specified line without notifying the debugger
/// Used for syncing breakpoints from the debugger to the UI
#[allow(dead_code)]
pub fn remove_breakpoint_visual(buffer: &Buffer, line: u32) {
    remove_breakpoint_mark(buffer, line);
}

/// Clears all breakpoint marks from the buffer
#[allow(dead_code)]
pub fn clear_all_breakpoints(buffer: &Buffer) {
    // Get all breakpoint marks
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    
    // Remove all marks with the breakpoint category
    buffer.remove_source_marks(&start, &end, Some(BREAKPOINT_CATEGORY));
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
        println!(
            "WARNING: Theme '{}' not found in available schemes!",
            preferred_scheme
        );

        // List available schemes for debugging
        let available_schemes: Vec<String> = scheme_manager
            .scheme_ids()
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
    let extension = file_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    // Try to get language from file path directly
    if let Some(language) =
        language_manager.guess_language(Some(file_path.to_str().unwrap_or("")), None)
    {
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
        "xml" | "ui" | "svg" => "xml",
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
        "svelte" => "html",
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
            // Check if keys exist before accessing them
            if let Some(schema) = gio_settings.settings_schema() {
                if schema.has_key("color-scheme") {
                    let color_scheme = gio_settings.string("color-scheme");
                    println!("GNOME color-scheme: {}", color_scheme);
                } else {
                    println!(
                        "GNOME color-scheme key not available (requires GNOME 42+/Ubuntu 22.04+)"
                    );
                }

                if schema.has_key("gtk-theme") {
                    let gtk_theme = gio_settings.string("gtk-theme");
                    println!("GNOME gtk-theme: {}", gtk_theme);
                }
            }
        }
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
            println!(
                "Syncing GTK setting: detected={}, current={}",
                detected_dark_mode, current_gtk_setting
            );
            settings.set_gtk_application_prefer_dark_theme(detected_dark_mode);

            // Force refresh of user settings to pick up theme change
            crate::settings::refresh_settings();
        }
    }
}

/// Increases the font size for all text editors
pub fn increase_font_size() {
    let mut settings = crate::settings::get_settings_mut();
    let current_size = settings.get_font_size();
    let new_size = (current_size + 1).min(72); // Cap at 72pt
    settings.set_font_size(new_size);
    let _ = settings.save();

    // Drop the mutex guard before calling refresh
    drop(settings);
    crate::settings::refresh_settings();

    // Apply the new font size to all editor views
    apply_font_size_globally(new_size);

    crate::status_log::log_info(&format!("Font size increased to {}", new_size));
}

/// Decreases the font size for all text editors  
pub fn decrease_font_size() {
    let mut settings = crate::settings::get_settings_mut();
    let current_size = settings.get_font_size();
    let new_size = (current_size.saturating_sub(1)).max(6); // Minimum 6pt
    settings.set_font_size(new_size);
    let _ = settings.save();

    // Drop the mutex guard before calling refresh
    drop(settings);
    crate::settings::refresh_settings();

    // Apply the new font size to all editor views
    apply_font_size_globally(new_size);

    crate::status_log::log_info(&format!("Font size decreased to {}", new_size));
}

/// Resets the font size to the default value
pub fn reset_font_size() {
    let mut settings = crate::settings::get_settings_mut();
    let default_size = crate::settings::DEFAULT_FONT_SIZE;
    settings.set_font_size(default_size);
    let _ = settings.save();

    // Drop the mutex guard before calling refresh
    drop(settings);
    crate::settings::refresh_settings();

    // Apply the new font size to all editor views
    apply_font_size_globally(default_size);

    crate::status_log::log_info(&format!("Font size reset to default ({})", default_size));
}

/// Applies the font size to all SourceView widgets in the application
fn apply_font_size_to_all_views(font_size: u32) {
    let css = format!(
        "textview {{ font-size: {}pt; }} 
         sourceview {{ font-size: {}pt; }}",
        font_size, font_size
    );

    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&css);

    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_USER, // Higher priority than application styles
    );
}

/// Applies font size globally by finding all SourceView widgets in the application
pub fn apply_font_size_globally(font_size: u32) {
    // First apply the global CSS for new views
    apply_font_size_to_all_views(font_size);

    // Then find all existing windows and update their SourceViews
    let app = gtk4::gio::Application::default();
    if let Some(app) = app {
        if let Ok(gtk_app) = app.downcast::<gtk4::Application>() {
            for window in gtk_app.windows() {
                if let Ok(app_window) = window.downcast::<gtk4::ApplicationWindow>() {
                    update_font_size_in_window(&app_window, font_size);
                }
            }
        }
    }
}

/// Updates font size for all SourceView widgets within a window
fn update_font_size_in_window(window: &gtk4::ApplicationWindow, font_size: u32) {
    // Recursively find all SourceView widgets in the window
    find_and_update_source_views(window.upcast_ref::<gtk4::Widget>(), font_size);
}

/// Recursively searches for SourceView widgets and updates their font size
fn find_and_update_source_views(widget: &gtk4::Widget, font_size: u32) {
    // Check if this widget is a SourceView
    if let Ok(source_view) = widget.clone().downcast::<sourceview5::View>() {
        apply_font_size_to_view(&source_view, font_size);
    }

    // Recursively search children
    let mut child = widget.first_child();
    while let Some(current_child) = child {
        find_and_update_source_views(&current_child, font_size);
        child = current_child.next_sibling();
    }
}

/// Applies font size to a specific SourceView widget
fn apply_font_size_to_view(source_view: &View, font_size: u32) {
    let css_class = format!("font-size-{}", font_size);
    let css = format!(".{} {{ font-size: {}pt; }}", css_class, font_size);

    let provider = gtk4::CssProvider::new();
    provider.load_from_data(&css);

    // Add the CSS provider to the view's style context
    let style_context = source_view.style_context();

    // Remove old font size classes to avoid accumulation
    for size in 6..=72 {
        let old_class = format!("font-size-{}", size);
        style_context.remove_class(&old_class);
    }

    style_context.add_provider(&provider, gtk4::STYLE_PROVIDER_PRIORITY_USER);
    style_context.add_class(&css_class);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_preferred_style_scheme() {
        let scheme = get_preferred_style_scheme();
        // Should return a valid scheme name
        assert!(!scheme.is_empty());
    }
}
