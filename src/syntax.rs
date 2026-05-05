//! # Syntax Highlighting and Theme Detection
//!
//! This module manages syntax highlighting for the code editor using **GtkSourceView5**,
//! a GTK widget specifically designed for source code editing. It provides:
//!
//! - **Language detection**: Automatically detects the programming language from file extensions
//!   and applies appropriate syntax highlighting rules (keywords, strings, comments, etc.)
//! - **Dark mode detection**: A multi-layered detection chain that tries:
//!   1. GNOME GSettings `color-scheme` (Ubuntu 22.04+ / GNOME 42+)
//!   2. GNOME GSettings `gtk-theme` name (checks for "dark" in theme name)
//!   3. KDE `kreadconfig5` (`ColorScheme` setting)
//!   4. GTK Settings `gtk-application-prefer-dark-theme` property
//!   5. Environment variable fallbacks
//! - **Style scheme selection**: Maps dark/light mode to SourceView color schemes
//!   (e.g. "Adwaita-dark", "classic-dark" for dark mode)
//! - **Large file optimization**: Files over 10 MB (`LARGE_FILE_THRESHOLD`) get reduced
//!   features to keep the editor responsive
//!
//! ## Key Concepts for Rust Beginners
//!
//! - **`sourceview5::Buffer`** vs **`gtk4::TextBuffer`**: SourceView extends GTK's basic
//!   text buffer with syntax highlighting, undo/redo, and line numbers. We often need to
//!   "downcast" (convert) a generic `TextBuffer` to a `sourceview5::Buffer` using
//!   `.dynamic_cast_ref::<sourceview5::Buffer>()` or `.downcast::<sourceview5::Buffer>()`.
//!
//! - **Thread-local storage** (`thread_local!`): Used to prevent infinite recursion when
//!   theme detection triggers GTK settings changes, which would trigger theme detection again.
//!
//! See FEATURES.md: Feature #2 — Syntax Highlighting (15+ Languages)
//! See FEATURES.md: Feature #3 — Line Numbers Display
//! See FEATURES.md: Feature #14 — Auto-Indent and Tab Support
//! See FEATURES.md: Feature #119 — Theme System
//! See FEATURES.md: Feature #120 — Dark Mode Detection

// Syntax highlighting functionality for the text editor
// This module manages syntax highlighting based on file types

use gtk4::ScrolledWindow;
use gtk4::Settings;
use sourceview5::{prelude::*, Buffer, LanguageManager, StyleSchemeManager, View};
use std::path::Path;

/// Files larger than this threshold (in bytes) are considered "large" and will
/// have expensive features (syntax highlighting, completion, linting) disabled
/// to keep the editor responsive.
pub const LARGE_FILE_THRESHOLD: usize = 10_485_760; // 10 MB

/// Determines whether the system is using a dark color scheme.
///
/// The detection follows a priority chain (most reliable first):
/// 1. **GNOME/Unity** — read `org.gnome.desktop.interface` GIO setting `color-scheme`
/// 2. **KDE Plasma** — parse `~/.config/kdeglobals` for `ColorScheme` containing "dark"
/// 3. **GTK fallback** — check `gtk-application-prefer-dark-theme` from GTK settings
///
/// Uses `std::panic::catch_unwind` around GIO access because `Settings::new()` panics
/// if the schema isn't installed (e.g., running outside GNOME).
///
/// See FEATURES.md: Feature #119 — Smart Dark Mode Detection

