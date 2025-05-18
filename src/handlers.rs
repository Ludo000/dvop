// Event handlers and business logic for the Basado Text Editor
// This module contains all the event handlers and core functionality for the editor

// GTK imports
use gtk4::prelude::*;
use gtk4::{
    // Widgets
    Button, TextBuffer, ApplicationWindow, ListBox, ScrolledWindow, 
    TextView, Label, Picture, Notebook, MenuButton,
    
    // Dialog components
    MessageDialog, DialogFlags, MessageType, ButtonsType, ResponseType
};

// Standard library imports
use std::collections::HashMap;  // For mapping tab indices to file paths
use std::rc::Rc;                // Reference counting for shared ownership
use std::cell::RefCell;         // Interior mutability pattern
use std::path::PathBuf;         // File system path representation
use std::fs::File;              // File operations
use std::io::Write;             // File writing capabilities

// Internal imports
use crate::utils;               // Utility functions

/// Gets the TextView and TextBuffer from the currently active notebook tab
///
/// This function navigates the widget hierarchy to find the text view in the current tab.
/// Returns None if there is no active tab or if the tab doesn't contain a text view
/// (e.g., if it's showing an image instead).
pub fn get_active_text_view_and_buffer(notebook: &Notebook) -> Option<(TextView, TextBuffer)> {
    // Get the current page number, then use it to find the page widget
    notebook.current_page().and_then(|page_num| {
        notebook.nth_page(Some(page_num)).and_then(|page_widget| {
            // Check if the page contains a ScrolledWindow (typical for text content)
            if let Some(scrolled_window) = page_widget.downcast_ref::<ScrolledWindow>() {
                // Get the child of the ScrolledWindow
                scrolled_window.child().and_then(|child| {
                    // Try to cast the child to a TextView
                    if let Some(text_view) = child.downcast_ref::<TextView>() {
                        // Return the TextView and its associated TextBuffer
                        Some((text_view.clone(), text_view.buffer()))
                    } else {
                        // Child exists but is not a TextView
                        None
                    }
                })
            } else {
                // Page widget is not a ScrolledWindow
                // This happens for non-text content like images
                None
            }
        })
    })
}

/// Gets the TextView and TextBuffer for a specific notebook tab by index
///
/// Similar to get_active_text_view_and_buffer, but works with an explicit page number
/// instead of the currently active tab.
pub fn get_text_view_and_buffer_for_page(notebook: &Notebook, page_num: u32) -> Option<(TextView, TextBuffer)> {
    // Get the page widget for the specified page number
    notebook.nth_page(Some(page_num)).and_then(|page_widget| {
        // Check if the page contains a ScrolledWindow
        if let Some(scrolled_window) = page_widget.downcast_ref::<ScrolledWindow>() {
            // Get the child of the ScrolledWindow
            scrolled_window.child().and_then(|child| {
                // Try to cast the child to a TextView
                if let Some(text_view) = child.downcast_ref::<TextView>() {
                    // Return the TextView and its associated TextBuffer
                    Some((text_view.clone(), text_view.buffer()))
                } else {
                    // Child exists but is not a TextView
                    None
                }
            })
        } else {
            // Page widget is not a ScrolledWindow
            None
        }
    })
}


/// Structure containing all dependencies needed for tab creation and management
///
/// This structure holds references to all the components and state that need
/// to be modified when creating, switching, or closing tabs. It makes it easier
/// to pass these references to various tab-related functions.
#[derive(Clone)]
pub struct NewTabDependencies {
    // Core UI components
    pub editor_notebook: Notebook,              // The tabbed container
    pub window: ApplicationWindow,              // Main window (for dialog parents)
    pub file_list_box: ListBox,                 // File browser list
    
    // State tracking
    pub active_tab_path: Rc<RefCell<Option<PathBuf>>>,       // Currently active file path
    pub file_path_manager: Rc<RefCell<HashMap<u32, PathBuf>>>, // Maps tab indices to file paths
    pub current_dir: Rc<RefCell<PathBuf>>,                   // Current working directory
    
    // Action buttons
    pub save_button: Button,                    // Save button
    pub save_as_button: Button,                 // Save As button
    pub _save_menu_button: Option<MenuButton>,  // Split button menu component (unused but kept for future)
}

