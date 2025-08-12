// Settings UI module for the Basado Text Editor
// Contains settings dialog and theme management functions

use gtk4::prelude::*;
use gtk4::{
    ApplicationWindow, 
    Box as GtkBox, 
    Label, 
    Dialog,
    Orientation,
    Notebook
};
use sourceview5::StyleSchemeManager;
use crate::syntax;
use crate::settings;
use crate::ui::terminal;

/// Creates a settings dialog for configuring editor preferences
///
/// This function creates a dialog where the user can:
/// - Choose preferred syntax highlighting color schemes
/// - Set other editor preferences
///
/// Returns the dialog for display
pub fn create_settings_dialog(parent: &ApplicationWindow) -> Dialog {
    // Create a dialog with standard buttons
    let dialog = Dialog::builder()
        .title("Editor Settings")
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .use_header_bar(1) // Use header bar
        .build();
    
    // Get the content area to add our widgets
    let content_area = dialog.content_area();
    content_area.set_margin_top(10);
    content_area.set_margin_bottom(10);
    content_area.set_margin_start(10);
    content_area.set_margin_end(10);
    content_area.set_spacing(10);
    
    // Create a container for the settings
    let settings_box = GtkBox::new(Orientation::Vertical, 10);
    
    // Create a section for syntax highlighting themes
    let themes_label = Label::new(Some("Syntax Highlighting Themes"));
    themes_label.set_halign(gtk4::Align::Start);
    themes_label.set_margin_bottom(5);
    themes_label.add_css_class("heading");
    settings_box.append(&themes_label);
    
    // Add info about current system theme mode
    let system_mode = if syntax::is_dark_mode_enabled() {
        "dark mode"
    } else {
        "light mode" 
    };
    let current_system_theme_name = syntax::get_preferred_style_scheme();
    let theme_info = Label::new(Some(&format!("Current system theme: {} (using {})", current_system_theme_name, system_mode)));
    theme_info.set_halign(gtk4::Align::Start);
    theme_info.set_margin_bottom(10);
    theme_info.add_css_class("caption");
    settings_box.append(&theme_info);
    
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
    
    // Create dropdowns for light and dark themes
    // Select the appropriate theme based on the current system state
    let light_theme_box = if !syntax::is_dark_mode_enabled() {
        // If we're in light mode, prioritize the current system theme for the light theme dropdown
        create_theme_selection_box("Light Mode Theme:", &available_schemes, current_system_theme.clone())
    } else {
        create_theme_selection_box("Light Mode Theme:", &available_schemes, current_light_theme)
    };
    
    let dark_theme_box = if syntax::is_dark_mode_enabled() {
        // If we're in dark mode, prioritize the current system theme for the dark theme dropdown
        create_theme_selection_box("Dark Mode Theme:", &available_schemes, current_system_theme.clone())
    } else {
        create_theme_selection_box("Dark Mode Theme:", &available_schemes, current_dark_theme)
    };
    
    settings_box.append(&light_theme_box.0);
    settings_box.append(&dark_theme_box.0);
    
    // Add the settings box to the content area
    content_area.append(&settings_box);
    
    // Add save and cancel buttons
    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
    dialog.add_button("Save", gtk4::ResponseType::Accept);
    dialog.set_default_response(gtk4::ResponseType::Accept);
    
    // Handle the dialog response
    // We need to capture the dropdowns and available_schemes to get their values when the user clicks Save
    let light_dropdown = light_theme_box.1;
    let dark_dropdown = dark_theme_box.1;
    let available_schemes_clone = available_schemes.clone();
    
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            // Get the selected theme values from the position in the dropdown
            let light_position = light_dropdown.selected() as usize;
            if light_position < available_schemes_clone.len() {
                let light_theme = available_schemes_clone[light_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_light_theme(&light_theme);
            }
            
            let dark_position = dark_dropdown.selected() as usize;
            if dark_position < available_schemes_clone.len() {
                let dark_theme = available_schemes_clone[dark_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_dark_theme(&dark_theme);
            }
            
            // Save settings to disk
            if let Err(e) = settings::get_settings_mut().save() {
                eprintln!("Failed to save settings: {}", e);
            }
            
            // Release the mutex before refreshing settings
            drop(settings::get_settings_mut());
            
            // Refresh settings across the application
            settings::refresh_settings();
            
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
    
    dialog
}

/// Creates a theme selection dropdown with label
///
/// Returns a tuple containing:
/// - A container with the label and dropdown
/// - The dropdown widget for connecting signals
fn create_theme_selection_box(label_text: &str, available_themes: &[String], current_theme: String) 
    -> (GtkBox, gtk4::DropDown) 
{
    let box_container = GtkBox::new(Orientation::Horizontal, 10);
    
    // Add label
    let label = Label::new(Some(label_text));
    label.set_halign(gtk4::Align::Start);
    label.set_width_chars(20);
    label.set_xalign(0.0);
    box_container.append(&label);
    
    // Create a string list model for the dropdown
    let model = gtk4::StringList::new(&[]);
    for theme in available_themes {
        model.append(theme);
    }
    
    // Create dropdown
    let dropdown = gtk4::DropDown::new(Some(model), None::<gtk4::Expression>);
    dropdown.set_hexpand(true);
    
    // Set current selection
    for (idx, theme) in available_themes.iter().enumerate() {
        if theme == &current_theme {
            dropdown.set_selected(idx as u32);
            break;
        }
    }
    
    box_container.append(&dropdown);
    
    (box_container, dropdown)
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
