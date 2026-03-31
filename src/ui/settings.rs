//! # Settings UI — Theme Picker & Preferences Dialog
//!
//! Builds the settings dialog (`create_settings_dialog`) and provides a
//! function to apply theme changes globally (`apply_theme_changes_globally`).
//!
//! The dialog uses the `SettingsDialog` composite template and populates
//! its dropdowns with all available GtkSourceView5 style schemes. When the
//! user selects a scheme, the change is persisted via `EditorSettings` and
//! immediately applied to all open buffers and terminals.
//!
//! See FEATURES.md: Feature #128 — Settings Menu
//! See FEATURES.md: Feature #131 — Theme Selection
//! See FEATURES.md: Feature #136 — Font Size Configuration

use crate::settings;
use crate::syntax;
use crate::ui::terminal;
use gtk4::prelude::*;
use gtk4::{ApplicationWindow, Dialog, Notebook};
use sourceview5::StyleSchemeManager;

use super::settings_dialog_template::SettingsDialog;

/// Creates a settings dialog for configuring editor preferences
///
/// This function creates a dialog where the user can:
/// - Choose preferred syntax highlighting color schemes
/// - Set other editor preferences
///
/// Returns the dialog for display
pub fn create_settings_dialog(parent: &impl IsA<ApplicationWindow>) -> Dialog {
    // Create the template-based dialog
    let dialog = SettingsDialog::new(parent);

    // Get references to widgets
    let theme_info = dialog.theme_info();
    let light_theme_dropdown = dialog.light_theme_dropdown();
    let dark_theme_dropdown = dialog.dark_theme_dropdown();
    let font_size_spin = dialog.font_size_spin();
    let terminal_font_size_spin = dialog.terminal_font_size_spin();

    // Update theme info label
    let system_mode = if syntax::is_dark_mode_enabled() {
        "dark mode"
    } else {
        "light mode"
    };
    let current_system_theme_name = syntax::get_preferred_style_scheme();
    theme_info.set_text(&format!(
        "Current system theme: {} (using {})",
        current_system_theme_name, system_mode
    ));

    // Get available color schemes
    let scheme_manager = StyleSchemeManager::new();
    let available_schemes: Vec<String> = scheme_manager
        .scheme_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();

    // Debug: print available schemes
    println!("Available style schemes: {:?}", available_schemes);

    // Make sure we have the latest settings
    settings::refresh_settings();

    // Get current settings
    let settings_instance = settings::get_settings();
    let current_light_theme = settings_instance.get_light_theme();
    let current_dark_theme = settings_instance.get_dark_theme();

    println!(
        "Settings dialog using - Light theme: {}, Dark theme: {}",
        current_light_theme, current_dark_theme
    );

    // Get current theme based on system theme
    let current_system_theme = syntax::get_preferred_style_scheme();

    // Setup dropdown models and selections
    setup_theme_dropdown(
        &light_theme_dropdown,
        &available_schemes,
        if !syntax::is_dark_mode_enabled() {
            current_system_theme.clone()
        } else {
            current_light_theme
        },
    );

    setup_theme_dropdown(
        &dark_theme_dropdown,
        &available_schemes,
        if syntax::is_dark_mode_enabled() {
            current_system_theme.clone()
        } else {
            current_dark_theme
        },
    );

    // Get current font size settings
    let settings_instance = settings::get_settings();
    font_size_spin.set_value(settings_instance.get_font_size() as f64);
    terminal_font_size_spin.set_value(settings_instance.get_terminal_font_size() as f64);
    drop(settings_instance);

    // Get the parent window as a concrete ApplicationWindow for callbacks
    let parent_window = parent.clone().upcast::<ApplicationWindow>();

    // Connect light theme dropdown change handler
    let available_schemes_light = available_schemes.clone();
    let parent_window_light = parent_window.clone();
    light_theme_dropdown.connect_selected_notify(move |dropdown| {
        let position = dropdown.selected() as usize;
        if position < available_schemes_light.len() {
            let theme = &available_schemes_light[position];
            println!("Light theme changed to: {}", theme);

            let mut settings = settings::get_settings_mut();
            settings.set_light_theme(theme);
            if let Err(e) = settings.save() {
                eprintln!("Failed to save settings: {}", e);
            }
            drop(settings);

            settings::refresh_settings();

            apply_theme_changes_globally(&parent_window_light);
        }
    });

    // Connect dark theme dropdown change handler
    let available_schemes_dark = available_schemes.clone();
    let parent_window_dark = parent_window.clone();
    dark_theme_dropdown.connect_selected_notify(move |dropdown| {
        let position = dropdown.selected() as usize;
        if position < available_schemes_dark.len() {
            let theme = &available_schemes_dark[position];
            println!("Dark theme changed to: {}", theme);

            let mut settings = settings::get_settings_mut();
            settings.set_dark_theme(theme);
            if let Err(e) = settings.save() {
                eprintln!("Failed to save settings: {}", e);
            }
            drop(settings);

            settings::refresh_settings();

            apply_theme_changes_globally(&parent_window_dark);
        }
    });

    // Connect font size change handler
    font_size_spin.connect_value_changed(move |spin| {
        let font_size = spin.value() as u32;
        println!("Font size changed to: {}", font_size);

        let mut settings = settings::get_settings_mut();
        settings.set_font_size(font_size);
        if let Err(e) = settings.save() {
            eprintln!("Failed to save settings: {}", e);
        }
        drop(settings);

        settings::refresh_settings();
        crate::syntax::apply_font_size_globally(font_size);
    });

    // Connect terminal font size change handler (currently just saves, not applied)
    let parent_window_terminal = parent_window.clone();
    terminal_font_size_spin.connect_value_changed(move |spin| {
        let font_size = spin.value() as u32;
        println!("Terminal font size changed to: {}", font_size);

        let mut settings = settings::get_settings_mut();
        settings.set_terminal_font_size(font_size);
        if let Err(e) = settings.save() {
            eprintln!("Failed to save settings: {}", e);
        }
        drop(settings);

        settings::refresh_settings();

        // Apply terminal font size to all terminal instances
        println!("Looking for terminal notebook...");
        if let Some(terminal_notebook) = find_terminal_notebook(&parent_window_terminal) {
            println!("Found terminal notebook, updating font sizes...");
            terminal::update_all_terminal_font_sizes(&terminal_notebook);
        } else {
            println!("WARNING: Could not find terminal notebook!");
        }
    });

    // Handle dialog close
    gtk4::prelude::DialogExt::connect_response(&dialog, move |dialog, _response| {
        dialog.close();
    });

    dialog.upcast::<Dialog>()
}