/// Creates a new empty tab with the title "Untitled"
///
/// This function is used to create a new tab for a new document,
/// setting up all the necessary UI components and state tracking.
fn create_new_empty_tab(deps: &NewTabDependencies) {
    // Create a new source view with syntax highlighting capabilities
    let (source_view, source_buffer) = crate::syntax::create_source_view();
    source_buffer.set_text(""); // Start with empty content
    
    // Get TextView and TextBuffer interfaces for compatibility with the rest of the code
    // Clone source_view to avoid ownership move
    let _new_text_view = source_view.clone().upcast::<TextView>();
    let new_text_buffer = source_buffer.upcast::<TextBuffer>();
    
    // Place the source view in a scrollable container
    let new_scrolled_window = crate::syntax::create_source_view_scrolled(&source_view);
    
    // Create a custom tab widget with label and close button
    let (tab_widget, tab_actual_label, tab_close_button) = crate::ui::create_tab_widget("Untitled");
    
    // Add the new tab to the notebook and switch to it
    let new_page_num = deps.editor_notebook.append_page(&new_scrolled_window, Some(&tab_widget));
    // Setting current page after append ensures the switch_page signal is emitted properly
    deps.editor_notebook.set_current_page(Some(new_page_num));
    
    // Update the active tab path to None (unsaved document)
    *deps.active_tab_path.borrow_mut() = None;
    
    // Note: We don't update file_path_manager for "Untitled" tabs until they're saved
    
    // Clone the data to release borrows before updating the UI
    // This prevents potential borrow conflicts in signal handlers
    let current_dir_path_clone = deps.current_dir.borrow().clone();
    let active_path_for_update = deps.active_tab_path.borrow().clone(); // Will be None here
    
    // Update the file browser to reflect the current state
    utils::update_file_list(&deps.file_list_box, &current_dir_path_clone, &active_path_for_update);
    
    // Enable save buttons appropriate for plain text content
    utils::update_save_buttons_visibility(
        &deps.save_button, 
        &deps.save_as_button, 
        Some(mime_guess::mime::TEXT_PLAIN_UTF_8)
    );
    
    // Also update the split button menu visibility if present
    if let Some(ref save_menu_button) = deps._save_menu_button {
        utils::update_save_menu_button_visibility(
            save_menu_button, 
            Some(mime_guess::mime::TEXT_PLAIN_UTF_8)
        );
    }

    // Connect dirty tracking for the new "Untitled" tab's label
    let tab_actual_label_clone = tab_actual_label.clone();
    new_text_buffer.connect_changed(move |buffer| {
        let label_text = tab_actual_label_clone.text();
        if label_text == "Untitled" && !buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).is_empty() {
            tab_actual_label_clone.set_text("Untitled*");
        } else if label_text == "Untitled*" && buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).is_empty() {
            tab_actual_label_clone.set_text("Untitled");
        }
    });

    // Connect close button for this new tab
    let deps_clone_for_close = deps.clone();
    tab_close_button.connect_clicked(move |_| {
        // Find the current page number of this tab, as it might have shifted
        if let Some(page_widget_of_tab) = new_scrolled_window.parent() { // This gets the GtkBox (tab_widget)
             if let Some(parent_notebook) = page_widget_of_tab.parent().and_then(|p| p.downcast::<Notebook>().ok()) { // This should be editor_notebook
                if let Some(current_idx_for_this_tab) = parent_notebook.page_num(&page_widget_of_tab.parent().unwrap()) { // page_num needs the child of notebook
                     handle_close_tab_request(
                        &deps_clone_for_close.editor_notebook,
                        current_idx_for_this_tab,
                        &deps_clone_for_close.window,
                        &deps_clone_for_close.file_path_manager,
                        &deps_clone_for_close.active_tab_path,
                        &deps_clone_for_close.current_dir, // New
                        &deps_clone_for_close.file_list_box, // New
                        Some(deps_clone_for_close.clone())
                    );
                }
             }
        }
    });
}

// Helper function to update tab label after save or name change
pub fn update_tab_label_after_save(notebook: &Notebook, page_num: u32, new_name_opt: Option<&str>, is_now_dirty: bool) {
    if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
        if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
            if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                    let base_name = new_name_opt.map(String::from)
                        .unwrap_or_else(|| label.text().trim_end_matches('*').to_string());
                    
                    let mut final_text = base_name;
                    if is_now_dirty {
                        if !final_text.ends_with('*') {
                            final_text.push('*');
                        }
                    }
                    // Ensure no double asterisks if base_name somehow had one and is_now_dirty is true
                    if final_text.ends_with("**") {
                        final_text.pop();
                    }
                    label.set_text(&final_text);
                }
            }
        }
    }
}