// Linux doesn't have one "Dark Mode" variable. Every desktop (GNOME, KDE, etc.) 
// does it differently. This function is a massive "if-else" detective that 
// tries every known method until it finds an answer.
pub fn is_dark_mode_enabled() -> bool {
    // Check for desktop environment specific settings FIRST (more reliable than GTK settings)
    let desktop_env = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

    if desktop_env.contains("GNOME") || desktop_env.contains("Unity") {
        // Try GIO Settings first (more reliable than gsettings command)
        use gtk4::gio::prelude::*;
        // GIO Settings new() can panic if schema doesn't exist, so we need to wrap it carefully
        // catch_unwind is like a try-catch block for code that might crash (panic).
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
        
        // std::process::Command literally runs a command in your terminal 
        // and reads the text output back into Rust.
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

/// Returns the GtkSourceView5 style-scheme name matching the current light/dark mode.
///
/// Reads the user's configured themes from `EditorSettings` (keys: `light_theme`,
/// `dark_theme`) and picks the one corresponding to `is_dark_mode_enabled()`.
///
/// **Recursion guard:** This function can be called re-entrantly when
/// `refresh_settings()` triggers a re-read. The `thread_local! GETTING_STYLE`
/// flag breaks the cycle by returning a safe fallback (`classic`/`classic-dark`).
///
/// See FEATURES.md: Feature #120 — Automatic Theme Switching
pub fn get_preferred_style_scheme() -> String {
    // Prevent recursive calls when refresh_settings calls back into this function
    
    // Recursion is when a function calls itself. If we don't have this check, 
    // the app might get stuck in an infinite loop and crash.
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

/// Creates a new `sourceview5::View` + `Buffer` pair with syntax highlighting.
///
/// Returns a tuple `(View, Buffer)`. The `View` is a drop-in replacement for
/// `gtk4::TextView` that adds line numbers, syntax coloring, bracket matching,
/// and other editor features from the GtkSourceView5 library.
///
/// The buffer is immediately styled with the user's preferred color scheme
/// (see `get_preferred_style_scheme()`). The view is configured with:
/// - line numbers, tab width = 4, auto-indent
/// - monospace font via CSS
/// - bracket matching
///
/// See FEATURES.md: Feature #2 — Syntax Highlighting
/// See FEATURES.md: Feature #14 — Line Numbers
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
    
    // This is where we turn on all the "Code Editor" features like line numbers.
    source_view.set_monospace(true);
    source_view.set_editable(true);
    source_view.set_cursor_visible(true);
    source_view.set_show_line_numbers(true);
    source_view.set_highlight_current_line(true);
    source_view.set_tab_width(4);
    source_view.set_auto_indent(true);

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

/// Creates a lightweight source view optimised for large files.
///
/// Compared to `create_source_view` this skips completion setup and disables
/// highlight-current-line to reduce per-frame work.  The caller is expected to
/// also skip syntax highlighting, linting and completion registration.
pub fn create_source_view_for_large_file() -> (View, Buffer) {
    let buffer = Buffer::new(None);

    // Apply colour scheme (cheap, and keeps the editor looking consistent)
    let scheme_manager = StyleSchemeManager::new();
    let preferred_scheme = get_preferred_style_scheme();
    if let Some(scheme) = scheme_manager.scheme(&preferred_scheme) {
        buffer.set_style_scheme(Some(&scheme));
    }

    let source_view = View::with_buffer(&buffer);

    source_view.set_monospace(true);
    source_view.set_editable(true);
    source_view.set_cursor_visible(true);
    source_view.set_show_line_numbers(true);
    // Disable features that are expensive on large buffers
    
    // On a 10MB file, highlighting the current line or checking for auto-indent 
    // on every keystroke can make the app "stutter". We turn them off here.
    source_view.set_highlight_current_line(false);
    source_view.set_tab_width(4);
    source_view.set_auto_indent(false);

    // Apply user font size
    let settings = crate::settings::get_settings();
    let font_size = settings.get_font_size();
    apply_font_size_to_view(&source_view, font_size);

    // No completion, no linting – those are too expensive for large files

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

/// Detects the programming language from a file's extension and applies
/// the corresponding syntax highlighting grammar to the buffer.
///
/// Returns `true` if a language was successfully detected and applied.
/// Uses `LanguageManager` to look up grammars by extension (e.g., `.rs` → Rust,
/// `.py` → Python). Also handles special cases like `Makefile`, `Dockerfile`,
/// and `.env` files that lack standard extensions.
///
/// See FEATURES.md: Feature #2 — Syntax Highlighting
/// See FEATURES.md: Feature #3 — Multi-Language Support
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
    
    // This is a simple lookup table. If the file is "main.rs", we tell GTK 
    // to use the "rust" rules.
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
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
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

/// Synchronises the GTK `prefer-dark-theme` setting with the detected OS theme.
///
/// Called periodically (via the GSettings monitor in `main.rs`) to handle theme
/// changes made outside the application. If the detected dark-mode state differs
/// from GTK's current setting, this updates GTK and triggers `refresh_settings()`
/// which reloads style schemes for all open buffers.
///
/// See FEATURES.md: Feature #120 — Automatic Theme Switching
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

// In GTK, we style widgets using CSS, just like a website. 
// We generate a CSS string with the font-size and "inject" it into the app.
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