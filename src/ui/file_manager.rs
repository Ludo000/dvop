// File Manager UI components for the Basado Text Editor
// Contains all file management panel and navigation components

use gtk4::prelude::*;
use gtk4::{
    // Layout containers  
    Box as GtkBox, ScrolledWindow,
    
    // Common UI elements
    Button, ListBox, Image, Label, Scale,
    
    // Layout orientation
    Orientation,
    
    // Drag and drop support
    DragSource, DropTarget, gdk, glib,
};

use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;

/// File clipboard operations
#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardOperation {
    Copy,
    Cut,
}

/// File clipboard data structure
#[derive(Clone, Debug)]
pub struct FileClipboard {
    pub file_path: PathBuf,
    pub operation: ClipboardOperation,
}

// Global file clipboard state - using thread-local storage for safety
thread_local! {
    static FILE_CLIPBOARD: RefCell<Option<FileClipboard>> = RefCell::new(None);
}

/// Copy a file to the clipboard
pub fn copy_file_to_clipboard(file_path: &PathBuf) {
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = Some(FileClipboard {
            file_path: file_path.clone(),
            operation: ClipboardOperation::Copy,
        });
    });
    
    let filename = file_path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    crate::status_log::log_info(&format!("Copied {} to clipboard", filename));
}

/// Cut a file to the clipboard
pub fn cut_file_to_clipboard(file_path: &PathBuf) {
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = Some(FileClipboard {
            file_path: file_path.clone(),
            operation: ClipboardOperation::Cut,
        });
    });
    
    let filename = file_path.file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    crate::status_log::log_info(&format!("Cut {} to clipboard", filename));
}

/// Check if a specific file is currently cut (not copied) in the clipboard
pub fn is_file_cut(file_path: &PathBuf) -> bool {
    FILE_CLIPBOARD.with(|clipboard| {
        if let Some(ref clipboard_content) = *clipboard.borrow() {
            clipboard_content.operation == ClipboardOperation::Cut && 
            clipboard_content.file_path == *file_path
        } else {
            false
        }
    })
}

/// Check if there's something in the file clipboard
pub fn has_clipboard_content() -> bool {
    FILE_CLIPBOARD.with(|clipboard| {
        clipboard.borrow().is_some()
    })
}

/// Get the current clipboard content (if any)
pub fn get_clipboard_content() -> Option<FileClipboard> {
    FILE_CLIPBOARD.with(|clipboard| {
        clipboard.borrow().clone()
    })
}

/// Clear the file clipboard
pub fn clear_clipboard() {
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = None;
    });
    crate::status_log::log_info("Clipboard cleared");
}

/// Paste a file from the clipboard to the target directory
pub fn paste_file_from_clipboard(
    target_dir: &PathBuf,
    window: &gtk4::ApplicationWindow,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
) {
    let clipboard_content = get_clipboard_content();
    
    if let Some(clipboard) = clipboard_content {
        let source_path = clipboard.file_path;
        let filename = source_path.file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());
        
        // Check if source file still exists
        if !source_path.exists() {
            crate::status_log::log_error(&format!("Source file {} no longer exists", filename));
            clear_clipboard();
            return;
        }
        
        // Create target path
        let mut target_path = target_dir.clone();
        target_path.push(&filename);
        
        match clipboard.operation {
            ClipboardOperation::Copy => {
                // For copy operations, handle name conflicts by generating unique names
                let final_target_path = if target_path.exists() {
                    generate_unique_filename(&target_path)
                } else {
                    target_path
                };
                
                // Copy the file
                match std::fs::copy(&source_path, &final_target_path) {
                    Ok(_) => {
                        let final_filename = final_target_path.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "file".to_string());
                        crate::status_log::log_success(&format!("Copied {} to {}", filename, final_filename));
                        
                        // Refresh file list
                        crate::utils::update_file_list(file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), crate::utils::FileSelectionSource::TabSwitch);
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!("Failed to copy {}: {}", filename, e));
                        show_error_dialog(window, &format!("Failed to copy file: {}", e));
                    }
                }
            }
            ClipboardOperation::Cut => {
                // For cut operations, check if we're moving to the same location
                if source_path == target_path {
                    // Same location - nothing to do, just clear clipboard and refresh to remove cut styling
                    crate::status_log::log_info(&format!("File {} is already in the target location", filename));
                    clear_clipboard();
                    
                    // Refresh file list to remove cut styling
                    crate::utils::update_file_list(file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), crate::utils::FileSelectionSource::TabSwitch);
                    return;
                }
                
                // For cut operations to different locations, handle conflicts by generating unique names
                let final_target_path = if target_path.exists() && source_path != target_path {
                    generate_unique_filename(&target_path)
                } else {
                    target_path
                };
                
                // Move the file
                match std::fs::rename(&source_path, &final_target_path) {
                    Ok(_) => {
                        let final_filename = final_target_path.file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "file".to_string());
                        crate::status_log::log_success(&format!("Moved {} to {}", filename, final_filename));
                        
                        // Update any open tabs that had this file open
                        crate::utils::trigger_tab_path_update(&source_path, &final_target_path);
                        
                        // Clear clipboard since cut operation is consumed
                        clear_clipboard();
                        
                        // Refresh file list
                        crate::utils::update_file_list(file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), crate::utils::FileSelectionSource::TabSwitch);
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!("Failed to move {}: {}", filename, e));
                        show_error_dialog(window, &format!("Failed to move file: {}", e));
                    }
                }
            }
        }
    } else {
        crate::status_log::log_error("No file in clipboard to paste");
    }
}

