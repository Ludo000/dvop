// Utility functions for the Basado Text Editor
// This module contains helper functions used throughout the application

use gtk4::prelude::*;
use gtk4::{Button, ListBox, MenuButton, pango};
use std::path::PathBuf;
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
