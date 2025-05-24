// Utility functions for the Basado Text Editor
// This module contains helper functions used throughout the application

use gtk4::prelude::*;
use gtk4::{Button, ListBox, MenuButton, pango, ApplicationWindow, EventControllerKey, gdk, glib};
use std::path::PathBuf;
use std::rc::Rc;
use std::cell::RefCell;
use mime_guess;
use mime_guess::Mime;

/// Checks if a MIME type is supported for editing in the text editor
///
/// This function determines whether a file with the given MIME type
/// should be opened for text editing or treated differently (e.g., as an image)
pub fn is_allowed_mime_type(mime_type: &Mime) -> bool {
    // Allow all text formats
    mime_type.type_() == "text" ||
    
    // Allow common file types that are text-based but might have different MIME types
    mime_type == &mime_guess::mime::APPLICATION_OCTET_STREAM ||
    mime_type == &mime_guess::mime::APPLICATION_JSON ||
    mime_type == &mime_guess::mime::APPLICATION_JAVASCRIPT ||
    
    // Additional check for text subtypes
    mime_type.type_().as_str().starts_with("text/") ||
    
    // Allow specific application types that are typically text-based
    mime_type.essence_str() == "application/xml" ||
    mime_type.essence_str() == "application/x-httpd-php" ||
    mime_type.essence_str() == "application/x-mspublisher"
}

/// Updates the file browser list with contents of the current directory
///
/// This function refreshes the file list to show folders and files in the current directory,
/// and highlights the currently open file if applicable.
pub fn update_file_list(file_list_box: &ListBox, current_dir: &PathBuf, file_path: &Option<PathBuf>) {
    // Clear the existing list contents
    while let Some(child) = file_list_box.first_child() {
        file_list_box.remove(&child);
    }

    // Clear any selection
    file_list_box.unselect_all();

    // Separate containers for folders and files
    // We'll display folders first, then files, both in alphabetical order
    let mut folders = Vec::new();
    let mut files = Vec::new();

    // Read directory contents
    if let Ok(entries) = std::fs::read_dir(current_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                // Skip hidden files (starting with .)
                if file_name_str.starts_with('.') {
                    continue;
                }

                // Categorize as folder or file
                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    folders.push((file_name_str.to_string(), entry));
                } else {
                    files.push((file_name_str.to_string(), entry));
                }
            }
        }
    }

    // Sort folders and files alphabetically
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // Add folders to the list first with bold formatting
    for (file_name_str, _entry) in folders {
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&file_name_str));
        label.set_halign(gtk4::Align::Start);        // Left-align text
        label.set_margin_start(5);                   // Add left margin
        label.set_ellipsize(pango::EllipsizeMode::End);

        // Make folder names bold for better visual distinction
        label.set_markup(&format!("<span weight=\"bold\">{}</span>", file_name_str));
        
        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    // Track which row should be selected (if any)
    let mut selected_row = None;
    
    // Add files to the list
    for (file_name_str, entry) in files {
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&file_name_str));
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(5);
        label.set_ellipsize(pango::EllipsizeMode::End);

        // Check if this file is the currently open one by comparing full paths
        if let Some(ref open_file_full_path) = file_path {
            let current_entry_full_path = entry.path(); // Get PathBuf from DirEntry
            if &current_entry_full_path == open_file_full_path {
                label.set_markup(&format!("<u>{}</u>", file_name_str));
                selected_row = Some(row.clone());
            }
        }

        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    // If we found the currently open file in the list, select it
    if let Some(row) = selected_row {
        file_list_box.select_row(Some(&row));
    } else {
        file_list_box.unselect_all();
    }
}

/// Updates the visibility of save buttons based on content type
///
/// Disables save functionality for content types that can't be edited,
/// such as images which are display-only in this editor.
pub fn update_save_buttons_visibility(save_button: &Button, save_as_button: &Button, mime_type: Option<mime_guess::Mime>) {
    match mime_type {
        // For images, disable save functionality since we don't support image editing
        Some(mime) if mime.type_() == "image" => {
            save_button.set_visible(false);
            save_as_button.set_visible(false);
        },
        // For all other content types (text, etc.), enable save functionality
        _ => {
            save_button.set_visible(true);
            save_as_button.set_visible(true);
        }
    }
}