/// Generate a unique filename by appending a number if the file already exists
fn generate_unique_filename(original_path: &PathBuf) -> PathBuf {
    let parent_dir = original_path.parent().unwrap_or_else(|| std::path::Path::new("."));
    let file_stem = original_path.file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let extension = original_path.extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();
    
    let mut counter = 1;
    loop {
        let new_filename = format!("{} ({}){}", file_stem, counter, extension);
        let mut new_path = parent_dir.to_path_buf();
        new_path.push(&new_filename);
        
        if !new_path.exists() {
            return new_path;
        }
        
        counter += 1;
        
        // Prevent infinite loops
        if counter > 1000 {
            break;
        }
    }
    
    // Fallback with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let fallback_filename = format!("{}-{}{}", file_stem, timestamp, extension);
    let mut fallback_path = parent_dir.to_path_buf();
    fallback_path.push(&fallback_filename);
    fallback_path
}

/// Show an error dialog
fn show_error_dialog(window: &gtk4::ApplicationWindow, message: &str) {
    let dialog = gtk4::MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
        gtk4::MessageType::Error,
        gtk4::ButtonsType::Ok,
        message,
    );
    
    dialog.connect_response(move |d, _| {
        d.close();
    });
    
    dialog.show();
}

/// Creates the file manager panel components with drag and drop support
/// 
/// Returns a tuple containing:
/// - ListBox: The list of files and directories
/// - ScrolledWindow: Container for the file list with scrolling
pub fn create_file_manager_panel() -> (ListBox, ScrolledWindow) {
    // Create the list box that will display files and directories
    let file_list_box = ListBox::new();
    file_list_box.set_selection_mode(gtk4::SelectionMode::Single); // Allow single item selection
    file_list_box.set_can_focus(true); // Make the list box focusable for keyboard events
    file_list_box.set_focusable(true); // Ensure it can receive focus
    
    // Set up drop target for the main file list area (for dropping into current directory)
    setup_file_list_drop_target(&file_list_box);
    
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

/// Sets up drop target for the file list box to handle drops into current directory
fn setup_file_list_drop_target(file_list_box: &ListBox) {
    let drop_target = DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE | gdk::DragAction::COPY);
    
    drop_target.connect_drop(move |target, value, _x, _y| {
        // Remove visual feedback immediately and force cleanup
        if let Some(widget) = target.widget() {
            widget.remove_css_class("drop-target-background");
            if let Some(list_box) = widget.downcast_ref::<ListBox>() {
                cleanup_drag_drop_styles(list_box);
            }
        }
        
        if let Ok(source_path_str) = value.get::<String>() {
            let _source_path = std::path::PathBuf::from(&source_path_str);
            
            // Get current directory - for now we'll just show a message
            // In a full implementation, we'd need access to the current_dir state
            crate::status_log::log_info(&format!("Drop on background: {} (not implemented yet)", source_path_str));
            return true;
        }
        false
    });
    
    // Visual feedback during drag over empty space
    drop_target.connect_enter(move |target, _x, _y| {
        if let Some(widget) = target.widget() {
            widget.add_css_class("drop-target-background");
        }
        gdk::DragAction::MOVE
    });
    
    drop_target.connect_leave(move |target| {
        if let Some(widget) = target.widget() {
            widget.remove_css_class("drop-target-background");
        }
    });
    
    file_list_box.add_controller(drop_target);
}