pub fn handle_close_tab_request(
    notebook: &Notebook,
    page_num_to_close: u32,
    window: &ApplicationWindow,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>, // New
    file_list_box: &ListBox,            // New
    new_tab_deps: Option<NewTabDependencies>, // Dependencies to create a new tab if the last one is closed
) {
    if let Some(page_widget) = notebook.nth_page(Some(page_num_to_close)) {
        if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
            let mut is_dirty = false;
            if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                    if label.text().ends_with('*') {
                        is_dirty = true;
                    }
                }
            }

            if !is_dirty {
                // Not dirty, close directly
                actually_close_tab(notebook, page_num_to_close, file_path_manager, active_tab_path, new_tab_deps.as_ref());
                return;
            }

            // Is dirty, show confirmation dialog
            let filename_str = file_path_manager.borrow().get(&page_num_to_close)
                .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                .unwrap_or_else(|| "Untitled".to_string());
            let dialog = MessageDialog::new(
                Some(window),
                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                MessageType::Question,
                ButtonsType::None,
                &format!("Save changes to {} before closing?", filename_str) // Corrected format string: removed quotes around {}
            );
            dialog.add_buttons(&[
                ("Save", ResponseType::Yes),
                ("Don't Save", ResponseType::No),
                ("Cancel", ResponseType::Cancel),
            ]);

            let notebook_clone = notebook.clone();
            let file_path_manager_clone = file_path_manager.clone();
            let active_tab_path_clone = active_tab_path.clone();
            let new_tab_deps_clone = new_tab_deps.clone();
            let window_clone = window.clone();
            let current_dir_clone = current_dir.clone();
            let file_list_box_clone = file_list_box.clone();

            dialog.connect_response(move |d, response| {
                match response {
                    ResponseType::Yes => {
                        // User chose "Save"
                        if let Some((_tv, buffer)) = get_text_view_and_buffer_for_page(&notebook_clone, page_num_to_close) {
                            let path_opt = file_path_manager_clone.borrow().get(&page_num_to_close).cloned();
                            if let Some(path) = path_opt { // Existing file
                                let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                                match File::create(&path) {
                                    Ok(mut file) => {
                                        if file.write_all(text.as_bytes()).is_ok() {
                                            update_tab_label_after_save(&notebook_clone, page_num_to_close, Some(&path.file_name().unwrap_or_default().to_string_lossy()), false);
                                            actually_close_tab(&notebook_clone, page_num_to_close, &file_path_manager_clone, &active_tab_path_clone, new_tab_deps_clone.as_ref());
                                        } else {
                                            eprintln!("Error writing to file: {:?}", path);
                                            // Optionally show error dialog to user
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Error creating file for writing: {:?}, error: {}", path, e);
                                        // Optionally show error dialog
                                    }
                                }
                            } else { // Untitled file, need to "Save As"
                                let save_as_dialog = gtk4::FileChooserDialog::new(
                                    Some("Save File As"), Some(&window_clone), gtk4::FileChooserAction::Save,
                                    &[("Save", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)]);
                                
                                let current_dialog_dir_path = current_dir_clone.borrow().clone();
                                
                                // Explicitly type annotation for gio_file_result and wrap the call in Ok()
                                let gio_file_result: Result<gtk4::gio::File, glib::Error> = Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
                                match gio_file_result {
                                    Ok(gfile) => {
                                        if current_dialog_dir_path.is_dir() {
                                            let _ = save_as_dialog.set_current_folder(Some(&gfile));
                                        } else if let Some(parent_gfile) = gfile.parent() {
                                            let _ = save_as_dialog.set_current_folder(Some(&parent_gfile));
                                        }
                                    }
                                    Err(e) => { 
                                        eprintln!("Failed to create GFile for path {:?}: {}", current_dialog_dir_path, e);
                                    }
                                }

                                save_as_dialog.set_current_name("Untitled.txt");

                                let buffer_clone_for_save_as = buffer.clone();
                                let nc_save_as = notebook_clone.clone();
                                let fpm_save_as = file_path_manager_clone.clone();
                                let atp_save_as = active_tab_path_clone.clone();
                                let ntd_save_as = new_tab_deps_clone.clone(); // For actually_close_tab
                                let cd_save_as = current_dir_clone.clone();
                                let flb_save_as = file_list_box_clone.clone();

                                save_as_dialog.connect_response(move |d_sa, resp_sa| {
                                    if resp_sa == gtk4::ResponseType::Accept {
                                        if let Some(file_to_save) = d_sa.file().and_then(|f| f.path()) {
                                            let text_to_save = buffer_clone_for_save_as.text(&buffer_clone_for_save_as.start_iter(), &buffer_clone_for_save_as.end_iter(), false);
                                            match File::create(&file_to_save) {
                                                Ok(mut f_obj) => {
                                                    if f_obj.write_all(text_to_save.as_bytes()).is_ok() {
                                                        fpm_save_as.borrow_mut().insert(page_num_to_close, file_to_save.clone());
                                                        if nc_save_as.current_page() == Some(page_num_to_close) {
                                                            *atp_save_as.borrow_mut() = Some(file_to_save.clone());
                                                        }
                                                        update_tab_label_after_save(&nc_save_as, page_num_to_close, Some(&file_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                                                        if let Some(parent) = file_to_save.parent() {
                                                            *cd_save_as.borrow_mut() = parent.to_path_buf();
                                                        }
                                                        utils::update_file_list(&flb_save_as, &cd_save_as.borrow(), &atp_save_as.borrow());
                                                        actually_close_tab(&nc_save_as, page_num_to_close, &fpm_save_as, &atp_save_as, ntd_save_as.as_ref());
                                                    } else { eprintln!("Error writing to new file: {:?}", file_to_save); }
                                                }
                                                Err(e) => { eprintln!("Error creating new file: {:?}, error: {}", file_to_save, e); }
                                            }
                                        }
                                    }
                                    d_sa.close(); // Close the "Save As" dialog
                                });
                                save_as_dialog.show();
                            }
                        }
                        d.close(); // Close the "Save changes?" dialog
                    }
                    ResponseType::No => {
                        d.close(); // Close the "Save changes?" dialog
                        actually_close_tab(&notebook_clone, page_num_to_close, &file_path_manager_clone, &active_tab_path_clone, new_tab_deps_clone.as_ref());
                    }
                    ResponseType::Cancel | _ => {
                        d.close(); // Close the "Save changes?" dialog
                        // Do nothing else, tab remains open
                    }
                }
            });
            dialog.show();
            // No direct close action here; dialog responses handle it.
        }
    }
}