/// Updates the visibility of the save menu button based on content type
///
/// Similar to update_save_buttons_visibility, but for the split button menu component
pub fn update_save_menu_button_visibility(save_menu_button: &MenuButton, mime_type: Option<mime_guess::Mime>) {
    match mime_type {
        // Hide menu button for images since saving is not supported
        Some(mime) if mime.type_() == "image" => {
            // Get the parent widget of the menu button to hide the entire split button
            if let Some(parent) = save_menu_button.parent() {
                parent.set_visible(false); // Hide the entire container (split button)
            } else {
                // Fallback if there's no parent for some reason
                save_menu_button.set_visible(false);
            }
        },
        _ => {
            // Show the entire split button
            if let Some(parent) = save_menu_button.parent() {
                parent.set_visible(true); // Show the entire container (split button)
            } else {
                // Fallback if there's no parent for some reason
                save_menu_button.set_visible(true);
            }
        }
    }
}

/// Updates the status bar path label with the current directory path
///
/// Updates the path label in the status bar with the current directory path
/// 
/// This formats the path in a user-friendly way and should be called whenever
/// the current directory changes.
pub fn update_path_label(path_label: &gtk4::Label, current_dir: &PathBuf) {
    // Simply display the full path for better reliability
    path_label.set_text(&format!("{}", current_dir.display()));
    
    // Set tooltip to show the full path on hover (helpful for long paths)
    path_label.set_tooltip_text(Some(&current_dir.display().to_string()));
    
    // Make the path label look interactive
    path_label.add_css_class("clickable-path");
}

/// Parses a PathBuf into its component segments
///
/// Returns a vector of (display_name, full_path) tuples for each segment of the path
/// Each tuple contains the segment name and the full path to that segment
pub fn parse_path_components(path: &PathBuf) -> Vec<(String, PathBuf)> {
    let mut components = Vec::new();
    let mut current = PathBuf::new();
    
    // Get user's home directory
    let home_dir = home::home_dir();
    
    // Check if the path is under the user's home directory
    if let Some(home) = &home_dir {
        if path.starts_with(home) {
            // Start with home directory
            current = home.clone();
            components.push(("Home".to_string(), current.clone()));
            
            // Skip the parts that are already included in the home path
            let rel_path = path.strip_prefix(home).unwrap_or(path);
            for component in rel_path.components() {
                match component {
                    std::path::Component::Normal(os_str) => {
                        if let Some(name) = os_str.to_str() {
                            current.push(name);
                            components.push((name.to_string(), current.clone()));
                        }
                    },
                    _ => {} // Skip other component types
                }
            }
            
            return components;
        }
    }
    
    // For paths not under home, start with root if it's an absolute path
    if path.is_absolute() {
        current.push("/");
        components.push(("Root".to_string(), current.clone()));
    }
    
    // Add each path component with its full path
    for component in path.components() {
        match component {
            std::path::Component::Normal(os_str) => {
                if let Some(name) = os_str.to_str() {
                    current.push(name);
                    components.push((name.to_string(), current.clone()));
                }
            },
            // Handle other path component types if needed
            std::path::Component::RootDir => {
                if components.is_empty() { // Only add if not already added
                    current = PathBuf::from("/");
                    components.push(("/".to_string(), current.clone()));
                }
            },
            std::path::Component::ParentDir => {
                if !current.as_os_str().is_empty() {
                    current.pop();
                    // Parent directory component (..) - not adding to components list
                }
            },
            _ => {} // Skip other component types
        }
    }
    
    components
}

