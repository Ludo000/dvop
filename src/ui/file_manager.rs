// File Manager UI components for the Basado Text Editor
// Contains all file management panel and navigation components

use gtk4::prelude::*;
use gtk4::{
    // Layout containers  
    Box as GtkBox, ScrolledWindow,
    
    // Common UI elements
    Button, ListBox, Image,
    
    // Layout orientation
    Orientation
};

/// Creates the file manager panel components
/// 
/// Returns a tuple containing:
/// - ListBox: The list of files and directories
/// - ScrolledWindow: Container for the file list with scrolling
pub fn create_file_manager_panel() -> (ListBox, ScrolledWindow) {
    // Create the list box that will display files and directories
    let file_list_box = ListBox::new();
    file_list_box.set_selection_mode(gtk4::SelectionMode::Single); // Allow single item selection
    
    // Place the list box in a scrolled window
    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never)       // No horizontal scrollbar
        .vscrollbar_policy(gtk4::PolicyType::Automatic)   // Show vertical scrollbar when needed
        .child(&file_list_box)
        .vexpand(true)                                    // Expand vertically to fill space
        .build();

    // Return the components for further assembly and event handling
    (file_list_box, scrolled_window)
}

/// Assembles the file manager panel from its components
/// 
/// Takes the file list and creates a single container
pub fn create_file_manager_panel_container(file_list_scrolled_window: ScrolledWindow) -> GtkBox {
    // Create a vertical box to hold all file manager components
    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    file_manager_panel.add_css_class("file-manager-panel"); // Add CSS class for styling
    
    // Add the scrollable file list
    file_manager_panel.append(&file_list_scrolled_window);
    
    // Make the panel expand vertically to use available space
    file_manager_panel.set_vexpand(true);
    
    file_manager_panel
}

/// Creates a path bar for displaying the current directory path with navigation buttons
///
/// This function creates a horizontal bar with navigation buttons and a path box to display 
/// the current directory path as a series of clickable buttons. This is designed to be
/// placed between the header bar and the main content.
/// 
/// Returns a tuple of:
/// - GtkBox: The path bar container
/// - GtkBox: The path box that will contain individual path segment buttons
/// - Button: Up button for navigating to parent directory
/// - Button: Refresh button for updating the file list
/// - Button: Open in Terminal button for opening the current directory in a terminal
pub fn create_path_bar() -> (GtkBox, GtkBox, Button, Button, Button) {
    // Create a horizontal box for the path bar
    let path_bar = GtkBox::new(Orientation::Horizontal, 5);
    path_bar.set_margin_start(10);
    path_bar.set_margin_end(10);
    path_bar.set_margin_top(6);
    path_bar.set_margin_bottom(6);
    
    // Create the "Up" button with a standard icon
    let up_button_icon = Image::from_icon_name("go-up-symbolic");
    let up_button = Button::new();
    up_button.set_child(Some(&up_button_icon));
    up_button.set_tooltip_text(Some("Go to parent directory"));
    up_button.set_margin_end(2); // Add spacing from path
    
    // Create a horizontal box to hold the path segment buttons
    let path_box = GtkBox::new(Orientation::Horizontal, 2);
    path_box.set_halign(gtk4::Align::Start); // Align to the left
    path_box.set_hexpand(true); // Use all available horizontal space
    
    // Add some styling to make the path box visually distinct
    path_box.add_css_class("path-box");
    
    // Create the "Refresh" button with a standard icon
    let refresh_button_icon = Image::from_icon_name("view-refresh-symbolic");
    let refresh_button = Button::new();
    refresh_button.set_child(Some(&refresh_button_icon));
    refresh_button.set_tooltip_text(Some("Refresh file list"));
    refresh_button.set_margin_start(2); // Add spacing from path
    refresh_button.set_margin_end(2); // Reduced spacing before terminal button
    
    // Create the "Open in Terminal" button with a terminal icon
    let terminal_button_icon = Image::from_icon_name("utilities-terminal-symbolic");
    let terminal_button = Button::new();
    terminal_button.set_child(Some(&terminal_button_icon));
    terminal_button.set_tooltip_text(Some("Open current folder in a new terminal"));
    terminal_button.set_margin_start(2); // Reduced spacing from refresh button
    
    // Assemble the path bar: up button, path, refresh button, terminal button
    path_bar.append(&up_button);
    path_bar.append(&path_box);
    path_bar.append(&refresh_button);
    path_bar.append(&terminal_button);
    
    // Add a CSS class for custom styling
    path_bar.add_css_class("basado-path-bar");
    
    (path_bar, path_box, up_button, refresh_button, terminal_button)
}