// Helper function to perform the actual tab closing and state update
fn actually_close_tab(
    notebook: &Notebook,
    page_num_to_close: u32,
    file_path_manager_rc: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path_rc: &Rc<RefCell<Option<PathBuf>>>,
    new_tab_deps: Option<&NewTabDependencies>,
) {
    notebook.remove_page(Some(page_num_to_close));
    
    { // Scope for mutable borrow of file_path_manager
        let mut manager = file_path_manager_rc.borrow_mut();
        manager.remove(&page_num_to_close); // Remove the closed tab's path

        // Collect paths from higher indices that need to be shifted
        let mut paths_to_shift = Vec::new();
        // Determine the range of indices to check for shifting.
        // Need to be careful if manager is empty or only had one element.
        let current_max_idx = manager.keys().max().cloned();

        if let Some(max_idx) = current_max_idx {
            for i in (page_num_to_close + 1)..=(max_idx + 1) { // Iterate up to one beyond max existing index to catch all
                                                              // This loop structure might be problematic if page_num_to_close was the max_idx
                if let Some(path) = manager.remove(&i) {
                    paths_to_shift.push(path); // Store path to be re-inserted at i-1
                }
            }
        }
        
        // Re-insert paths at their new, shifted indices
        for (idx_offset, path) in paths_to_shift.into_iter().enumerate() {
            manager.insert(page_num_to_close + idx_offset as u32, path);
        }
    } // Mutable borrow of file_path_manager_rc is dropped here

    if notebook.n_pages() == 0 {
        // No pages left, active_tab_path should be None.
        *active_tab_path_rc.borrow_mut() = None;
        if let Some(deps) = new_tab_deps {
            // It's now safe to call create_new_empty_tab as the mutable borrow 
            // on file_path_manager_rc has been released.
            create_new_empty_tab(deps);
        }
    } else {
        // If other tabs remain, GTK will automatically switch to a new page (e.g., the one at page_num_to_close, or page 0).
        // The connect_switch_page handler in main.rs is responsible for updating active_tab_path.
        // We need to ensure that file_path_manager contains the correct path for the new current page.
        // The re-indexing above should have handled this.
        // If the active tab was closed, switch_page will fire. If a different tab was closed, 
        // the current page might not change, but its index in file_path_manager might be wrong if it was after the closed tab.
        // However, the switch_page handler uses the *new* page_num provided by the signal, which should be correct.
    }
}


// Helper function to open a file in a new tab or focus if already open
fn open_or_focus_tab(
    notebook: &Notebook,
    file_to_open: &PathBuf,
    content: &str,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    save_button: &Button,
    save_as_button: &Button, 
    _mime_type: &mime_guess::Mime, // Used now for save menu button visibility
    window: &ApplicationWindow, // Added for dialogs and NewTabDependencies
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    _save_menu_button: Option<&MenuButton>, // Added save_menu_button parameter
) {
    // Check if file is already open
    let mut page_to_focus = None;
    let num_pages = notebook.n_pages();
    for i in 0..num_pages {
        if let Some(path) = file_path_manager.borrow().get(&i) {
            if path == file_to_open {
                page_to_focus = Some(i);
                break;
            }
        }
    }

    if let Some(page_num) = page_to_focus {
        notebook.set_current_page(Some(page_num));
        *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());
    } else {
        // Get file MIME type 
        let mime_type = mime_guess::from_path(&file_to_open).first_or_octet_stream();
        let file_name = file_to_open.file_name().unwrap_or_default().to_string_lossy().to_string();
        
        // Create tab widget regardless of content type
        let (tab_widget, tab_actual_label, tab_close_button) = crate::ui::create_tab_widget(&file_name);
        
        let new_scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();
            
        // Handle different file types
        if mime_type.type_() == "image" {
            // Handle image file
            if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&file_to_open) {
                let picture = Picture::new();
                picture.set_pixbuf(Some(&pixbuf));
                new_scrolled_window.set_child(Some(&picture));
            } else {
                // Failed to load image, show error
                let error_msg = format!("Failed to load image: {}", file_name);
                let error_label = Label::new(Some(&error_msg));
                new_scrolled_window.set_child(Some(&error_label));
            }
        } else if utils::is_allowed_mime_type(&mime_type) {
            // Handle text file
            // Create source view with syntax highlighting
            let (source_view, source_buffer) = crate::syntax::create_source_view();
            source_buffer.set_text(content);
            
            // Apply syntax highlighting based on file extension
            crate::syntax::set_language_for_file(&source_buffer, file_to_open);
            
            // Get TextBuffer interfaces for compatibility with the rest of the code
            // Clone source_view to avoid ownership move
            let new_text_buffer = source_buffer.upcast::<TextBuffer>();
            
            // Set the source view as the child of the scrolled window
            new_scrolled_window.set_child(Some(&source_view));

            // Dirty tracking
            let tab_actual_label_clone = tab_actual_label.clone();
            let file_name_clone = file_name.clone();
            new_text_buffer.connect_changed(move |_buffer| { 
                if !tab_actual_label_clone.text().ends_with("*") {
                     tab_actual_label_clone.set_text(&format!("{}*", file_name_clone));
                }
            });
        } else {
            // Unsupported file type
            let error_msg = format!("Unsupported file type: {}", file_name);
            let error_label = Label::new(Some(&error_msg));
            new_scrolled_window.set_child(Some(&error_label));
        }

        // Add the new tab to the notebook and make it the current page
        let new_page_num = notebook.append_page(&new_scrolled_window, Some(&tab_widget));
        notebook.set_current_page(Some(new_page_num));

        // Update state
        file_path_manager.borrow_mut().insert(new_page_num, file_to_open.clone());
        *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());

        // Connect close button
        let notebook_clone = notebook.clone();
        let window_clone = window.clone();
        let file_path_manager_clone = file_path_manager.clone();
        let active_tab_path_ref_clone = active_tab_path_ref.clone();
        
        let deps_for_new_tab_creation = NewTabDependencies {
            editor_notebook: notebook.clone(),
            active_tab_path: active_tab_path_ref_clone.clone(),
            file_path_manager: file_path_manager_clone.clone(),
            window: window_clone.clone(),
            file_list_box: file_list_box.clone(),
            current_dir: current_dir.clone(),
            save_button: save_button.clone(),
            save_as_button: save_as_button.clone(),
            _save_menu_button: _save_menu_button.map(|btn| btn.clone()), // Pass the save menu button if available
        };

        tab_close_button.connect_clicked(move |_| {
            // Need to find the current page number of this tab when button is clicked
            // The new_page_num captured at creation might be stale if other tabs were manipulated.
            // Find the page by its child (new_scrolled_window)
            if let Some(current_idx_for_this_tab) = notebook_clone.page_num(&new_scrolled_window) {
                handle_close_tab_request(
                    &notebook_clone,
                    current_idx_for_this_tab,
                    &window_clone,
                    &file_path_manager_clone,
                    &active_tab_path_ref_clone,
                    &deps_for_new_tab_creation.current_dir, // New
                    &deps_for_new_tab_creation.file_list_box, // New
                    Some(deps_for_new_tab_creation.clone())
                );
            }
        });
        
        // Update save buttons visibility based on mime type
        utils::update_save_buttons_visibility(save_button, save_as_button, Some(mime_type.clone()));
        
        // Also update the save menu button if available
        if let Some(save_menu_btn) = _save_menu_button {
            utils::update_save_menu_button_visibility(save_menu_btn, Some(mime_type));
        }
    }
}
pub fn setup_button_handlers(
    new_button: &Button,
    open_button: &Button,
    save_button: &Button,
    save_as_button: &Button,
    _initial_text_buffer: &TextBuffer, 
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow, // Already present, good.
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    editor_notebook: &Notebook, 
    error_label: &Label,
    picture: &Picture, 
    up_button: &Button,
    refresh_button: &Button,
    file_list_box_clone: &ListBox, // This is likely the same as file_list_box, ensure it's used consistently
    _save_menu_button: Option<&MenuButton> // Prefix with underscore to acknowledge it's unused
) {
    setup_new_button_handler(
        new_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        file_list_box, // Pass the main file_list_box
        current_dir,
        save_button,
        save_as_button,
        window, // Pass window
    );

    setup_open_button_handler(
        open_button,
        editor_notebook,
        window, // Already passed
        current_dir,
        file_list_box, // Pass the main file_list_box
        error_label,
        picture, 
        save_button,
        save_as_button,
        active_tab_path,
        file_path_manager,
        _save_menu_button,
    );

    setup_save_button_handler(
        save_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        window,
        file_list_box,
        current_dir,
    );

    setup_save_as_button_handler(
        save_as_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        window,
        current_dir,
        file_list_box,
    );

    setup_file_selection_handler(
        file_list_box_clone, // Ensure this is the intended ListBox instance
        editor_notebook,
        active_tab_path,
        file_path_manager,
        current_dir,
        error_label,
        picture, 
        save_button,
        save_as_button,
        window, // Pass window
        _save_menu_button, // Pass save_menu_button with the renamed parameter
    );

    // These handlers likely don't need direct access to the editor_notebook content itself
    // but might influence which file is considered "active" if that logic is centralized.
    setup_up_button_handler(up_button, current_dir, file_list_box, active_tab_path); // Pass active_tab_path
    setup_refresh_button_handler(refresh_button, file_list_box, current_dir, active_tab_path); // Pass active_tab_path
}

