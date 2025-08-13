// File Manager UI components for the Basado Text Editor
// Contains all file management panel and navigation components

use gtk4::prelude::*;
use gtk4::{
    // Layout containers  
    Box as GtkBox, ScrolledWindow,
    
    // Common UI elements
    Button, ListBox, Image, Label, Scale,
    
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
/// Creates the path navigation bar with file path, volume control, and action buttons
/// 
/// The path bar layout is organized as follows:
/// - Up button (left)
/// - Path breadcrumbs (middle, expandable)
/// - Volume control (right side, conditionally visible for music files)
/// - Refresh button (right side)
/// - Terminal button (rightmost)
///
/// Returns a tuple containing:
/// - GtkBox: The complete path bar container
/// - GtkBox: The path breadcrumb container
/// - Button: Up navigation button
/// - Button: Refresh file list button  
/// - Button: Open terminal button
/// - GtkBox: Volume control container (for showing/hiding)
/// - Scale: Volume slider widget
pub fn create_path_bar() -> (GtkBox, GtkBox, Button, Button, Button, GtkBox, Scale) {
    // Create a horizontal box for the path bar
    let path_bar = GtkBox::new(Orientation::Horizontal, 5);
    path_bar.set_margin_start(10);
    path_bar.set_margin_end(10);
    path_bar.set_margin_top(6);
    path_bar.set_margin_bottom(6);
    path_bar.set_vexpand(false); // Prevent vertical expansion
    path_bar.set_valign(gtk4::Align::Center); // Center contents vertically
    path_bar.set_height_request(36); // Fixed height to prevent shifts when volume control appears/disappears
    
    // Create the "Up" button with a standard icon
    let up_button_icon = Image::from_icon_name("go-up-symbolic");
    let up_button = Button::new();
    up_button.set_child(Some(&up_button_icon));
    up_button.set_tooltip_text(Some("Go to parent directory"));
    up_button.set_margin_end(2); // Add spacing from path
    up_button.set_valign(gtk4::Align::Center); // Center vertically
    
    // Create a horizontal box to hold the path segment buttons
    let path_box = GtkBox::new(Orientation::Horizontal, 2);
    path_box.set_halign(gtk4::Align::Start); // Align to the left
    path_box.set_hexpand(true); // Use all available horizontal space
    
    // Add some styling to make the path box visually distinct
    path_box.add_css_class("path-box");
    
    // Create global volume control - compact design for path bar
    let volume_control_box = GtkBox::new(Orientation::Horizontal, 4); // Reduced spacing from 6 to 4
    volume_control_box.set_halign(gtk4::Align::End);
    volume_control_box.set_valign(gtk4::Align::Center); // Center vertically in path bar
    volume_control_box.set_margin_start(12);
    volume_control_box.set_margin_end(8);
    volume_control_box.set_margin_top(0); // Remove any vertical margins
    volume_control_box.set_margin_bottom(0);
    volume_control_box.set_height_request(24); // Fixed height to prevent layout shifts
    volume_control_box.set_vexpand(false); // Prevent vertical expansion
    
    // Volume icon - smaller for compact design
    let volume_icon = Image::from_icon_name("audio-volume-medium-symbolic");
    volume_icon.set_pixel_size(14); // Reduced from 16 to 14
    volume_icon.set_tooltip_text(Some("Global Volume"));
    volume_icon.set_valign(gtk4::Align::Center);
    volume_control_box.append(&volume_icon);
    
    // Global volume scale - compact height
    let global_volume_scale = Scale::with_range(Orientation::Horizontal, 0.0, 1.0, 0.01);
    global_volume_scale.set_size_request(100, 20); // Set explicit height to 20px
    global_volume_scale.set_hexpand(false);
    global_volume_scale.set_vexpand(false); // Prevent vertical expansion
    global_volume_scale.set_valign(gtk4::Align::Center);
    global_volume_scale.set_tooltip_text(Some("Global Audio Volume"));
    global_volume_scale.add_css_class("global-volume-scale");
    
    // Set initial volume from settings
    let initial_volume = crate::settings::get_settings().get_audio_volume();
    global_volume_scale.set_value(initial_volume);
    
    volume_control_box.append(&global_volume_scale);
    
    // Volume percentage label - compact design
    let volume_percent = (initial_volume * 100.0) as i32;
    let volume_label = Label::new(Some(&format!("{}%", volume_percent)));
    volume_label.set_size_request(28, 20); // Set explicit height to match scale
    volume_label.set_valign(gtk4::Align::Center);
    volume_label.set_margin_top(0); // Remove vertical margins
    volume_label.set_margin_bottom(0);
    volume_label.add_css_class("volume-percent");
    volume_control_box.append(&volume_label);
    
    // Set up volume scale change handler
    let volume_icon_clone = volume_icon.clone();
    let volume_label_clone = volume_label.clone();
    global_volume_scale.connect_value_changed(move |scale| {
        let volume = scale.value();
        
        // Update global volume via audio module
        crate::audio::set_global_volume(volume);
        
        // Update percentage label
        let percent = (volume * 100.0) as i32;
        volume_label_clone.set_text(&format!("{}%", percent));
        
        // Update volume icon based on level
        let icon_name = if volume < 0.01 {
            "audio-volume-muted-symbolic"
        } else if volume < 0.33 {
            "audio-volume-low-symbolic"
        } else if volume < 0.67 {
            "audio-volume-medium-symbolic"
        } else {
            "audio-volume-high-symbolic"
        };
        volume_icon_clone.set_icon_name(Some(icon_name));
    });
    
    // Create the "Refresh" button with a standard icon
    let refresh_button_icon = Image::from_icon_name("view-refresh-symbolic");
    let refresh_button = Button::new();
    refresh_button.set_child(Some(&refresh_button_icon));
    refresh_button.set_tooltip_text(Some("Refresh file list"));
    refresh_button.set_margin_start(2); // Add spacing from volume control
    refresh_button.set_margin_end(2); // Reduced spacing before terminal button
    refresh_button.set_valign(gtk4::Align::Center); // Center vertically
    
    // Create the "Open in Terminal" button with a terminal icon
    let terminal_button_icon = Image::from_icon_name("utilities-terminal-symbolic");
    let terminal_button = Button::new();
    terminal_button.set_child(Some(&terminal_button_icon));
    terminal_button.set_tooltip_text(Some("Open current folder in a new terminal"));
    terminal_button.set_margin_start(2); // Reduced spacing from refresh button
    terminal_button.set_valign(gtk4::Align::Center); // Center vertically
    
    // Assemble the path bar: up button, path, volume control, refresh button, terminal button
    path_bar.append(&up_button);
    path_bar.append(&path_box);
    path_bar.append(&volume_control_box);
    path_bar.append(&refresh_button);
    path_bar.append(&terminal_button);
    
    // Add a CSS class for custom styling
    path_bar.add_css_class("basado-path-bar");
    
    // Initially hide volume controls (will be shown when music content is active)
    volume_control_box.set_visible(false);
    
    (path_bar, path_box, up_button, refresh_button, terminal_button, volume_control_box, global_volume_scale)
}