/// Sets up a theme dropdown with available themes and current selection
fn setup_theme_dropdown(
    dropdown: &gtk4::DropDown,
    available_themes: &[String],
    current_theme: String,
) {
    // Create a string list model for the dropdown
    let model = gtk4::StringList::new(&[]);
    for theme in available_themes {
        model.append(theme);
    }

    // Set the model
    dropdown.set_model(Some(&model));

    // Set current selection
    for (idx, theme) in available_themes.iter().enumerate() {
        if theme == &current_theme {
            dropdown.set_selected(idx as u32);
            break;
        }
    }
}

/// Updates themes throughout the application
///
/// This function updates all theme-related components to reflect the current settings:
/// - Updates all editor buffers with the current syntax highlighting theme
/// - Updates all terminal tabs with matching theme colors
/// - Updates any other UI elements that depend on theme settings
///
/// Should be called after changing theme settings.
pub fn apply_theme_changes_globally(parent_window: &ApplicationWindow) {
    println!("Applying global theme changes...");

    // Get fresh settings to ensure we have the latest values
    let settings = crate::settings::get_settings();
    println!(
        "Current settings - Light theme: {}, Dark theme: {}",
        settings.get_light_theme(),
        settings.get_dark_theme()
    );

    // Use the robust buffer update function from main.rs instead of the simpler one
    // This ensures all editor buffers are updated regardless of their widget structure
    crate::update_all_buffer_themes(parent_window);

    // Find the terminal notebook if it exists
    if let Some(terminal_notebook) = find_terminal_notebook(parent_window) {
        // Update terminal themes
        terminal::update_all_terminal_themes(&terminal_notebook);
    }

    // Force a redraw of the window to ensure theme changes are visible
    parent_window.queue_draw();

    println!("Theme changes applied successfully");
}

/// Finds the terminal notebook within a window
fn find_terminal_notebook(window: &ApplicationWindow) -> Option<Notebook> {
    // Search recursively for all notebooks in the window
    fn find_all_notebooks(widget: &gtk4::Widget) -> Vec<Notebook> {
        let mut notebooks = Vec::new();

        // Check if this widget is a notebook
        if let Some(notebook) = widget.downcast_ref::<Notebook>() {
            notebooks.push(notebook.clone());
        }

        // Recursively search children
        let mut child = widget.first_child();
        while let Some(current_child) = child {
            notebooks.extend(find_all_notebooks(&current_child));
            child = current_child.next_sibling();
        }

        notebooks
    }

    println!("Searching for terminal notebook by checking all notebooks...");
    let all_notebooks = find_all_notebooks(window.upcast_ref::<gtk4::Widget>());
    println!("Found {} notebooks total", all_notebooks.len());

    // Find the notebook that contains terminal widgets
    for (idx, notebook) in all_notebooks.iter().enumerate() {
        println!("Checking notebook {}", idx);
        if notebook.n_pages() > 0 {
            if let Some(page) = notebook.nth_page(Some(0)) {
                // Check if this page contains a VteTerminal
                if contains_terminal(&page) {
                    println!("Found terminal notebook at index {}", idx);
                    return Some(notebook.clone());
                }
            }
        }
    }

    println!("No terminal notebook found");
    None
}

/// Helper function to check if a widget or its children contain a VteTerminal
fn contains_terminal(widget: &gtk4::Widget) -> bool {
    use vte4::Terminal as VteTerminal;

    // Check if this widget is a terminal
    if widget.is::<VteTerminal>() {
        return true;
    }

    // Recursively check children
    let mut child = widget.first_child();
    while let Some(current_child) = child {
        if contains_terminal(&current_child) {
            return true;
        }
        child = current_child.next_sibling();
    }

    false
}