fn setup_new_button_handler(
    new_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    file_list_box: &ListBox, // To update file list selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list
    save_button: &Button,
    save_as_button: &Button,
    window: &ApplicationWindow, // Added for NewTabDependencies
) {
    let editor_notebook_clone = editor_notebook.clone(); // Clone for the main closure
    let active_tab_path_ref_clone = active_tab_path_ref.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let file_list_box_clone = file_list_box.clone();
    let current_dir_clone = current_dir.clone();
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    let window_clone = window.clone();


    new_button.connect_clicked(move |_| {
        let save_as_button_for_new_tab = save_as_button_clone.clone(); // Clone for this specific closure
        let new_text_view = TextView::new();
        let new_text_buffer = new_text_view.buffer();
        new_text_buffer.set_text(""); // Empty content for new file

        let new_scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&new_text_view)
            .build();

        // Use ui::create_tab_widget
        let (tab_widget, tab_actual_label, tab_close_button) = crate::ui::create_tab_widget("Untitled");

        let new_page_num = editor_notebook_clone.append_page(&new_scrolled_window, Some(&tab_widget));
        editor_notebook_clone.set_current_page(Some(new_page_num));

        *active_tab_path_ref_clone.borrow_mut() = None; // No path for new, unsaved file
        // file_path_manager.borrow_mut().insert(new_page_num, PathBuf::new()); // Or some placeholder for untitled

        // Update file list (optional, as no file is selected)
        utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_clone.borrow());
        utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_guess::mime::TEXT_PLAIN_UTF_8));

        // Connect changed signal for dirty tracking
        let tab_actual_label_clone = tab_actual_label.clone();
        let new_text_buffer_clone_for_dirty = new_text_buffer.clone();
        new_text_buffer.connect_changed(move |_buffer| {
            let label_text = tab_actual_label_clone.text();
            if label_text == "Untitled" && !new_text_buffer_clone_for_dirty.text(&new_text_buffer_clone_for_dirty.start_iter(), &new_text_buffer_clone_for_dirty.end_iter(), false).is_empty() {
                 tab_actual_label_clone.set_text("Untitled*");
            } else if label_text == "Untitled*" && new_text_buffer_clone_for_dirty.text(&new_text_buffer_clone_for_dirty.start_iter(), &new_text_buffer_clone_for_dirty.end_iter(), false).is_empty() {
                tab_actual_label_clone.set_text("Untitled");
            }
        });

        // Connect close button
        let notebook_for_close = editor_notebook_clone.clone();
        let window_for_close = window_clone.clone(); // window_clone is from the outer scope
        let file_path_manager_for_close = file_path_manager_clone.clone();
        let active_tab_path_for_close = active_tab_path_ref_clone.clone();
        
        let deps_for_new_tab_creation = NewTabDependencies {
            editor_notebook: editor_notebook_clone.clone(), // Use the notebook_clone from this scope
            active_tab_path: active_tab_path_ref_clone.clone(),
            file_path_manager: file_path_manager_clone.clone(),
            window: window_clone.clone(), // Pass the window for dialogs
            file_list_box: file_list_box_clone.clone(),
            current_dir: current_dir_clone.clone(),
            save_button: save_button_clone.clone(), // Clone for NewTabDependencies
            save_as_button: save_as_button_for_new_tab, // Use the clone specific to this closure
            _save_menu_button: None, // We don't have a menu button in this scope
        };
        let new_scrolled_window_clone_for_close = new_scrolled_window.clone();
        tab_close_button.connect_clicked(move |_| {
            if let Some(current_idx_for_this_tab) = notebook_for_close.page_num(&new_scrolled_window_clone_for_close) {
                handle_close_tab_request(
                    &notebook_for_close,
                    current_idx_for_this_tab,
                    &window_for_close,
                    &file_path_manager_for_close,
                    &active_tab_path_for_close,
                    &deps_for_new_tab_creation.current_dir, // New
                    &deps_for_new_tab_creation.file_list_box, // New
                    Some(deps_for_new_tab_creation.clone())
                );
            }
        });
    });
}

