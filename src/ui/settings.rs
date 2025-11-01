// Settings UI module for Dvop
// Contains settings dialog and theme management functions

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, 
    Dialog,
    Notebook
};
use sourceview5::StyleSchemeManager;
use crate::syntax;
use crate::settings;
use crate::ui::terminal;

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
    theme_info.set_text(&format!("Current system theme: {} (using {})", current_system_theme_name, system_mode));
    
    // Get available color schemes
    let scheme_manager = StyleSchemeManager::new();
    let available_schemes: Vec<String> = scheme_manager.scheme_ids()
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
    
    println!("Settings dialog using - Light theme: {}, Dark theme: {}", 
             current_light_theme, current_dark_theme);
    
    // Get current theme based on system theme
    let current_system_theme = syntax::get_preferred_style_scheme();
    
    // Setup dropdown models and selections
    setup_theme_dropdown(&light_theme_dropdown, &available_schemes, 
        if !syntax::is_dark_mode_enabled() { current_system_theme.clone() } else { current_light_theme });
    
    setup_theme_dropdown(&dark_theme_dropdown, &available_schemes,
        if syntax::is_dark_mode_enabled() { current_system_theme.clone() } else { current_dark_theme });
    
    // Get current font size settings
    let settings_instance = settings::get_settings();
    font_size_spin.set_value(settings_instance.get_font_size() as f64);
    // Use editor font size for terminal if no separate terminal setting exists
    terminal_font_size_spin.set_value(settings_instance.get_font_size() as f64);
    
    // Handle the dialog response
    let available_schemes_clone = available_schemes.clone();
    
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            // Get the selected theme values from the position in the dropdown
            let light_position = light_theme_dropdown.selected() as usize;
            if light_position < available_schemes_clone.len() {
                let light_theme = available_schemes_clone[light_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_light_theme(&light_theme);
            }
            
            let dark_position = dark_theme_dropdown.selected() as usize;
            if dark_position < available_schemes_clone.len() {
                let dark_theme = available_schemes_clone[dark_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_dark_theme(&dark_theme);
            }
            
            // Get the font size values and save them
            let font_size = font_size_spin.value() as u32;
            let _terminal_font_size = terminal_font_size_spin.value() as u32;
            {
                let mut settings = settings::get_settings_mut();
                settings.set_font_size(font_size);
                // Note: Terminal font size setting not yet implemented in EditorSettings
            }
            
            // Save settings to disk
            if let Err(e) = settings::get_settings_mut().save() {
                eprintln!("Failed to save settings: {}", e);
            }
            
            // Release the mutex before refreshing settings
            drop(settings::get_settings_mut());
            
            // Refresh settings across the application
            settings::refresh_settings();
            
            // Apply font size changes globally
            crate::syntax::apply_font_size_globally(font_size);
            
            // Get a reference to the parent window to update themes
            if let Some(parent) = dialog.transient_for() {
                if let Ok(parent_window) = parent.downcast::<ApplicationWindow>() {
                    // Apply theme changes throughout the application
                    apply_theme_changes_globally(&parent_window);
                }
            }
        }
        
        dialog.close();
    });
    
    dialog.upcast::<Dialog>()
}

/// Sets up a theme dropdown with available themes and current selection
fn setup_theme_dropdown(dropdown: &gtk4::DropDown, available_themes: &[String], current_theme: String) {
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
    println!("Current settings - Light theme: {}, Dark theme: {}", 
             settings.get_light_theme(), settings.get_dark_theme());
    
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
    // Look through the window structure to find the terminal notebook
    // This is specific to the structure of our application window
    
    window.child()
        .and_then(|main_box| main_box.first_child())  // Main content box
        .and_then(|paned| paned.last_child())         // The horizontal paned container
        .and_then(|editor_paned| editor_paned.last_child()) // The vertical paned container
        .and_then(|terminal_box| terminal_box.first_child())
        .and_then(|child| {
            // Check if this is our terminal notebook
            child.downcast::<Notebook>().ok()
        })
}