/// Sets up drag and drop functionality for a file list row
///
/// This function configures both drag source and drop target for a file or directory item
/// in the file manager. It enables users to drag files and folders to move them around.
pub fn setup_drag_drop_for_row(
    row: &gtk4::ListBoxRow, 
    file_path: &std::path::Path,
    is_directory: bool
) {
    let file_path_clone = file_path.to_path_buf();
    let _file_name = file_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    
    // Set up drag source - what this item can be dragged as
    let drag_source = DragSource::new();
    drag_source.set_actions(gdk::DragAction::MOVE | gdk::DragAction::COPY);
    
    // Prepare drag data - we'll send the full file path as text
    let file_path_for_drag = file_path_clone.clone();
    drag_source.connect_prepare(move |_, _x, _y| {
        let file_path_str = file_path_for_drag.to_string_lossy().to_string();
        let content_provider = gdk::ContentProvider::for_value(&glib::Value::from(&file_path_str));
        Some(content_provider)
    });
    
    // Set up drag begin handler to show what's being dragged
    drag_source.connect_drag_begin(move |_source, _drag| {
        // For now, we'll skip setting a custom icon since it's complex in GTK4
        // The system will provide a default drag cursor
    });
    
    row.add_controller(drag_source);
    
    // Set up drop target - what can be dropped on this item (only for directories)
    if is_directory {
        let drop_target = DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE | gdk::DragAction::COPY);
        
        // Visual feedback during drag over
        drop_target.connect_enter(move |target, _x, _y| {
            if let Some(widget) = target.widget() {
                widget.add_css_class("drop-target");
            }
            gdk::DragAction::MOVE
        });
        
        drop_target.connect_leave(move |target| {
            if let Some(widget) = target.widget() {
                widget.remove_css_class("drop-target");
            }
        });
        
        // Ensure drop target class is removed after drop operation
        let target_dir_for_cleanup = file_path_clone.clone();
        drop_target.connect_drop(move |target, value, _x, _y| {
            // Remove visual feedback immediately
            if let Some(widget) = target.widget() {
                widget.remove_css_class("drop-target");
                // Also try to get the parent list box and clean it up
                if let Some(parent) = widget.parent() {
                    if let Some(list_box) = parent.downcast_ref::<ListBox>() {
                        cleanup_drag_drop_styles(list_box);
                    }
                }
            }
            
            if let Ok(source_path_str) = value.get::<String>() {
                let source_path = std::path::PathBuf::from(&source_path_str);
                let target_path = target_dir_for_cleanup.clone();
                
                // Show confirmation modal for the move operation
                crate::ui::file_manager::show_move_confirmation_modal(&source_path, &target_path);
                return true;
            }
            false
        });
        
        row.add_controller(drop_target);
    }
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

/// Forces cleanup of all drag and drop CSS classes from the file list
///
/// This function ensures that no drag-drop visual feedback persists after operations complete
pub fn cleanup_drag_drop_styles(file_list_box: &ListBox) {
    // Iterate through all rows and remove any drag-drop related CSS classes
    let mut child = file_list_box.first_child();
    while let Some(current_child) = child {
        if let Some(row) = current_child.downcast_ref::<gtk4::ListBoxRow>() {
            row.remove_css_class("drop-target");
            row.remove_css_class("drop-target-background");
        }
        current_child.remove_css_class("drop-target");
        current_child.remove_css_class("drop-target-background");
        child = current_child.next_sibling();
    }
    
    // Also remove from the file list box itself
    file_list_box.remove_css_class("drop-target");
    file_list_box.remove_css_class("drop-target-background");
}