fn setup_open_button_handler(
    open_button: &Button,
    editor_notebook: &Notebook,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    error_label: &Label, // For showing errors if a tab can't display content
    picture: &Picture,   // For image files - this needs to be rethought for tabs
    save_button: &Button,
    save_as_button: &Button,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    _save_menu_button: Option<&MenuButton>, // Renamed with underscore to acknowledge it's unused
) {
    let editor_notebook = editor_notebook.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();
    let error_label = error_label.clone();
    let picture = picture.clone();
    let save_button = save_button.clone();
    let save_as_button = save_as_button.clone();
    // Clone the Rc itself, not the reference, to move ownership into the closure
    let active_tab_path_ref_owned = active_tab_path_ref.clone();
    let file_path_manager_owned = file_path_manager.clone();
    // Clone the save_menu_button (renamed to match the parameter name)
    let __save_menu_button = _save_menu_button.cloned(); // Double underscore to avoid confusion with parameter name

    open_button.connect_clicked(move |_| {
        let dialog = gtk4::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk4::FileChooserAction::Open,
            &[("Open", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
        );

        let current_dialog_dir_path = current_dir.borrow().clone();
        // Explicitly type annotation for gio_file_result and wrap the call in Ok()
        let gio_file_result: Result<gtk4::gio::File, glib::Error> = Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
        match gio_file_result {
            Ok(gfile) => {
                if current_dialog_dir_path.is_dir() {
                    let _ = dialog.set_current_folder(Some(&gfile));
                } else if let Some(parent_gfile) = gfile.parent() {
                    let _ = dialog.set_current_folder(Some(&parent_gfile));
                }
            }
            Err(e) => { 
                eprintln!("Failed to create GFile for path {:?}: {}", current_dialog_dir_path, e);
            }
        }

        let editor_notebook_clone = editor_notebook.clone();
        let current_dir_clone = current_dir.clone();
        let file_list_box_clone = file_list_box.clone();
        let error_label_clone = error_label.clone();
        let picture_clone = picture.clone();
        let save_button_clone = save_button.clone();
        let save_as_button_clone = save_as_button.clone();
        // Use the owned Rcs for the nested closure
        let active_tab_path_ref_for_response = active_tab_path_ref_owned.clone();
        let file_path_manager_for_response = file_path_manager_owned.clone();
        // Need window, file_list_box, current_dir for open_or_focus_tab's NewTabDependencies
        let window_for_response = window.clone();
        let file_list_box_for_response = file_list_box.clone();
        let current_dir_for_response = current_dir.clone();
        let save_menu_button_for_response = __save_menu_button.clone(); // Clone before the inner closure


        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file_to_open) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file_to_open).first_or_octet_stream();
                    if utils::is_allowed_mime_type(&mime_type) {
                        if let Ok(content) = std::fs::read_to_string(&file_to_open) {                        open_or_focus_tab(
                            &editor_notebook_clone,
                            &file_to_open,
                            &content,
                            &active_tab_path_ref_for_response, 
                            &file_path_manager_for_response,   
                            &save_button_clone,
                            &save_as_button_clone,
                            &mime_type.clone(), // Clone here to avoid ownership move
                            &window_for_response, // Pass window
                            &file_list_box_for_response, // Pass file_list_box
                            &current_dir_for_response, // Pass current_dir
                            save_menu_button_for_response.as_ref(), // Pass the save_menu_button
                        );

                            if let Some(parent) = file_to_open.parent() {
                                *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_for_response.borrow());
                            }
                        }
                    } else if mime_type.type_() == "image" {
                        // For images, use open_or_focus_tab with empty content
                        open_or_focus_tab(
                            &editor_notebook_clone,
                            &file_to_open,
                            "", // Empty content for images
                            &active_tab_path_ref_for_response,
                            &file_path_manager_for_response,
                            &save_button_clone,
                            &save_as_button_clone,
                            &mime_type,
                            &window_for_response,
                            &file_list_box_for_response,
                            &current_dir_for_response,
                            save_menu_button_for_response.as_ref(),
                        );

                        if let Some(parent) = file_to_open.parent() {
                            *current_dir_clone.borrow_mut() = parent.to_path_buf();
                            utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_for_response.borrow());
                        }
                    } else {
                        // Handle unsupported file types
                        open_or_focus_tab(
                            &editor_notebook_clone,
                            &file_to_open,
                            "", // Empty content for unsupported files
                            &active_tab_path_ref_for_response,
                            &file_path_manager_for_response,
                            &save_button_clone,
                            &save_as_button_clone,
                            &mime_type,
                            &window_for_response,
                            &file_list_box_for_response,
                            &current_dir_for_response,
                            save_menu_button_for_response.as_ref(),
                        );

                        if let Some(parent) = file_to_open.parent() {
                            *current_dir_clone.borrow_mut() = parent.to_path_buf();
                            utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_for_response.borrow());
                        }
                    }
                }
            }
            dialog.close();
        });
        dialog.show();
    });
}