/// Updates the status bar path box with clickable buttons for each path segment
///
/// This creates a series of buttons, one for each directory in the path,
/// allowing the user to click on any folder to navigate directly to it.
pub fn update_path_buttons(
    path_box: &gtk4::Box,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &gtk4::ListBox,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>
) {
    let current_path = current_dir.borrow().clone();
    // Clear any existing buttons
    while let Some(child) = path_box.first_child() {
        path_box.remove(&child);
    }
    
    // Get path components
    let components = parse_path_components(&current_path);
    
    // Create a button for each path component
    for (i, (display_name, path)) in components.iter().enumerate() {
        // Create a button for this path segment
        let button = gtk4::Button::new();
        
        // Special handling for home and root directories
        if i == 0 {
            if display_name == "Home" {
                // Use home icon for user's home directory
                let icon = gtk4::Image::from_icon_name("user-home-symbolic");
                button.set_child(Some(&icon));
                button.set_tooltip_text(Some("Home Directory"));
            } else if display_name == "Root" {
                // Use drive icon for root directory
                let icon = gtk4::Image::from_icon_name("drive-harddisk-symbolic");
                button.set_child(Some(&icon));
                button.set_tooltip_text(Some("Root Directory"));
            } else {
                button.set_label(display_name);
            }
        } else {
            button.set_label(display_name);
        }
        
        // Add styling
        button.add_css_class("path-segment-button");
        button.set_has_frame(false);  // Make it look like a link
        
        // Clone needed variables for the closure
        let path_clone = path.clone();
        let file_list_box_clone = file_list_box.clone();
        let active_tab_path_clone = active_tab_path.clone();
        let current_dir_clone = current_dir.clone();
        
        // We need weak references to the path_box to avoid ownership issues
        let path_box_weak = glib::object::WeakRef::new();
        path_box_weak.set(Some(path_box));
        
        // Connect clicked signal
        button.connect_clicked(move |_| {
            // Navigate to this path segment by updating the current_dir
            *current_dir_clone.borrow_mut() = path_clone.clone();
            
            // Update the file list to show this directory
            update_file_list(
                &file_list_box_clone,
                &current_dir_clone.borrow(),
                &active_tab_path_clone.borrow()
            );
            
            // Update path buttons to reflect the new current directory
            // Get the path_box from the weak reference
            if let Some(pb) = path_box_weak.upgrade() {
                if let Some(box_widget) = pb.downcast_ref::<gtk4::Box>() {
                    update_path_buttons(box_widget, &current_dir_clone, &file_list_box_clone, &active_tab_path_clone);
                }
            }
        });
        
        // Add a separator after all but the last component
        path_box.append(&button);
        if i < components.len() - 1 {
            let separator = gtk4::Label::new(Some("/"));
            separator.add_css_class("path-separator");
            path_box.append(&separator);
        }
    }
}