/// Shows a confirmation modal for file/folder move operations
///
/// This function displays a dialog asking the user to confirm moving a file or folder
/// from the source location to the target directory.
pub fn show_move_confirmation_modal(source_path: &std::path::Path, target_dir: &std::path::Path) {
    use gtk4::{MessageDialog, DialogFlags, MessageType, ButtonsType, ResponseType};
    
    let source_name = source_path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown item");
    
    let target_name = target_dir.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown folder");
    
    // Determine if we're moving a file or directory
    let item_type = if source_path.is_dir() { "folder" } else { "file" };
    
    // Build the target path where the item would be moved
    let mut final_target = target_dir.to_path_buf();
    final_target.push(source_name);
    
    // Check if target already exists
    let conflict_message = if final_target.exists() {
        format!("\n\nWarning: A {} with this name already exists in the destination. It will be replaced.", item_type)
    } else {
        String::new()
    };
    
    let message = format!(
        "Move {} \"{}\" to folder \"{}\"?{}",
        item_type, source_name, target_name, conflict_message
    );
    
    // Find the application window to use as parent
    if let Some(app) = gtk4::gio::Application::default() {
        if let Some(gtk_app) = app.downcast_ref::<gtk4::Application>() {
            if let Some(window) = gtk_app.active_window() {
                let dialog = MessageDialog::new(
                    Some(&window),
                    DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                    MessageType::Question,
                    ButtonsType::None,
                    &message
                );
                
                dialog.add_buttons(&[
                    ("Cancel", ResponseType::Cancel),
                    ("Move", ResponseType::Accept),
                ]);
                
                dialog.set_default_response(ResponseType::Cancel);
                
                let source_path = source_path.to_path_buf();
                let final_target = final_target.clone();
                let window_clone = window.clone();
                
                dialog.connect_response(move |d, response| {
                    if response == ResponseType::Accept {
                        perform_file_move(&source_path, &final_target, &window_clone);
                    }
                    d.close();
                });
                
                dialog.show();
            }
        }
    }
}

/// Performs the actual file/folder move operation
///
/// This function handles the filesystem operation of moving a file or directory
/// and shows appropriate success or error messages.
fn perform_file_move(source: &std::path::Path, target: &std::path::Path, window: &gtk4::Window) {
    use gtk4::{MessageDialog, DialogFlags, MessageType, ButtonsType};
    
    let source_name = source.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown item");
    
    // Attempt to move the file/directory
    match std::fs::rename(source, target) {
        Ok(()) => {
            // Success - log it
            crate::status_log::log_success(&format!("Moved {} successfully", source_name));
            
            // Update any open tabs that had this file open
            crate::utils::trigger_tab_path_update(&source.to_path_buf(), &target.to_path_buf());
            
            // Force cleanup of any lingering drag-drop styles
            if let Some(app) = gtk4::gio::Application::default() {
                if let Some(gtk_app) = app.downcast_ref::<gtk4::Application>() {
                    if let Some(active_window) = gtk_app.active_window() {
                        cleanup_all_drag_drop_styles(active_window.upcast_ref::<gtk4::Widget>());
                    }
                }
            }
            
            // Trigger a file list refresh by sending a custom event
            refresh_file_list_after_move();
        }
        Err(e) => {
            // Error - show error dialog
            let error_msg = format!("Failed to move {}: {}", source_name, e);
            crate::status_log::log_error(&error_msg);
            
            let error_dialog = MessageDialog::new(
                Some(window),
                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                MessageType::Error,
                ButtonsType::Ok,
                &error_msg
            );
            
            error_dialog.show();
        }
    }
}

/// Triggers a refresh of the file list after a file move operation
/// This uses the callback system to refresh the file list
fn refresh_file_list_after_move() {
    // Trigger the refresh callback with a small delay to ensure the filesystem operation is complete
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), || {
        crate::utils::trigger_file_list_refresh();
    });
}

/// Recursively finds and cleans up all drag-drop styles from a widget hierarchy
///
/// This function walks through the widget tree and removes any lingering drag-drop CSS classes
fn cleanup_all_drag_drop_styles(widget: &gtk4::Widget) {
    // Remove classes from this widget
    widget.remove_css_class("drop-target");
    widget.remove_css_class("drop-target-background");
    
    // If this is a ListBox, use the specialized cleanup
    if let Some(list_box) = widget.downcast_ref::<ListBox>() {
        cleanup_drag_drop_styles(list_box);
    }
    
    // Recursively clean up all children
    let mut child = widget.first_child();
    while let Some(current_child) = child {
        cleanup_all_drag_drop_styles(&current_child);
        child = current_child.next_sibling();
    }
}
