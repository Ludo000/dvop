// Activity Bar module - VS Code-style vertical icon panel
// Contains icon buttons for different panels (file manager, search, extensions, etc.)

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Orientation, ToggleButton};

/// Creates the activity bar - a vertical panel with icon buttons like VS Code
/// 
/// Returns:
/// - GtkBox: The activity bar container
/// - ToggleButton: The file explorer toggle button
/// - ToggleButton: The global search toggle button
pub fn create_activity_bar() -> (GtkBox, ToggleButton, ToggleButton) {
    // Create vertical box for the activity bar
    let activity_bar = GtkBox::new(Orientation::Vertical, 0);
    activity_bar.add_css_class("activity-bar");
    activity_bar.set_width_request(48); // Fixed width like VS Code
    
    // Create the file explorer button (folder icon) - first button
    let explorer_button = ToggleButton::new();
    explorer_button.set_icon_name("folder-symbolic");
    explorer_button.add_css_class("activity-bar-button");
    explorer_button.set_tooltip_text(Some("Explorer (Ctrl+Shift+E)"));
    explorer_button.set_active(true); // Start with explorer visible
    
    // Create the global search button (magnifying glass icon) - second button
    let search_button = ToggleButton::new();
    search_button.set_icon_name("system-search-symbolic");
    search_button.add_css_class("activity-bar-button");
    search_button.set_tooltip_text(Some("Search (Ctrl+Shift+F)"));
    search_button.set_active(false); // Start hidden
    
    // Add buttons to the activity bar
    activity_bar.append(&explorer_button);
    activity_bar.append(&search_button);
    
    // Make buttons mutually exclusive (only one can be active at a time)
    let search_clone = search_button.clone();
    
    explorer_button.connect_toggled(move |button| {
        if button.is_active() {
            search_clone.set_active(false);
        }
    });
    
    let explorer_clone = explorer_button.clone();
    search_button.connect_toggled(move |button| {
        if button.is_active() {
            explorer_clone.set_active(false);
        }
    });
    
    (activity_bar, explorer_button, search_button)
}