/// Sets up common keyboard shortcuts for the application
///
/// This function adds keyboard shortcuts like Ctrl+S for saving, Ctrl+O for opening files,
/// Ctrl+N for new files, Ctrl+Tab for navigating tabs, and other standard editor shortcuts.
pub fn setup_keyboard_shortcuts(
    window: &ApplicationWindow, 
    save_button: &Button, 
    open_button: &Button, 
    new_button: &Button, 
    save_as_button: &Button,
    editor_notebook: Option<&gtk4::Notebook>
) {
    // Create a key event controller
    let key_controller = EventControllerKey::new();
    
    // Clone button references for use in the closure
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    let open_button_clone = open_button.clone();
    let new_button_clone = new_button.clone();
    let window_clone = window.clone();
    
    // Clone notebook for use in the closure
    let editor_notebook_clone = editor_notebook.cloned();
    
    // Connect the key pressed event
    key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
        // Check modifier keys
        let ctrl_pressed = state.contains(gdk::ModifierType::CONTROL_MASK);
        let shift_pressed = state.contains(gdk::ModifierType::SHIFT_MASK);
        let alt_pressed = state.contains(gdk::ModifierType::ALT_MASK);
        
        // Handle keyboard shortcuts with Ctrl modifier
        if ctrl_pressed && !alt_pressed {
            match keyval.name().as_deref() {
                // File operations
                // Ctrl+S: Save
                Some("s") => {
                    if !shift_pressed {
                        save_button_clone.emit_clicked();
                        println!("Keyboard shortcut: Ctrl+S (Save)");
                        return glib::Propagation::Stop; // Event handled
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+Shift+S: Save As
                Some("S") | Some("s") if shift_pressed => {
                    save_as_button_clone.emit_clicked();
                    println!("Keyboard shortcut: Ctrl+Shift+S (Save As)");
                    return glib::Propagation::Stop;
                },
                // Ctrl+O: Open
                Some("o") => {
                    if !shift_pressed {
                        open_button_clone.emit_clicked();
                        println!("Keyboard shortcut: Ctrl+O (Open)");
                        return glib::Propagation::Stop;
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+N: New file
                Some("n") => {
                    if !shift_pressed {
                        new_button_clone.emit_clicked();
                        println!("Keyboard shortcut: Ctrl+N (New File)");
                        return glib::Propagation::Stop;
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+Q or Ctrl+W: Quit/Close
                Some("q") | Some("w") => {
                    // For Ctrl+Q, close the entire application
                    if keyval.name().as_deref() == Some("q") {
                        println!("Keyboard shortcut: Ctrl+Q (Quit)");
                        window_clone.close();
                        return glib::Propagation::Stop;
                    }
                    // For Ctrl+W, we could close the current tab (not implemented here)
                    if keyval.name().as_deref() == Some("w") {
                        println!("Keyboard shortcut: Ctrl+W (Close Tab) - Not implemented yet");
                        // Future implementation could close the current tab
                        return glib::Propagation::Proceed;
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+Tab: Next tab
                Some("Tab") => {
                    if let Some(notebook) = &editor_notebook_clone {
                        if let Some(current_page) = notebook.current_page() {
                            let page_count = notebook.n_pages();
                            if page_count > 0 {
                                let next_page = if shift_pressed {
                                    // Ctrl+Shift+Tab: Go to previous tab
                                    if current_page == 0 { page_count - 1 } else { current_page - 1 }
                                } else {
                                    // Ctrl+Tab: Go to next tab
                                    (current_page + 1) % page_count
                                };
                                notebook.set_current_page(Some(next_page));
                                println!("Keyboard shortcut: Ctrl+{}Tab (Switch Tab)",
                                    if shift_pressed { "Shift+" } else { "" });
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+PageDown: Next tab
                Some("Page_Down") => {
                    if let Some(notebook) = &editor_notebook_clone {
                        if let Some(current_page) = notebook.current_page() {
                            let page_count = notebook.n_pages();
                            if page_count > 0 {
                                let next_page = (current_page + 1) % page_count;
                                notebook.set_current_page(Some(next_page));
                                println!("Keyboard shortcut: Ctrl+PageDown (Next Tab)");
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+PageUp: Previous tab
                Some("Page_Up") => {
                    if let Some(notebook) = &editor_notebook_clone {
                        if let Some(current_page) = notebook.current_page() {
                            let page_count = notebook.n_pages();
                            if page_count > 0 {
                                let prev_page = if current_page == 0 { page_count - 1 } else { current_page - 1 };
                                notebook.set_current_page(Some(prev_page));
                                println!("Keyboard shortcut: Ctrl+PageUp (Previous Tab)");
                                return glib::Propagation::Stop;
                            }
                        }
                    }
                    return glib::Propagation::Proceed;
                },
                // Ctrl+F: Find - Placeholder for future implementation
                Some("f") => {
                    println!("Keyboard shortcut: Ctrl+F (Find) - Not implemented yet");
                    // Implementation of Find functionality could be added here
                    return glib::Propagation::Proceed;
                },
                // Ctrl+H: Replace - Placeholder for future implementation
                Some("h") => {
                    println!("Keyboard shortcut: Ctrl+H (Replace) - Not implemented yet");
                    // Implementation of Replace functionality could be added here
                    return glib::Propagation::Proceed;
                },
                // Ctrl+Z: Undo - Managed by GtkTextView but log for debugging
                Some("z") => {
                    println!("Keyboard shortcut: Ctrl+Z (Undo) - Handled by GtkTextView");
                    return glib::Propagation::Proceed; // Let GtkTextView handle it
                },
                // Ctrl+Y/Ctrl+Shift+Z: Redo - Managed by GtkTextView but log for debugging
                Some("y") | Some("Z") => {
                    println!("Keyboard shortcut: Ctrl+{} (Redo) - Handled by GtkTextView", 
                             if keyval.name().as_deref() == Some("y") { "Y" } else { "Shift+Z" });
                    return glib::Propagation::Proceed; // Let GtkTextView handle it
                },
                // Let other Ctrl shortcuts pass through to the editor (like Ctrl+C, Ctrl+V)
                _ => {}
            }
        }
        
        // Let the event propagate to other handlers (like the text editor's built-in shortcuts)
        glib::Propagation::Proceed
    });
    
    // Add the controller to the window
    window.add_controller(key_controller);
    
    // Log that keyboard shortcuts have been set up
    println!("Keyboard shortcuts initialized:");
    println!("  - Ctrl+S: Save");
    println!("  - Ctrl+Shift+S: Save As");
    println!("  - Ctrl+O: Open");
    println!("  - Ctrl+N: New file");
    println!("  - Ctrl+Q: Quit application");
    println!("  - Ctrl+Tab/Ctrl+Shift+Tab: Switch between tabs");
    println!("  - Ctrl+PageDown/Ctrl+PageUp: Navigate between tabs");
    println!("  - Other standard shortcuts (Ctrl+C, Ctrl+V, etc.) handled by GTK");
}