fn setup_save_button_handler(
    save_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    window: &ApplicationWindow,
    file_list_box: &ListBox, // To update selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list path
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();

    save_button.connect_clicked(move |_| {
        if let Some((_active_text_view, active_buffer)) = get_active_text_view_and_buffer(&editor_notebook) { // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            let path_to_save_opt = file_path_manager.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                let mime_type = mime_guess::from_path(&path_to_save).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(mut file) = File::create(&path_to_save) {
                        let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                        if file.write_all(text.as_bytes()).is_ok() {
                            // Update tab label (remove *)
                            update_tab_label_after_save(&editor_notebook, current_page_num, Some(&path_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                        }
                    }
                }
            } else { // No path associated, treat as "Save As"
                // This logic should ideally call a shared "save_as" function
                let dialog = gtk4::FileChooserDialog::new(
                    Some("Save File"),
                    Some(&window),
                    gtk4::FileChooserAction::Save,
                    &[("Save", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
                );
                // ... (rest of save_as logic, simplified here) ...
                let editor_notebook_clone = editor_notebook.clone();
                let active_tab_path_ref_clone = active_tab_path_ref.clone();
                let file_path_manager_clone = file_path_manager.clone();
                let file_list_box_clone = file_list_box.clone();
                let current_dir_clone = current_dir.clone();

                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file().and_then(|f| f.path()) {
                             if let Ok(mut f_obj) = File::create(&file) {
                                let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                                if f_obj.write_all(text.as_bytes()).is_ok() {
                                    file_path_manager_clone.borrow_mut().insert(current_page_num, file.clone());
                                    *active_tab_path_ref_clone.borrow_mut() = Some(file.clone());
                                     // Update tab label
                                    update_tab_label_after_save(&editor_notebook_clone, current_page_num, Some(&file.file_name().unwrap_or_default().to_string_lossy()), false);
                                    // Update main window title potentially
                                    if let Some(parent) = file.parent() {
                                        *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                    }
                                    utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_clone.borrow());
                                }
                            }
                        }
                    }
                    d.close();
                });
                dialog.show();
            }
        }
    });
}

fn setup_save_as_button_handler(
    save_as_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>, // To set initial dialog directory and update after save
    file_list_box: &ListBox, // To update file list
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();

    save_as_button.connect_clicked(move |_| {
        if let Some((_active_text_view, active_buffer)) = get_active_text_view_and_buffer(&editor_notebook) { // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            let dialog = gtk4::FileChooserDialog::new(
                Some("Save File As"),
                Some(&window),
                gtk4::FileChooserAction::Save,
                &[("Save As", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
            );

            let current_dialog_dir_path = current_dir.borrow().clone();
            // Explicitly type annotation for gio_file_result and wrap the call in Ok()
            let gio_file_result: Result<gtk4::gio::File, glib::Error> = Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
            match gio_file_result {
                Ok(gfile) => {
                    if current_dialog_dir_path.is_dir() {
                        let _ = dialog.set_current_folder(Some(&gfile));
                    } else if let Some(parent_gfile) = gfile.parent() {
                        let _ = dialog.set_current_folder(Some(&parent_gfile));
                    }
                }
                Err(e) => { 
                    eprintln!("Failed to create GFile for path {:?}: {}", current_dialog_dir_path, e);
                }
            }
            // Suggest current file name if available
            if let Some(p) = file_path_manager.borrow().get(&current_page_num) {
                if let Some(name) = p.file_name() {
                    dialog.set_current_name(&name.to_string_lossy());
                }
            }


            let editor_notebook_clone = editor_notebook.clone();
            let active_tab_path_ref_clone = active_tab_path_ref.clone();
            let file_path_manager_clone = file_path_manager.clone();
            let current_dir_clone = current_dir.clone();
            let file_list_box_clone = file_list_box.clone();

            dialog.connect_response(move |d, resp| {
                if resp == gtk4::ResponseType::Accept {
                    if let Some(file_to_save) = d.file().and_then(|f| f.path()) {
                        let mime_type = mime_guess::from_path(&file_to_save).first_or_octet_stream();
                        if utils::is_allowed_mime_type(&mime_type) {
                            if let Ok(mut f_obj) = File::create(&file_to_save) {
                                let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                                if f_obj.write_all(text.as_bytes()).is_ok() {
                                    file_path_manager_clone.borrow_mut().insert(current_page_num, file_to_save.clone());
                                    *active_tab_path_ref_clone.borrow_mut() = Some(file_to_save.clone());

                                    // Update tab label
                                    update_tab_label_after_save(&editor_notebook_clone, current_page_num, Some(&file_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                                    if let Some(parent) = file_to_save.parent() {
                                        *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                    }
                                     utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_clone.borrow());
                                }
                            }
                        }
                    }
                }
                d.close();
            });
            dialog.show();
        }
    });
}


fn setup_file_selection_handler(
    file_list_box: &ListBox,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>,
    error_label: &Label,
    picture: &Picture, // Needs tab-specific handling
    save_button: &Button,
    save_as_button: &Button,
    window: &ApplicationWindow, // Added for NewTabDependencies
    _save_menu_button: Option<&MenuButton> // Prefix with _ to acknowledge it's unused currently
) {
    let editor_notebook_clone = editor_notebook.clone(); // Renamed for clarity
    let active_tab_path_ref_clone = active_tab_path_ref.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let current_dir_clone = current_dir.clone();
    let file_list_box_for_update = file_list_box.clone(); 
    let _error_label_clone = error_label.clone();
    let _picture_clone = picture.clone(); // picture is now cloned
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    let window_clone = window.clone(); // For NewTabDependencies
    // Clone the MenuButton option to own it
    let save_menu_button_option = _save_menu_button.map(|btn| btn.clone());


    file_list_box.connect_row_activated(move |_, row| {
        // Clone necessary items again for the inner part of the closure if they are used across awaits or complex logic
        // For simple moves like this, the outer clones are usually sufficient.
        let editor_notebook_for_handler = editor_notebook_clone.clone();
        let active_tab_path_for_handler = active_tab_path_ref_clone.clone();
        let file_path_manager_for_handler = file_path_manager_clone.clone();
        let current_dir_for_handler = current_dir_clone.clone();
        let file_list_box_for_handler_update = file_list_box_for_update.clone();
        // No need to clone these as they're not used directly
        // let _error_label_for_handler = _error_label_clone.clone();
        // let _picture_for_handler = _picture_clone.clone();
        let save_button_for_handler = save_button_clone.clone();
        let save_as_button_for_handler = save_as_button_clone.clone();
        let window_for_handler = window_clone.clone();
        // Clone the already-owned option
        let save_menu_button_for_handler = save_menu_button_option.clone();


        if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
            let file_name = label.text();
            let mut path_from_list = current_dir_for_handler.borrow().clone(); // Use cloned current_dir
            path_from_list.push(&file_name.as_str());

            if path_from_list.is_dir() {
                *current_dir_for_handler.borrow_mut() = path_from_list;
                utils::update_file_list(&file_list_box_for_handler_update, &current_dir_for_handler.borrow(), &active_tab_path_for_handler.borrow());
                file_list_box_for_handler_update.grab_focus(); // Add this line to shift focus
            } else if path_from_list.is_file() {
                let mime_type = mime_guess::from_path(&path_from_list).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(content) = std::fs::read_to_string(&path_from_list) {                            open_or_focus_tab(
                            &editor_notebook_for_handler, 
                            &path_from_list,
                            &content,
                            &active_tab_path_for_handler, 
                            &file_path_manager_for_handler,   
                            &save_button_for_handler,
                            &save_as_button_for_handler,
                            &mime_type,
                            &window_for_handler, 
                            &file_list_box_for_handler_update, 
                            &current_dir_for_handler,
                            save_menu_button_for_handler.as_ref(), // Pass the save menu button option
                        );
                        // Ensure the list reflects the newly opened file as active
                        utils::update_file_list(
                            &file_list_box_for_handler_update,
                            &current_dir_for_handler.borrow(),
                            &active_tab_path_for_handler.borrow()
                        );
                    }
                } else if mime_type.type_() == "image" {
                    // Use open_or_focus_tab for images
                    open_or_focus_tab(
                        &editor_notebook_for_handler, 
                        &path_from_list,
                        "", // Empty content for images
                        &active_tab_path_for_handler, 
                        &file_path_manager_for_handler,   
                        &save_button_for_handler,
                        &save_as_button_for_handler,
                        &mime_type,
                        &window_for_handler, 
                        &file_list_box_for_handler_update, 
                        &current_dir_for_handler,
                        save_menu_button_for_handler.as_ref() // Pass the save menu button option
                    );
                    // Ensure the list reflects the newly opened file as active
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow()
                    );
                } else {
                    // Handle unsupported file type in a new tab
                    open_or_focus_tab(
                        &editor_notebook_for_handler, 
                        &path_from_list,
                        "", // Empty content for unsupported files
                        &active_tab_path_for_handler, 
                        &file_path_manager_for_handler,   
                        &save_button_for_handler,
                        &save_as_button_for_handler,
                        &mime_type,
                        &window_for_handler, 
                        &file_list_box_for_handler_update, 
                        &current_dir_for_handler,
                        save_menu_button_for_handler.as_ref(), // Pass the save menu button option
                    );
                    // Ensure the list reflects the newly opened file as active
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow()
                    );
                }
            }
        }
    });
}

fn setup_up_button_handler(
    up_button: &Button,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>> // Changed from file_path
) {
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let active_tab_path = active_tab_path.clone(); // Clone Rc for closure
    up_button.connect_clicked(move |_| {
        let mut path = current_dir.borrow().clone();
        if path.pop() {
            *current_dir.borrow_mut() = path;
            // Pass the active tab\'s path for selection highlighting
            utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &active_tab_path.borrow());
        }
    });
}

fn setup_refresh_button_handler(
    refresh_button: &Button,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>> // Changed from file_path
) {
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let active_tab_path = active_tab_path.clone(); // Clone Rc for closure
    refresh_button.connect_clicked(move |_| {
        // Pass the active tab\'s path for selection highlighting
        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow());
    });
}
