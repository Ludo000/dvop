// Event handlers and business logic for Dvop
// This module contains all the event handlers and core functionality for the editor

// GTK imports
use gtk4::prelude::*;
use gtk4::{
    // GLib
    glib::{self, clone},
    ApplicationWindow,
    // Layout
    Box as GtkBox,
    // Widgets
    Button,
    ButtonsType,
    DialogFlags,
    EventControllerKey,

    // Event handling
    GestureClick,
    Label,
    ListBox,
    MenuButton,

    // Dialog components
    MessageDialog,
    MessageType,
    Notebook,
    Orientation,

    Picture,
    ResponseType,

    ScrolledWindow,
    TextBuffer,
    TextView,
};

// SourceView specific imports
 // For specific types like Buffer and View

// Standard library imports
use std::cell::RefCell; // Interior mutability pattern
use std::collections::HashMap; // For efficient mapping and compatibility
use std::fs::File; // File operations
use std::io::Write;
use std::path::PathBuf; // File system path representation
use std::rc::Rc; // Reference counting for shared ownership // File writing capabilities

// Internal imports
use crate::utils; // Utility functions

/// Gets the TextView and TextBuffer from the currently active notebook tab
///
/// This function navigates the widget hierarchy to find the text view in the current tab.
/// Returns None if there is no active tab or if the tab doesn't contain a text view
/// (e.g., if it's showing an image instead).
pub fn get_active_text_view_and_buffer(notebook: &Notebook) -> Option<(TextView, TextBuffer)> {
    // Get the current page number, then use it to find the page widget
    notebook.current_page().and_then(|page_num| {
        notebook.nth_page(Some(page_num)).and_then(|page_widget| {
            // First, check if the page is a Paned widget (for Markdown/SVG split view)
            if let Some(paned) = page_widget.downcast_ref::<gtk4::Paned>() {
                // Get the left child (which contains the code editor)
                if let Some(left_child) = paned.start_child() {
                    // The left child should be a ScrolledWindow containing the TextView
                    if let Some(scrolled_window) = left_child.downcast_ref::<ScrolledWindow>() {
                        if let Some(child) = scrolled_window.child() {
                            if let Some(text_view) = child.downcast_ref::<TextView>() {
                                return Some((text_view.clone(), text_view.buffer()));
                            }
                        }
                    }
                }
                return None;
            }
            
            // Check if the page contains a ScrolledWindow (typical for text content)
            if let Some(scrolled_window) = page_widget.downcast_ref::<ScrolledWindow>() {
                // Get the child of the ScrolledWindow
                scrolled_window.child().and_then(|child| {
                    // Try to cast the child to a TextView
                    child.downcast_ref::<TextView>().map(|text_view| (text_view.clone(), text_view.buffer()))
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
pub fn get_text_view_and_buffer_for_page(
    notebook: &Notebook,
    page_num: u32,
) -> Option<(TextView, TextBuffer)> {
    // Get the page widget for the specified page number
    notebook.nth_page(Some(page_num)).and_then(|page_widget| {
        // First, check if the page is a Paned widget (for SVG split view)
        if let Some(paned) = page_widget.downcast_ref::<gtk4::Paned>() {
            // Get the left child (which contains the code editor)
            if let Some(left_child) = paned.start_child() {
                // The left child should be a ScrolledWindow containing the TextView
                if let Some(scrolled_window) = left_child.downcast_ref::<ScrolledWindow>() {
                    if let Some(child) = scrolled_window.child() {
                        if let Some(text_view) = child.downcast_ref::<TextView>() {
                            return Some((text_view.clone(), text_view.buffer()));
                        }
                    }
                }
            }
            return None;
        }

        // Check if the page contains a ScrolledWindow (normal text files)
        if let Some(scrolled_window) = page_widget.downcast_ref::<ScrolledWindow>() {
            // Get the child of the ScrolledWindow
            scrolled_window.child().and_then(|child| {
                // Try to cast the child to a TextView
                child.downcast_ref::<TextView>().map(|text_view| (text_view.clone(), text_view.buffer()))
            })
        } else {
            // Page widget is not a ScrolledWindow
            None
        }
    })
}

/// Helper function to get the SourceView buffer from a TextView
/// This is needed because we upcast SourceView to TextView for compatibility,
/// but syntax highlighting needs the original SourceView buffer
fn get_source_buffer_from_text_view(text_view: &TextView) -> Option<sourceview5::Buffer> {
    // Try to downcast the TextView back to SourceView
    if let Ok(source_view) = text_view.clone().downcast::<sourceview5::View>() {
        // Get the buffer and try to downcast it to SourceView Buffer
        if let Ok(source_buffer) = source_view.buffer().downcast::<sourceview5::Buffer>() {
            return Some(source_buffer);
        }
    }
    None
}

/// Helper function to apply syntax highlighting to a file after save
/// Gets the source buffer and applies syntax highlighting based on file extension
fn apply_syntax_highlighting_after_save(
    notebook: &Notebook,
    page_num: u32,
    file_path: &std::path::Path,
) {
    if let Some((text_view, _)) = get_text_view_and_buffer_for_page(notebook, page_num) {
        if let Some(source_buffer) = get_source_buffer_from_text_view(&text_view) {
            crate::syntax::set_language_for_file(&source_buffer, file_path);
        }
    }
}

/// Structure containing all dependencies needed for tab creation and management
///
/// This structure holds references to all the components and state that need
/// to be modified when creating, switching, or closing tabs. It makes it easier
/// to pass these references to various tab-related functions.
///
/// Using weak references where possible to prevent circular reference memory leaks.
#[derive(Clone)]
pub struct NewTabDependencies {
    // Core UI components (using weak refs to prevent cycles)
    pub editor_notebook: Notebook, // The tabbed container
    pub window: ApplicationWindow, // Main window (for dialog parents)
    pub file_list_box: ListBox,    // File browser list

    // State tracking
    pub active_tab_path: Rc<RefCell<Option<PathBuf>>>, // Currently active file path
    pub file_path_manager: Rc<RefCell<HashMap<u32, PathBuf>>>, // Maps tab indices to file paths
    pub current_dir: Rc<RefCell<PathBuf>>,             // Current working directory

    // Action buttons
    pub save_button: Button,                   // Save button
    pub save_as_button: Button,                // Save As button
    pub _save_menu_button: Option<MenuButton>, // Split button menu component (unused but kept for future)
}

/// Creates a new empty tab with the title "Untitled"
///
/// This function is used to create a new tab for a new document,
/// setting up all the necessary UI components and state tracking.
pub fn create_new_empty_tab(deps: &NewTabDependencies) {
    // Log new file creation
    crate::status_log::log_info("Creating new file...");

    // Create a new source view with syntax highlighting capabilities
    let (source_view, source_buffer) = crate::syntax::create_source_view();
    
    // Setup breakpoints
    crate::debugger::breakpoints::setup_breakpoint_attributes(&source_view);
    crate::debugger::breakpoints::setup_gutter_click_handler(
        &source_view, 
        deps.editor_notebook.clone(), 
        deps.file_path_manager.clone()
    );

    source_buffer.set_text(""); // Start with empty content

    // Clone source_view to avoid ownership move - use Rc instead of full clone for efficiency
    let new_text_view = source_view.clone().upcast::<TextView>();
    let new_text_buffer = source_buffer.upcast::<TextBuffer>();

    // Set up interaction tracking for the new text editor
    setup_text_editor_interaction_tracking(&new_text_view);

    // Place the source view in a scrollable container
    let new_scrolled_window = crate::syntax::create_source_view_scrolled(&source_view);

    // Create a custom tab widget with label and close button
    let (tab_widget, tab_actual_label, tab_close_button) = crate::ui::create_tab_widget("Untitled");

    // Add middle mouse click support for the tab
    crate::ui::setup_tab_middle_click(&tab_widget, &tab_close_button);

    // Add right-click context menu support for the tab
    crate::ui::setup_tab_right_click(
        &tab_widget,
        &deps.editor_notebook,
        &deps.window,
        &deps.file_path_manager,
        &deps.active_tab_path,
        &deps.current_dir,
        &deps.file_list_box,
        Some(deps.clone()),
    );

    // Add the new tab to the notebook and switch to it
    let new_page_num = deps
        .editor_notebook
        .append_page(&new_scrolled_window, Some(&tab_widget));
    // Setting current page after append ensures the switch_page signal is emitted properly
    deps.editor_notebook.set_current_page(Some(new_page_num));

    // Focus the text area of the new tab so the user can start typing immediately
    new_text_view.grab_focus();

    // Mark text editor as the last active area
    LAST_ACTIVE_AREA.with(|area| {
        *area.borrow_mut() = LastActiveArea::TextEditor;
    });

    // Update the active tab path to None (unsaved document)
    *deps.active_tab_path.borrow_mut() = None;

    // Note: We don't update file_path_manager for "Untitled" tabs until they're saved

    // Get immutable references to avoid unnecessary clones
    let current_dir_ref = deps.current_dir.borrow();
    let active_path_ref = deps.active_tab_path.borrow();

    // Update the file browser to reflect the current state
    utils::update_file_list(
        &deps.file_list_box,
        &current_dir_ref,
        &active_path_ref,
        utils::FileSelectionSource::TabSwitch,
    );

    // Drop borrows before continuing
    drop(current_dir_ref);
    drop(active_path_ref);

    // Enable save buttons appropriate for plain text content
    utils::update_save_buttons_visibility(
        &deps.save_button,
        &deps.save_as_button,
        Some(mime_guess::mime::TEXT_PLAIN_UTF_8),
    );

    // Also update the split button menu visibility if present
    if let Some(ref save_menu_button) = deps._save_menu_button {
        utils::update_save_menu_button_visibility(
            save_menu_button,
            Some(mime_guess::mime::TEXT_PLAIN_UTF_8),
        );
    }

    // Log success
    crate::status_log::log_success("New file ready");

    // Connect dirty tracking for the new "Untitled" tab's label
    // Use weak reference to prevent memory leaks from circular references
    let tab_actual_label_weak = tab_actual_label.downgrade();
    new_text_buffer.connect_changed(move |buffer| {
        // Mark text editor as active when user actually types/modifies content
        LAST_ACTIVE_AREA.with(|area| {
            *area.borrow_mut() = LastActiveArea::TextEditor;
        });

        if let Some(label) = tab_actual_label_weak.upgrade() {
            let label_text = label.text();
            let buffer_content = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);

            if label_text == "Untitled" && !buffer_content.is_empty() {
                label.set_text("*Untitled");
                crate::status_log::log_info("File modified");
            } else if label_text == "*Untitled" && buffer_content.is_empty() {
                label.set_text("Untitled");
                crate::status_log::log_info("File no longer modified");
            }
        }
    });

    // Connect close button for this new tab
    let deps_clone_for_close = deps.clone();
    let new_scrolled_window_clone = new_scrolled_window.clone();
    tab_close_button.connect_clicked(move |_| {
        // Find the current page number of this tab using the correct widget reference
        if let Some(current_idx_for_this_tab) = deps_clone_for_close
            .editor_notebook
            .page_num(&new_scrolled_window_clone)
        {
            handle_close_tab_request(
                &deps_clone_for_close.editor_notebook,
                current_idx_for_this_tab,
                &deps_clone_for_close.window,
                &deps_clone_for_close.file_path_manager,
                &deps_clone_for_close.active_tab_path,
                &deps_clone_for_close.current_dir,
                &deps_clone_for_close.file_list_box,
                Some(deps_clone_for_close.clone()),
            );
        }
    });

    // Log successful creation
    crate::status_log::log_success("New file ready");
}

// Helper function to update tab label after save or name change
// Optimized to reduce string allocations
pub fn update_tab_label_after_save(
    notebook: &Notebook,
    page_num: u32,
    new_name_opt: Option<&str>,
    is_now_dirty: bool,
) {
    if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
        if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
            if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                if let Some(label) = tab_box
                    .first_child()
                    .and_then(|w| w.downcast::<Label>().ok())
                {
                    let current_text = label.text();
                    let base_name = new_name_opt
                        .unwrap_or_else(|| current_text.strip_prefix('*').unwrap_or(&current_text));

                    let final_text = if is_now_dirty {
                        if current_text.starts_with('*') {
                            current_text.to_string() // Already has asterisk
                        } else {
                            format!("*{}", base_name)
                        }
                    } else {
                        base_name.to_string()
                    };

                    // Only update if text actually changed
                    if final_text != current_text {
                        label.set_text(&final_text);
                    }
                }
            }
        }
    }
}

pub fn handle_close_tab_request(
    notebook: &Notebook,
    page_num_to_close: u32,
    window: &impl IsA<ApplicationWindow>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>,       // New
    file_list_box: &ListBox,                  // New
    new_tab_deps: Option<NewTabDependencies>, // Dependencies to create a new tab if the last one is closed
) {
    if let Some(page_widget) = notebook.nth_page(Some(page_num_to_close)) {
        // Get file name for logging
        let filename = file_path_manager
            .borrow()
            .get(&page_num_to_close)
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());

        crate::status_log::log_info(&format!("Closing {}", filename));

        if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
            let mut is_dirty = false;
            if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                if let Some(label) = tab_box
                    .first_child()
                    .and_then(|w| w.downcast::<Label>().ok())
                {
                    if label.text().starts_with('*') {
                        is_dirty = true;
                    }
                }
            }

            if !is_dirty {
                // Not dirty, close directly
                actually_close_tab(
                    notebook,
                    page_num_to_close,
                    file_path_manager,
                    active_tab_path,
                    new_tab_deps.as_ref(),
                );
                return;
            }

            // Is dirty, show confirmation dialog
            // Use more efficient string handling to avoid temporary borrow issues
            let filename_str = {
                let manager = file_path_manager.borrow();
                manager
                    .get(&page_num_to_close)
                    .and_then(|p| p.file_name()?.to_str())
                    .unwrap_or("Untitled")
                    .to_owned()
            };
            let dialog = MessageDialog::new(
                Some(window.as_ref().upcast_ref::<gtk4::Window>()),
                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                MessageType::Question,
                ButtonsType::None,
                format!("Save changes to {} before closing?", filename_str),
            );
            dialog.add_buttons(&[
                ("Cancel", ResponseType::Cancel),
                ("Don't Save", ResponseType::No),
                ("Save", ResponseType::Yes),
            ]);

            dialog.set_default_response(ResponseType::Cancel);

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
                        if let Some((_tv, buffer)) =
                            get_text_view_and_buffer_for_page(&notebook_clone, page_num_to_close)
                        {
                            let path_opt = file_path_manager_clone
                                .borrow()
                                .get(&page_num_to_close)
                                .cloned();
                            if let Some(path) = path_opt {
                                // Existing file
                                let text =
                                    buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
                                match File::create(&path) {
                                    Ok(mut file) => {
                                        if file.write_all(text.as_bytes()).is_ok() {
                                            update_tab_label_after_save(
                                                &notebook_clone,
                                                page_num_to_close,
                                                Some(
                                                    &path
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy(),
                                                ),
                                                false,
                                            );

                                            // Apply syntax highlighting based on file extension
                                            apply_syntax_highlighting_after_save(
                                                &notebook_clone,
                                                page_num_to_close,
                                                &path,
                                            );

                                            // Notify LSP that file was saved (for rust-analyzer)
                                            crate::linter::ui::notify_file_saved(&path);

                                            actually_close_tab(
                                                &notebook_clone,
                                                page_num_to_close,
                                                &file_path_manager_clone,
                                                &active_tab_path_clone,
                                                new_tab_deps_clone.as_ref(),
                                            );
                                        } else {
                                            eprintln!("Error writing to file: {:?}", path);
                                            // Optionally show error dialog to user
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Error creating file for writing: {:?}, error: {}",
                                            path, e
                                        );
                                        // Optionally show error dialog
                                    }
                                }
                            } else {
                                // Untitled file, need to "Save As"
                                let save_as_dialog = gtk4::FileChooserDialog::new(
                                    Some("Save File As"),
                                    Some(window_clone.as_ref().upcast_ref::<gtk4::Window>()),
                                    gtk4::FileChooserAction::Save,
                                    &[
                                        ("Cancel", gtk4::ResponseType::Cancel),
                                        ("Save", gtk4::ResponseType::Accept),
                                    ],
                                );

                                save_as_dialog.set_default_response(gtk4::ResponseType::Cancel);

                                let current_dialog_dir_path = current_dir_clone.borrow().clone();

                                // Explicitly type annotation for gio_file_result and wrap the call in Ok()
                                let gio_file_result: Result<gtk4::gio::File, glib::Error> =
                                    Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
                                match gio_file_result {
                                    Ok(gfile) => {
                                        if current_dialog_dir_path.is_dir() {
                                            let _ = save_as_dialog.set_current_folder(Some(&gfile));
                                        } else if let Some(parent_gfile) = gfile.parent() {
                                            let _ = save_as_dialog
                                                .set_current_folder(Some(&parent_gfile));
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!(
                                            "Failed to create GFile for path {:?}: {}",
                                            current_dialog_dir_path, e
                                        );
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
                                        if let Some(file_to_save) =
                                            d_sa.file().and_then(|f| f.path())
                                        {
                                            let text_to_save = buffer_clone_for_save_as.text(
                                                &buffer_clone_for_save_as.start_iter(),
                                                &buffer_clone_for_save_as.end_iter(),
                                                false,
                                            );
                                            match File::create(&file_to_save) {
                                                Ok(mut f_obj) => {
                                                    if f_obj
                                                        .write_all(text_to_save.as_bytes())
                                                        .is_ok()
                                                    {
                                                        fpm_save_as.borrow_mut().insert(
                                                            page_num_to_close,
                                                            file_to_save.clone(),
                                                        );
                                                        if nc_save_as.current_page()
                                                            == Some(page_num_to_close)
                                                        {
                                                            *atp_save_as.borrow_mut() =
                                                                Some(file_to_save.clone());
                                                        }
                                                        update_tab_label_after_save(
                                                            &nc_save_as,
                                                            page_num_to_close,
                                                            Some(
                                                                &file_to_save
                                                                    .file_name()
                                                                    .unwrap_or_default()
                                                                    .to_string_lossy(),
                                                            ),
                                                            false,
                                                        );

                                                        // Apply syntax highlighting based on file extension
                                                        apply_syntax_highlighting_after_save(
                                                            &nc_save_as,
                                                            page_num_to_close,
                                                            &file_to_save,
                                                        );

                                                        // Notify LSP that file was saved (for rust-analyzer)
                                                        crate::linter::ui::notify_file_saved(
                                                            &file_to_save,
                                                        );

                                                        if let Some(parent) = file_to_save.parent()
                                                        {
                                                            *cd_save_as.borrow_mut() =
                                                                parent.to_path_buf();
                                                        }
                                                        utils::update_file_list(
                                                            &flb_save_as,
                                                            &cd_save_as.borrow(),
                                                            &atp_save_as.borrow(),
                                                            utils::FileSelectionSource::TabSwitch,
                                                        );
                                                        actually_close_tab(
                                                            &nc_save_as,
                                                            page_num_to_close,
                                                            &fpm_save_as,
                                                            &atp_save_as,
                                                            ntd_save_as.as_ref(),
                                                        );
                                                    } else {
                                                        eprintln!(
                                                            "Error writing to new file: {:?}",
                                                            file_to_save
                                                        );
                                                    }
                                                }
                                                Err(e) => {
                                                    eprintln!(
                                                        "Error creating new file: {:?}, error: {}",
                                                        file_to_save, e
                                                    );
                                                }
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
                        actually_close_tab(
                            &notebook_clone,
                            page_num_to_close,
                            &file_path_manager_clone,
                            &active_tab_path_clone,
                            new_tab_deps_clone.as_ref(),
                        );
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

// Optimized tab closing function - more efficient index management
fn actually_close_tab(
    notebook: &Notebook,
    page_num_to_close: u32,
    file_path_manager_rc: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path_rc: &Rc<RefCell<Option<PathBuf>>>,
    new_tab_deps: Option<&NewTabDependencies>,
) {
    let n_pages_before_close = notebook.n_pages();
    
    println!("\n=== actually_close_tab called ===");
    println!("Attempting to close page {}", page_num_to_close);
    println!("Total pages before close: {}", n_pages_before_close);

    // Get file path and filename for logging and audio cleanup before we remove it
    let (filename, file_path_opt) = {
        let manager = file_path_manager_rc.borrow();
        let path_opt = manager.get(&page_num_to_close).cloned();
        let name = path_opt
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string());
        (name, path_opt)
    };
    
    println!("Closing file: {}", filename);

    // Stop any audio playback for this file if it's a music file
    if let Some(ref file_path) = file_path_opt {
        if crate::audio::is_music_file(file_path) {
            crate::audio::stop_audio_for_file(file_path);
        }
        // Stop any video playback for this file if it's a video file
        if crate::video::is_video_file(file_path) {
            crate::video::stop_video_for_file(file_path);
        }
    }

    notebook.remove_page(Some(page_num_to_close));

    crate::status_log::log_success(&format!("Closed {}", filename));

    // Efficiently handle HashMap index updates
    {
        let mut manager = file_path_manager_rc.borrow_mut();
        println!("=== Closing tab {} ===", page_num_to_close);
        println!("Before close:");
        for (k, v) in manager.iter() {
            println!("  Page {}: {:?}", k, v.file_name().unwrap_or_default());
        }
        
        manager.remove(&page_num_to_close);

        // Collect entries above the closed index and reinsert with decremented keys
        let entries_to_update: Vec<(u32, PathBuf)> = manager
            .iter()
            .filter_map(|(&k, v)| {
                if k > page_num_to_close {
                    Some((k, v.clone()))
                } else {
                    None
                }
            })
            .collect();

        // Remove old entries
        for &(key, _) in &entries_to_update {
            manager.remove(&key);
        }

        // Reinsert with decremented indices
        for (old_key, path) in entries_to_update {
            manager.insert(old_key - 1, path);
        }
        
        println!("After close:");
        for (k, v) in manager.iter() {
            println!("  Page {}: {:?}", k, v.file_name().unwrap_or_default());
        }
        println!("Total pages after close: {}", notebook.n_pages());
        println!("=== End tab close ===\n");
    } // Drop mutable borrow here

    if notebook.n_pages() == 0 {
        // No pages left, active_tab_path should be None.
        *active_tab_path_rc.borrow_mut() = None;

        // Only create a new empty tab if this wasn't the last tab and we have dependencies
        if n_pages_before_close > 1 && new_tab_deps.is_some() {
            if let Some(deps) = new_tab_deps {
                // It's now safe to call create_new_empty_tab as the mutable borrow
                // on file_path_manager_rc has been released.
                create_new_empty_tab(deps);
            }
        }
        // If it was the last tab (n_pages_before_close == 1), we don't create a new one
    } else {
        // If other tabs remain, GTK will automatically switch to a new page (e.g., the one at page_num_to_close, or page 0).
        // The connect_switch_page handler in main.rs is responsible for updating active_tab_path.
        // We need to ensure that file_path_manager contains the correct path for the new current page.

        // Get the current page after tab removal and update active_tab_path
        if let Some(current_page) = notebook.current_page() {
            let new_active_path = file_path_manager_rc.borrow().get(&current_page).cloned();
            *active_tab_path_rc.borrow_mut() = new_active_path;

            // If we have dependencies provided, update the file list selection
            if let Some(deps) = new_tab_deps {
                utils::update_file_list(
                    &deps.file_list_box,
                    &deps.current_dir.borrow(),
                    &active_tab_path_rc.borrow(),
                    utils::FileSelectionSource::TabSwitch,
                );
            }
        }
        // The re-indexing above should have handled this.
        // If the active tab was closed, switch_page will fire. If a different tab was closed,
        // the current page might not change, but its index in file_path_manager might be wrong if it was after the closed tab.
        // However, the switch_page handler uses the *new* page_num provided by the signal, which should be correct.
    }
}

/// Enter fullscreen mode for an image
/// For animated GIFs, we need to share the Picture widget data, not just copy the paintable
fn enter_image_fullscreen(picture: &Picture) {
    // Get the paintable from the original picture
    let paintable = picture.paintable();
    if paintable.is_none() {
        println!("Image: No paintable available for fullscreen");
        return;
    }

    // Create fullscreen window
    let fullscreen_window = gtk4::Window::new();
    fullscreen_window.set_title(Some("Image - Fullscreen"));
    fullscreen_window.set_decorated(false);
    fullscreen_window.set_resizable(true);
    fullscreen_window.set_modal(false);

    // Create a new picture widget for fullscreen with the same paintable
    let fullscreen_picture = Picture::new();
    fullscreen_picture.set_paintable(paintable.as_ref());
    fullscreen_picture.set_can_shrink(true); // Allow shrinking to fit
    fullscreen_picture.set_keep_aspect_ratio(true);
    fullscreen_picture.set_hexpand(true);
    fullscreen_picture.set_vexpand(true);
    fullscreen_picture.set_halign(gtk4::Align::Center);
    fullscreen_picture.set_valign(gtk4::Align::Center);

    // For animated GIFs, we need to keep updating the fullscreen picture
    // when the original picture updates at high frequency for smooth animation
    let original_picture = picture.clone();
    let fullscreen_picture_clone = fullscreen_picture.clone();
    let fullscreen_window_for_update = fullscreen_window.clone();

    // Poll for updates at 60fps (16ms) for smooth animation
    glib::timeout_add_local(
        std::time::Duration::from_millis(16),
        clone!(
            #[weak]
            original_picture,
            #[weak]
            fullscreen_picture_clone,
            #[weak]
            fullscreen_window_for_update,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move || {
                // Stop if fullscreen window is closed
                if !fullscreen_window_for_update.is_visible() {
                    return glib::ControlFlow::Break;
                }

                // Sync the paintable from original to fullscreen
                if let Some(paintable) = original_picture.paintable() {
                    fullscreen_picture_clone.set_paintable(Some(&paintable));
                }

                glib::ControlFlow::Continue
            }
        ),
    );

    // Create a container for the image in fullscreen
    let fullscreen_box = GtkBox::new(Orientation::Vertical, 0);
    fullscreen_box.set_vexpand(true);
    fullscreen_box.set_hexpand(true);
    fullscreen_box.add_css_class("fullscreen-image");
    fullscreen_box.append(&fullscreen_picture);
    fullscreen_window.set_child(Some(&fullscreen_box));

    // Present the window first, then fullscreen
    fullscreen_window.present();

    // Delay fullscreen to ensure proper initialization
    let fullscreen_window_fs = fullscreen_window.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        fullscreen_window_fs.fullscreen();
    });

    // Add Escape and F key handler to exit fullscreen
    let key_controller = EventControllerKey::new();
    let fullscreen_window_keys = fullscreen_window.clone();

    key_controller.connect_key_pressed(move |_controller, key, _code, _modifier| {
        match key {
            // Escape or F: Exit fullscreen
            gtk4::gdk::Key::Escape | gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                println!("Image: Exiting fullscreen via key press");
                fullscreen_window_keys.close();
                return glib::Propagation::Stop;
            }
            _ => {}
        }
        glib::Propagation::Proceed
    });
    fullscreen_window.add_controller(key_controller);

    // Add double-click gesture to exit fullscreen on the fullscreen picture
    let double_click_gesture = GestureClick::new();
    double_click_gesture.set_button(1); // Left mouse button
    let fullscreen_window_double_click = fullscreen_window.clone();

    double_click_gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
        if n_press == 2 {
            // Double-click
            println!("Image: Exiting fullscreen via double-click");
            fullscreen_window_double_click.close();
        }
    });
    fullscreen_picture.add_controller(double_click_gesture);

    println!("Image: Fullscreen window created and shown");
}

/// Creates a split view for SVG files with code on the left and rendered image on the right
///
/// This function creates a horizontal paned widget with:
/// - Left side: Source code editor with XML syntax highlighting
/// - Right side: Rendered SVG image
fn create_svg_split_view(
    content: &str,
    file_path: &std::path::Path,
    tab_actual_label: &Label,
    file_name: &str,
) -> gtk4::Paned {
    // Create the paned widget for split view
    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    paned.set_resize_start_child(true);
    paned.set_resize_end_child(true);

    // LEFT SIDE: Code editor
    let (source_view, source_buffer) = crate::syntax::create_source_view();
    source_buffer.set_text(content);

    // Apply XML syntax highlighting for SVG
    crate::syntax::set_language_for_file(&source_buffer, file_path);

    // Setup completion for XML/SVG
    crate::completion::setup_completion_for_file(&source_view, Some(file_path));
    crate::completion::setup_completion_shortcuts(&source_view);

    // Setup linting for the file
    crate::linter::ui::setup_linting(&source_view, Some(file_path));

    // Set up interaction tracking for the text editor
    let text_view = source_view.clone().upcast::<TextView>();
    setup_text_editor_interaction_tracking(&text_view);

    let left_scrolled = crate::syntax::create_source_view_scrolled(&source_view);
    left_scrolled.set_vexpand(true);
    left_scrolled.set_hexpand(true);

    // Track changes for dirty state
    let tab_label_weak = tab_actual_label.downgrade();
    let file_name_clone = file_name.to_string();
    let text_buffer = source_buffer.clone().upcast::<TextBuffer>();
    text_buffer.connect_changed(move |_buffer| {
        LAST_ACTIVE_AREA.with(|area| {
            *area.borrow_mut() = LastActiveArea::TextEditor;
        });

        if let Some(label) = tab_label_weak.upgrade() {
            let current_text = label.text();
            if !current_text.starts_with('*') {
                label.set_text(&format!("*{}", file_name_clone));
                crate::status_log::log_info(&format!("{} modified", file_name_clone));
            }
        }
    });

    // RIGHT SIDE: Rendered SVG image
    let right_box = GtkBox::new(Orientation::Vertical, 0);
    right_box.set_vexpand(true);
    right_box.set_hexpand(true);

    // Add a label to indicate this is the preview
    let preview_label = Label::new(Some("Preview"));
    preview_label.set_halign(gtk4::Align::Start);
    preview_label.set_margin_start(10);
    preview_label.set_margin_top(5);
    preview_label.set_margin_bottom(5);
    preview_label.add_css_class("dim-label");
    right_box.append(&preview_label);

    // Create Picture widget for SVG rendering
    let picture = Picture::new();

    // Load SVG from file
    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(file_path) {
        picture.set_pixbuf(Some(&pixbuf));
        picture.set_can_focus(true);
        picture.set_focusable(true);

        // Add fullscreen support (F key and double-click)
        let key_controller = EventControllerKey::new();
        let picture_keys = picture.clone();

        key_controller.connect_key_pressed(move |_controller, key, _code, _modifier| {
            match key {
                gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                    enter_image_fullscreen(&picture_keys);
                    return glib::Propagation::Stop;
                }
                _ => {}
            }
            glib::Propagation::Proceed
        });
        picture.add_controller(key_controller);

        let double_click_gesture = GestureClick::new();
        double_click_gesture.set_button(1);
        let picture_fullscreen = picture.clone();

        double_click_gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
            if n_press == 2 {
                enter_image_fullscreen(&picture_fullscreen);
            }
        });
        picture.add_controller(double_click_gesture);
    } else {
        // If loading fails, show error message
        let error_label = Label::new(Some("Failed to render SVG"));
        error_label.add_css_class("error");
        right_box.append(&error_label);
    }

    // Put picture in a scrolled window
    let right_scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Automatic)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .vexpand(true)
        .hexpand(true)
        .build();
    right_scrolled.set_child(Some(&picture));
    right_box.append(&right_scrolled);

    // Add both sides to the paned widget
    paned.set_start_child(Some(&left_scrolled));
    paned.set_end_child(Some(&right_box));

    // Set initial position to 50/50 split
    paned.set_position(400); // Default split position (will adjust based on window width)

    paned
}

/// Creates a split view for Markdown files with code on the left and rendered preview on the right
///
/// This function creates a horizontal paned widget with:
/// - Left side: Source code editor with Markdown syntax highlighting
/// - Right side: Rendered markdown preview with basic styling
fn create_markdown_split_view(
    content: &str,
    file_path: &std::path::Path,
    tab_actual_label: &Label,
    file_name: &str,
) -> gtk4::Paned {
    // Create the paned widget for split view
    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_wide_handle(true);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    paned.set_resize_start_child(true);
    paned.set_resize_end_child(true);

    // LEFT SIDE: Code editor
    let (source_view, source_buffer) = crate::syntax::create_source_view();
    source_buffer.set_text(content);

    // Apply Markdown syntax highlighting
    crate::syntax::set_language_for_file(&source_buffer, file_path);

    // Setup completion for Markdown
    crate::completion::setup_completion_for_file(&source_view, Some(file_path));
    crate::completion::setup_completion_shortcuts(&source_view);

    // Setup linting for the file
    crate::linter::ui::setup_linting(&source_view, Some(file_path));

    // Set up interaction tracking for the text editor
    let text_view = source_view.clone().upcast::<TextView>();
    setup_text_editor_interaction_tracking(&text_view);

    let left_scrolled = crate::syntax::create_source_view_scrolled(&source_view);
    left_scrolled.set_vexpand(true);
    left_scrolled.set_hexpand(true);

    // RIGHT SIDE: Rendered preview
    let right_box = GtkBox::new(Orientation::Vertical, 0);
    right_box.set_vexpand(true);
    right_box.set_hexpand(true);

    // Add a label to indicate this is the preview
    let preview_label = Label::new(Some("Preview"));
    preview_label.set_halign(gtk4::Align::Start);
    preview_label.set_margin_start(10);
    preview_label.set_margin_top(5);
    preview_label.set_margin_bottom(5);
    preview_label.add_css_class("dim-label");
    right_box.append(&preview_label);

    // Create WebView for HTML preview
    use webkit6::WebView;
    use webkit6::prelude::*;
    let webview = WebView::new();
    webview.set_vexpand(true);
    webview.set_hexpand(true);

    // Helper function to convert markdown to HTML
    let markdown_to_html = |markdown_text: &str| -> String {
        use pulldown_cmark::{html, Options, Parser};
        
        let mut options = Options::empty();
        options.insert(Options::ENABLE_STRIKETHROUGH);
        options.insert(Options::ENABLE_TABLES);
        options.insert(Options::ENABLE_FOOTNOTES);
        options.insert(Options::ENABLE_TASKLISTS);
        
        let parser = Parser::new_ext(markdown_text, options);
        let mut html_output = String::new();
        html::push_html(&mut html_output, parser);
        
        // Wrap in a styled HTML document with dark mode support
        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="color-scheme" content="light dark">
    <style>
        :root {{
            --bg-color: #ffffff;
            --text-color: #333333;
            --border-color: #dddddd;
            --table-header-bg: #f2f2f2;
            --table-stripe-bg: #f9f9f9;
            --code-bg: #f4f4f4;
            --code-text: #000000;
            --link-color: #0366d6;
            --blockquote-border: #dddddd;
            --blockquote-text: #666666;
            --heading-border: #eeeeee;
        }}
        
        @media (prefers-color-scheme: dark) {{
            :root {{
                --bg-color: #1e1e1e;
                --text-color: #d4d4d4;
                --border-color: #404040;
                --table-header-bg: #2d2d2d;
                --table-stripe-bg: #252525;
                --code-bg: #2d2d2d;
                --code-text: #d4d4d4;
                --link-color: #4dabf7;
                --blockquote-border: #404040;
                --blockquote-text: #999999;
                --heading-border: #404040;
            }}
        }}
        
        body {{
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
            line-height: 1.6;
            padding: 20px;
            max-width: 900px;
            margin: 0 auto;
            color: var(--text-color);
            background-color: var(--bg-color);
        }}
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 20px 0;
        }}
        th, td {{
            border: 1px solid var(--border-color);
            padding: 12px;
            text-align: left;
        }}
        th {{
            background-color: var(--table-header-bg);
            font-weight: bold;
        }}
        tr:nth-child(even) {{
            background-color: var(--table-stripe-bg);
        }}
        code {{
            background-color: var(--code-bg);
            color: var(--code-text);
            padding: 2px 6px;
            border-radius: 3px;
            font-family: 'Courier New', monospace;
        }}
        pre {{
            background-color: var(--code-bg);
            padding: 10px;
            border-radius: 5px;
            overflow-x: auto;
        }}
        pre code {{
            background-color: transparent;
            padding: 0;
        }}
        h1, h2, h3, h4, h5, h6 {{
            margin-top: 24px;
            margin-bottom: 16px;
            font-weight: 600;
            line-height: 1.25;
        }}
        h1 {{ font-size: 2em; border-bottom: 1px solid var(--heading-border); padding-bottom: 10px; }}
        h2 {{ font-size: 1.5em; border-bottom: 1px solid var(--heading-border); padding-bottom: 8px; }}
        h3 {{ font-size: 1.25em; }}
        a {{ color: var(--link-color); text-decoration: none; }}
        a:hover {{ text-decoration: underline; }}
        blockquote {{
            border-left: 4px solid var(--blockquote-border);
            padding-left: 16px;
            color: var(--blockquote-text);
            margin: 0;
        }}
        ul, ol {{ padding-left: 2em; }}
    </style>
</head>
<body>
{}
</body>
</html>"#,
            html_output
        )
    };

    // Render initial content
    let initial_html = markdown_to_html(content);
    
    // Set base URI to the file's directory so relative links work
    let base_uri = file_path
        .parent()
        .and_then(|p| p.to_str())
        .map(|p| format!("file://{}/", p));
    
    webview.load_html(&initial_html, base_uri.as_deref());

    // WebView handles its own scrolling, add it directly
    right_box.append(&webview);

    // Track changes for dirty state and update preview
    let tab_label_weak = tab_actual_label.downgrade();
    let file_name_clone = file_name.to_string();
    let webview_clone = webview.clone();
    let file_path_clone = file_path.to_path_buf();
    let text_buffer = source_buffer.clone().upcast::<TextBuffer>();

    text_buffer.connect_changed(move |buffer| {
        LAST_ACTIVE_AREA.with(|area| {
            *area.borrow_mut() = LastActiveArea::TextEditor;
        });

        if let Some(label) = tab_label_weak.upgrade() {
            let current_text = label.text();
            if !current_text.starts_with('*') {
                label.set_text(&format!("*{}", file_name_clone));
                crate::status_log::log_info(&format!("{} modified", file_name_clone));
            }
        }

        // Update preview when content changes
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        let markdown_content = buffer.text(&start, &end, false);
        let html = markdown_to_html(&markdown_content.to_string());
        
        // Set base URI so relative links work
        let base_uri = file_path_clone
            .parent()
            .and_then(|p| p.to_str())
            .map(|p| format!("file://{}/", p));
        
        webview_clone.load_html(&html, base_uri.as_deref());
    });

    // OLD TextView code removed - now using WebView
    // Add both sides to the paned widget

    // Add both sides to the paned widget
    paned.set_start_child(Some(&left_scrolled));
    paned.set_end_child(Some(&right_box));

    // Set initial position to 50/50 split
    paned.set_position(400); // Default split position (will adjust based on window width)

    paned
}

// Helper function to open a file in a new tab or focus if already open
pub fn open_or_focus_tab(
    notebook: &Notebook,
    file_to_open: &PathBuf,
    content: &str,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    save_button: &Button,
    save_as_button: &Button,
    _mime_type: &mime_guess::Mime, // Used now for save menu button visibility
    window: &impl IsA<ApplicationWindow>, // Added for dialogs and NewTabDependencies
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    _save_menu_button: Option<&MenuButton>, // Added save_menu_button parameter
) {
    let filename = file_to_open
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());

    // Log opening operation
    crate::status_log::log_info(&format!("Opening {}...", filename));

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

        // Refresh diagnostics panel when focusing a file
        crate::linter::ui::refresh_diagnostics_panel();

        // Focus the text area of the existing tab
        if let Some((text_view, _)) = get_text_view_and_buffer_for_page(notebook, page_num) {
            text_view.grab_focus();
            // Don't change last active area when just switching to existing tabs
            // The user's intent (file manager vs text editor) should remain unchanged
        }

        crate::status_log::log_success(&format!("Focused {}", filename));
    } else {
        // Get file MIME type
        let mut mime_type = mime_guess::from_path(file_to_open).first_or_octet_stream();

        // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
        // but should be treated as TypeScript files (text/plain)
        if let Some(ext) = file_to_open.extension() {
            if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                // Override MIME type for TypeScript files
                mime_type = mime_guess::mime::TEXT_PLAIN;
            }
        }

        let file_name = file_to_open
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // Create tab widget regardless of content type
        let (tab_widget, tab_actual_label, tab_close_button) =
            crate::ui::create_tab_widget(&file_name);

        // Add middle mouse click support for the tab
        crate::ui::setup_tab_middle_click(&tab_widget, &tab_close_button);

        // Add right-click context menu support for the tab
        // Create NewTabDependencies for right-click menu
        let new_tab_deps_for_context = NewTabDependencies {
            editor_notebook: notebook.clone(),
            active_tab_path: active_tab_path_ref.clone(),
            file_path_manager: file_path_manager.clone(),
            window: window.clone().upcast::<ApplicationWindow>(),
            file_list_box: file_list_box.clone(),
            current_dir: current_dir.clone(),
            save_button: save_button.clone(),
            save_as_button: save_as_button.clone(),
            _save_menu_button: _save_menu_button.cloned(),
        };
        crate::ui::setup_tab_right_click(
            &tab_widget,
            notebook,
            &window.clone().upcast::<ApplicationWindow>(),
            file_path_manager,
            active_tab_path_ref,
            current_dir,
            file_list_box,
            Some(new_tab_deps_for_context),
        );

        // Check if this is an SVG file - handle with split view
        let is_svg = file_to_open
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase() == "svg")
            .unwrap_or(false);

        if is_svg {
            // Create split view for SVG (code on left, preview on right)
            let svg_paned =
                create_svg_split_view(content, file_to_open, &tab_actual_label, &file_name);

            // Add the paned widget directly to the notebook (no scrolled window wrapper)
            let new_page_num = notebook.append_page(&svg_paned, Some(&tab_widget));
            notebook.set_current_page(Some(new_page_num));

            // Update state
            println!("Adding SVG file to HashMap: page {} = {:?}", new_page_num, file_to_open.file_name());
            file_path_manager
                .borrow_mut()
                .insert(new_page_num, file_to_open.clone());
            *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());

            // Log successful opening
            crate::status_log::log_success(&format!("Opened {}", filename));

            // Connect close button
            let notebook_clone = notebook.clone();
            let window_clone = window.clone();
            let file_path_manager_clone = file_path_manager.clone();
            let active_tab_path_ref_clone = active_tab_path_ref.clone();

            let deps_for_new_tab_creation = NewTabDependencies {
                editor_notebook: notebook.clone(),
                active_tab_path: active_tab_path_ref_clone.clone(),
                file_path_manager: file_path_manager_clone.clone(),
                window: window_clone.clone().upcast::<ApplicationWindow>(),
                file_list_box: file_list_box.clone(),
                current_dir: current_dir.clone(),
                save_button: save_button.clone(),
                save_as_button: save_as_button.clone(),
                _save_menu_button: _save_menu_button.cloned(),
            };

            let svg_paned_for_close = svg_paned.clone();
            tab_close_button.connect_clicked(move |_| {
                if let Some(current_idx_for_this_tab) =
                    notebook_clone.page_num(&svg_paned_for_close)
                {
                    handle_close_tab_request(
                        &notebook_clone,
                        current_idx_for_this_tab,
                        &window_clone,
                        &file_path_manager_clone,
                        &active_tab_path_ref_clone,
                        &deps_for_new_tab_creation.current_dir,
                        &deps_for_new_tab_creation.file_list_box,
                        Some(deps_for_new_tab_creation.clone()),
                    );
                }
            });

            // Enable save buttons for SVG files (they're editable text)
            utils::update_save_buttons_visibility(
                save_button,
                save_as_button,
                Some(mime_guess::mime::TEXT_PLAIN),
            );
            if let Some(save_menu_btn) = _save_menu_button {
                utils::update_save_menu_button_visibility(
                    save_menu_btn,
                    Some(mime_guess::mime::TEXT_PLAIN),
                );
            }

            return; // Exit early for SVG files
        }

        // Check if this is a Markdown file - handle with split view
        let is_markdown = file_to_open
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| {
                let ext = s.to_lowercase();
                ext == "md" || ext == "markdown"
            })
            .unwrap_or(false);

        if is_markdown {
            // Create split view for Markdown (code on left, preview on right)
            let markdown_paned =
                create_markdown_split_view(content, file_to_open, &tab_actual_label, &file_name);

            // Add the paned widget directly to the notebook (no scrolled window wrapper)
            let new_page_num = notebook.append_page(&markdown_paned, Some(&tab_widget));
            notebook.set_current_page(Some(new_page_num));

            // Update state
            println!("Adding Markdown file to HashMap: page {} = {:?}", new_page_num, file_to_open.file_name());
            file_path_manager
                .borrow_mut()
                .insert(new_page_num, file_to_open.clone());
            *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());

            // Log successful opening
            crate::status_log::log_success(&format!("Opened {}", filename));

            // Connect close button
            let notebook_clone = notebook.clone();
            let window_clone = window.clone();
            let file_path_manager_clone = file_path_manager.clone();
            let active_tab_path_ref_clone = active_tab_path_ref.clone();

            let deps_for_new_tab_creation = NewTabDependencies {
                editor_notebook: notebook.clone(),
                active_tab_path: active_tab_path_ref_clone.clone(),
                file_path_manager: file_path_manager_clone.clone(),
                window: window_clone.clone().upcast::<ApplicationWindow>(),
                file_list_box: file_list_box.clone(),
                current_dir: current_dir.clone(),
                save_button: save_button.clone(),
                save_as_button: save_as_button.clone(),
                _save_menu_button: _save_menu_button.cloned(),
            };

            let markdown_paned_for_close = markdown_paned.clone();
            tab_close_button.connect_clicked(move |_| {
                if let Some(current_idx_for_this_tab) =
                    notebook_clone.page_num(&markdown_paned_for_close)
                {
                    handle_close_tab_request(
                        &notebook_clone,
                        current_idx_for_this_tab,
                        &window_clone,
                        &file_path_manager_clone,
                        &active_tab_path_ref_clone,
                        &deps_for_new_tab_creation.current_dir,
                        &deps_for_new_tab_creation.file_list_box,
                        Some(deps_for_new_tab_creation.clone()),
                    );
                }
            });

            // Enable save buttons for Markdown files (they're editable text)
            utils::update_save_buttons_visibility(
                save_button,
                save_as_button,
                Some(mime_guess::mime::TEXT_PLAIN),
            );
            if let Some(save_menu_btn) = _save_menu_button {
                utils::update_save_menu_button_visibility(
                    save_menu_btn,
                    Some(mime_guess::mime::TEXT_PLAIN),
                );
            }

            return; // Exit early for Markdown files
        }

        let new_scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .build();

        if mime_type.type_() == "image" {
            // Handle image file - check if it's a GIF animation
            let is_gif = file_to_open
                .extension()
                .and_then(|e| e.to_str())
                .map(|s| s.to_lowercase() == "gif")
                .unwrap_or(false);

            if is_gif {
                // Check file size to decide loading strategy
                let file_size = std::fs::metadata(file_to_open)
                    .map(|m| m.len())
                    .unwrap_or(0);

                if file_size > 10 * 1024 * 1024 {
                    // > 10MB
                    // Large GIF - load only first frame to prevent crash
                    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(file_to_open) {
                        let picture = Picture::new();
                        picture.set_pixbuf(Some(&pixbuf));
                        picture.set_can_focus(true);
                        picture.set_focusable(true);

                        // Show warning label
                        let info_label = Label::new(Some(&format!(
                            "GIF is {:.1} MB - showing first frame only to prevent crash",
                            file_size as f64 / (1024.0 * 1024.0)
                        )));
                        info_label.add_css_class("dim-label");

                        let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 5);
                        vbox.set_margin_top(10);
                        vbox.set_margin_bottom(10);
                        vbox.append(&info_label);
                        vbox.append(&picture);

                        // Add keyboard event handler
                        let key_controller = EventControllerKey::new();
                        let picture_keys = picture.clone();

                        key_controller.connect_key_pressed(
                            move |_controller, key, _code, _modifier| {
                                match key {
                                    gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                                        enter_image_fullscreen(&picture_keys);
                                        return glib::Propagation::Stop;
                                    }
                                    _ => {}
                                }
                                glib::Propagation::Proceed
                            },
                        );
                        new_scrolled_window.add_controller(key_controller);

                        // Add double-click gesture
                        let double_click_gesture = GestureClick::new();
                        double_click_gesture.set_button(1);
                        let picture_fullscreen = picture.clone();

                        double_click_gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
                            if n_press == 2 {
                                enter_image_fullscreen(&picture_fullscreen);
                            }
                        });
                        picture.add_controller(double_click_gesture);

                        new_scrolled_window.set_child(Some(&vbox));
                    } else {
                        let error_label = Label::new(Some("Failed to load large GIF"));
                        error_label.add_css_class("error");
                        new_scrolled_window.set_child(Some(&error_label));
                    }
                } else {
                    // Normal/small GIF - animate it
                    let loading_label = Label::new(Some("Loading GIF..."));
                    loading_label.add_css_class("dim-label");
                    new_scrolled_window.set_child(Some(&loading_label));

                    let file_path_clone = file_to_open.clone();
                    let new_scrolled_window_clone = new_scrolled_window.clone();

                    // Load with idle priority to keep UI responsive
                    glib::idle_add_local_once(move || {
                        match gtk4::gdk_pixbuf::PixbufAnimation::from_file(&file_path_clone) {
                            Ok(animation) => {
                                let picture = Picture::new();

                                if animation.is_static_image() {
                                    // Static image
                                    if let Some(pixbuf) = animation.static_image() {
                                        picture.set_pixbuf(Some(&pixbuf));
                                    }
                                } else {
                                    // Animated GIF - use native timing
                                    if let Some(pixbuf) = animation.static_image() {
                                        picture.set_pixbuf(Some(&pixbuf));
                                    }

                                    let picture_anim = picture.clone();
                                    let iter = animation.iter(None);
                                    let iter_rc = Rc::new(RefCell::new(iter));

                                    // Use GIF's native delay, capped at 60fps
                                    let delay_ms: u64 = {
                                        let iter_borrow = iter_rc.borrow();
                                        iter_borrow
                                            .delay_time()
                                            .map(|d| d.as_millis() as u64)
                                            .unwrap_or(100)
                                            .max(16) // Cap at 60fps (16ms)
                                    };

                                    // Animation loop
                                    glib::timeout_add_local(
                                        std::time::Duration::from_millis(delay_ms),
                                        clone!(
                                            #[weak]
                                            picture_anim,
                                            #[strong]
                                            iter_rc,
                                            #[upgrade_or]
                                            glib::ControlFlow::Break,
                                            move || {
                                                // Stop if widget destroyed or not mapped (tab not visible)
                                                if !picture_anim.is_visible()
                                                    || !picture_anim.is_mapped()
                                                {
                                                    return glib::ControlFlow::Break;
                                                }

                                                // Advance and display next frame
                                                use std::time::SystemTime;
                                                let iter = iter_rc.borrow_mut();
                                                iter.advance(SystemTime::now());
                                                let pixbuf = iter.pixbuf();
                                                drop(iter);

                                                picture_anim.set_pixbuf(Some(&pixbuf));

                                                glib::ControlFlow::Continue
                                            }
                                        ),
                                    );
                                }

                                picture.set_can_focus(true);
                                picture.set_focusable(true);

                                // Add keyboard event handler
                                let key_controller = EventControllerKey::new();
                                let picture_keys = picture.clone();

                                key_controller.connect_key_pressed(
                                    move |_controller, key, _code, _modifier| {
                                        match key {
                                            gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                                                enter_image_fullscreen(&picture_keys);
                                                return glib::Propagation::Stop;
                                            }
                                            _ => {}
                                        }
                                        glib::Propagation::Proceed
                                    },
                                );
                                new_scrolled_window_clone.add_controller(key_controller);

                                // Add double-click gesture
                                let double_click_gesture = GestureClick::new();
                                double_click_gesture.set_button(1);
                                let picture_fullscreen = picture.clone();

                                double_click_gesture.connect_pressed(
                                    move |_gesture, n_press, _x, _y| {
                                        if n_press == 2 {
                                            enter_image_fullscreen(&picture_fullscreen);
                                        }
                                    },
                                );
                                picture.add_controller(double_click_gesture);

                                new_scrolled_window_clone.set_child(Some(&picture));
                            }
                            Err(e) => {
                                let error_msg = format!("Failed to load GIF: {}", e);
                                let error_label = Label::new(Some(&error_msg));
                                error_label.add_css_class("error");
                                new_scrolled_window_clone.set_child(Some(&error_label));
                            }
                        }
                    });
                }
            } else {
                // Load as static image for non-GIF files
                if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(file_to_open) {
                    let picture = Picture::new();
                    picture.set_pixbuf(Some(&pixbuf));
                    picture.set_can_focus(true);
                    picture.set_focusable(true);

                    new_scrolled_window.set_child(Some(&picture));

                    // Add keyboard event handler on the scrolled window to catch F key
                    let key_controller = EventControllerKey::new();
                    let picture_keys = picture.clone();

                    key_controller.connect_key_pressed(
                        move |_controller, key, _code, _modifier| {
                            match key {
                                // F: Enter fullscreen
                                gtk4::gdk::Key::f | gtk4::gdk::Key::F => {
                                    println!("Image: F key pressed, entering fullscreen");
                                    enter_image_fullscreen(&picture_keys);
                                    return glib::Propagation::Stop;
                                }
                                _ => {}
                            }
                            glib::Propagation::Proceed
                        },
                    );
                    new_scrolled_window.add_controller(key_controller);

                    // Add double-click gesture for fullscreen
                    let double_click_gesture = GestureClick::new();
                    double_click_gesture.set_button(1); // Left mouse button
                    let picture_fullscreen = picture.clone();

                    double_click_gesture.connect_pressed(move |_gesture, n_press, _x, _y| {
                        if n_press == 2 {
                            // Double-click
                            println!("Image: Double-click detected, entering fullscreen");
                            enter_image_fullscreen(&picture_fullscreen);
                        }
                    });
                    picture.add_controller(double_click_gesture);
                } else {
                    // Failed to load image, show error
                    let error_msg = format!("Failed to load image: {}", file_name);
                    let error_label = Label::new(Some(&error_msg));
                    new_scrolled_window.set_child(Some(&error_label));
                }
            }
        } else if mime_type.type_() == "audio" {
            // Handle audio file
            match crate::audio::AudioPlayer::new(file_to_open) {
                Ok(audio_player) => {
                    new_scrolled_window.set_child(Some(&audio_player.widget));
                }
                Err(e) => {
                    // Failed to create audio player, show error
                    let error_msg = format!("Failed to load audio file {}: {:?}", file_name, e);
                    let error_label = Label::new(Some(&error_msg));
                    error_label.add_css_class("error");
                    new_scrolled_window.set_child(Some(&error_label));
                    crate::status_log::log_error(&error_msg);
                }
            }
        } else if mime_type.type_() == "video" {
            // Handle video file
            match crate::video::VideoPlayer::new(file_to_open) {
                Ok(video_player) => {
                    new_scrolled_window.set_child(Some(&video_player.widget));
                }
                Err(e) => {
                    // Failed to create video player, show error
                    let error_msg = format!("Failed to load video file {}: {:?}", file_name, e);
                    let error_label = Label::new(Some(&error_msg));
                    error_label.add_css_class("error");
                    new_scrolled_window.set_child(Some(&error_label));
                    crate::status_log::log_error(&error_msg);
                }
            }
        } else if utils::is_allowed_mime_type(&mime_type) {
            // Handle text file - use cached file reading for performance
            // Create source view with syntax highlighting
            let (source_view, source_buffer) = crate::syntax::create_source_view();
            
            // Setup breakpoints
            crate::debugger::breakpoints::setup_breakpoint_attributes(&source_view);
            crate::debugger::breakpoints::setup_gutter_click_handler(
                &source_view, 
                notebook.clone(), 
                file_path_manager.clone()
            );
            crate::debugger::breakpoints::update_visual_breakpoints(&source_view, file_to_open);

            source_buffer.set_text(content);

            // Apply syntax highlighting based on file extension
            crate::syntax::set_language_for_file(&source_buffer, file_to_open);

            // Apply diagnostic underlines if there are any stored for this file
            crate::linter::apply_diagnostic_underlines(&source_buffer, &file_to_open.to_string_lossy());

            // Register buffer and view for future diagnostic updates
            crate::linter::ui::register_buffer_for_diagnostics(file_to_open, &source_buffer, &source_view);

            // Setup completion for the specific file type
            crate::completion::setup_completion_for_file(&source_view, Some(file_to_open));

            // Setup keyboard shortcuts for completion
            crate::completion::setup_completion_shortcuts(&source_view);

            // Setup linting for the file
            crate::linter::ui::setup_linting(&source_view, Some(file_to_open));

            // Ctrl+Click on an underlined diagnostic focuses corresponding entry in diagnostics panel
            {
                let source_view_for_gesture = source_view.clone();
                let file_path_for_gesture = file_to_open.to_string_lossy().to_string();
                let gesture = gtk4::GestureClick::new();
                gesture.set_button(1); // Left click only
                gesture.set_propagation_phase(gtk4::PropagationPhase::Bubble);
                gesture.connect_pressed(move |g, _n_press, x, y| {
                    let state = g.current_event_state();
                    if state.contains(gtk4::gdk::ModifierType::CONTROL_MASK) {
                        // Determine line at click position
                        let buffer = source_view_for_gesture.buffer();
                        // Convert widget coords to buffer coords
                        let (bx, by) = source_view_for_gesture.window_to_buffer_coords(
                            gtk4::TextWindowType::Widget,
                            x as i32,
                            y as i32,
                        );
                        // Get iter at location; API returns Option<TextIter>
                        let line = if let Some(iter) = source_view_for_gesture.iter_at_location(bx, by) {
                            iter.line() + 1
                        } else {
                            // Fallback to current cursor position
                            let insert_mark = buffer.get_insert();
                            let cursor_iter = buffer.iter_at_mark(&insert_mark);
                            cursor_iter.line() + 1
                        }; // convert to 1-based
                        // Ensure diagnostics panel and terminal area are visible
                        crate::linter::ui::show_diagnostics_panel();
                        // Also trigger the show-diagnostics window action so the terminal notebook becomes visible if hidden
                        if let Some(window) = source_view_for_gesture.root().and_then(|r| r.downcast::<gtk4::ApplicationWindow>().ok()) {
                            // Fire the same action bound to menu/shortcuts (ignores if already visible)
                            let _ = gtk4::prelude::WidgetExt::activate_action(&window, "win.show-diagnostics", None);
                        }
                        crate::linter::diagnostics_panel::focus_diagnostic(&file_path_for_gesture, line as usize);
                    }
                });
                source_view.add_controller(gesture);
            }

            // Set up interaction tracking for the text editor
            let text_view = source_view.clone().upcast::<TextView>();
            setup_text_editor_interaction_tracking(&text_view);

            // Get TextBuffer interfaces for compatibility with the rest of the code
            let new_text_buffer = source_buffer.upcast::<TextBuffer>();

            // Set the source view as the child of the scrolled window
            new_scrolled_window.set_child(Some(&source_view));

            // Optimized dirty tracking - avoid string cloning
            let tab_actual_label_weak = tab_actual_label.downgrade();
            let file_name_ref = file_name.clone(); // Only clone once
            new_text_buffer.connect_changed(move |_buffer| {
                // Mark text editor as active when user actually types/modifies content
                LAST_ACTIVE_AREA.with(|area| {
                    *area.borrow_mut() = LastActiveArea::TextEditor;
                });

                if let Some(label) = tab_actual_label_weak.upgrade() {
                    let current_text = label.text();
                    if !current_text.starts_with('*') {
                        label.set_text(&format!("*{}", file_name_ref));
                        crate::status_log::log_info(&format!("{} modified", file_name_ref));
                    }
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

        // Focus the text area of the newly opened file if it's a text file
        if utils::is_allowed_mime_type(&mime_type) {
            if let Some((text_view, _)) = get_text_view_and_buffer_for_page(notebook, new_page_num)
            {
                text_view.grab_focus();
                // Don't change last active area when opening files from file manager
                // The user's intent (file manager vs text editor) should remain unchanged
            }
        }

        // Update state
        println!("Adding text file to HashMap: page {} = {:?}", new_page_num, file_to_open.file_name());
        file_path_manager
            .borrow_mut()
            .insert(new_page_num, file_to_open.clone());
        *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());

        // Log successful opening
        crate::status_log::log_success(&format!("Opened {}", filename));

        // Refresh diagnostics panel when opening a file
        crate::linter::ui::refresh_diagnostics_panel();

        // Connect close button
        let notebook_clone = notebook.clone();
        let window_clone = window.clone();
        let file_path_manager_clone = file_path_manager.clone();
        let active_tab_path_ref_clone = active_tab_path_ref.clone();

        let deps_for_new_tab_creation = NewTabDependencies {
            editor_notebook: notebook.clone(),
            active_tab_path: active_tab_path_ref_clone.clone(),
            file_path_manager: file_path_manager_clone.clone(),
            window: window_clone.clone().upcast::<ApplicationWindow>(),
            file_list_box: file_list_box.clone(),
            current_dir: current_dir.clone(),
            save_button: save_button.clone(),
            save_as_button: save_as_button.clone(),
            _save_menu_button: _save_menu_button.cloned(), // Pass the save menu button if available
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
                    Some(deps_for_new_tab_creation.clone()),
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
    window: &impl IsA<ApplicationWindow>, // Accept any type that implements ApplicationWindow
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    editor_notebook: &Notebook,
    error_label: &Label,
    picture: &Picture,
    up_button: &Button,
    file_list_box_clone: &ListBox, // This is likely the same as file_list_box, ensure it's used consistently
    _save_menu_button: Option<&MenuButton>, // Prefix with underscore to acknowledge it's unused
    path_box: Option<&gtk4::Box>,  // Optional path box for status bar
    current_selection_source: &Rc<RefCell<utils::FileSelectionSource>>, // Track selection source for click-outside detection
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
        path_box,
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
        window,                   // Pass window
        _save_menu_button,        // Pass save_menu_button with the renamed parameter
        path_box,                 // Pass the path box for status bar with clickable segments
        current_selection_source, // Pass the selection source tracker for click-outside detection
    );

    // These handlers likely don't need direct access to the editor_notebook content itself
    // but might influence which file is considered "active" if that logic is centralized.
    setup_up_button_handler(
        up_button,
        current_dir,
        file_list_box,
        active_tab_path,
        path_box,
    ); // Pass active_tab_path and path_box
}

fn setup_new_button_handler(
    new_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    file_list_box: &ListBox,            // To update file list selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list
    save_button: &Button,
    save_as_button: &Button,
    window: &impl IsA<ApplicationWindow>, // Added for NewTabDependencies
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
        // Use the modern create_new_empty_tab function which creates SourceView widgets
        // that are properly found by the theme update system
        let deps_for_new_tab_creation = NewTabDependencies {
            editor_notebook: editor_notebook_clone.clone(),
            active_tab_path: active_tab_path_ref_clone.clone(),
            file_path_manager: file_path_manager_clone.clone(),
            window: window_clone.clone().upcast::<ApplicationWindow>(),
            file_list_box: file_list_box_clone.clone(),
            current_dir: current_dir_clone.clone(),
            save_button: save_button_clone.clone(),
            save_as_button: save_as_button_clone.clone(),
            _save_menu_button: None, // We don't have a menu button in this scope
        };

        // Create the new tab using the modern system that creates SourceView widgets
        create_new_empty_tab(&deps_for_new_tab_creation);
    });
}

fn setup_open_button_handler(
    open_button: &Button,
    editor_notebook: &Notebook,
    window: &impl IsA<ApplicationWindow>,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    error_label: &Label, // For showing errors if a tab can't display content
    picture: &Picture,   // For image files - this needs to be rethought for tabs
    save_button: &Button,
    save_as_button: &Button,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    _save_menu_button: Option<&MenuButton>, // Renamed with underscore to acknowledge it's unused
    path_box: Option<&gtk4::Box>, // Optional path box for status bar with clickable segments
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
    let path_box = path_box.cloned(); // Clone the optional path box

    open_button.connect_clicked(move |_| {
        crate::status_log::log_info("Opening file dialog...");

        let dialog = gtk4::FileChooserDialog::new(
            Some("Open File"),
            Some(window.as_ref().upcast_ref::<gtk4::Window>()),
            gtk4::FileChooserAction::Open,
            &[
                ("Cancel", gtk4::ResponseType::Cancel),
                ("Open", gtk4::ResponseType::Accept),
            ],
        );

        dialog.set_default_response(gtk4::ResponseType::Cancel);

        let current_dialog_dir_path = current_dir.borrow().clone();
        // Explicitly type annotation for gio_file_result and wrap the call in Ok()
        let gio_file_result: Result<gtk4::gio::File, glib::Error> =
            Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
        match gio_file_result {
            Ok(gfile) => {
                if current_dialog_dir_path.is_dir() {
                    let _ = dialog.set_current_folder(Some(&gfile));
                } else if let Some(parent_gfile) = gfile.parent() {
                    let _ = dialog.set_current_folder(Some(&parent_gfile));
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to create GFile for path {:?}: {}",
                    current_dialog_dir_path, e
                );
            }
        }

        let editor_notebook_clone = editor_notebook.clone();
        let current_dir_clone = current_dir.clone();
        let file_list_box_clone = file_list_box.clone();
        let _error_label_clone = error_label.clone();
        let _picture_clone = picture.clone();
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
        let _path_box_for_response = path_box.clone(); // Clone path_box for the inner closure (unused but kept for future use)

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file_to_open) = dialog.file().and_then(|f| f.path()) {
                    // Close any empty untitled tabs before opening the file
                    close_empty_untitled_tabs(
                        &editor_notebook_clone,
                        &file_path_manager_for_response,
                    );

                    let mime_type = mime_guess::from_path(&file_to_open).first_or_octet_stream();
                    if utils::is_allowed_mime_type(&mime_type) {
                        match std::fs::read_to_string(&file_to_open) {
                            Ok(content) => {
                                open_or_focus_tab(
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
                                    let parent_path = parent.to_path_buf();
                                    *current_dir_clone.borrow_mut() = parent_path.clone();
                                    utils::update_file_list(
                                        &file_list_box_clone,
                                        &current_dir_clone.borrow(),
                                        &active_tab_path_ref_for_response.borrow(),
                                        utils::FileSelectionSource::TabSwitch,
                                    );
                                }

                                // Log successful file opening
                                let filename = file_to_open
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "file".to_string());
                                crate::status_log::log_success(&format!("Opened {}", filename));
                            }
                            Err(e) => {
                                let filename = file_to_open
                                    .file_name()
                                    .map(|name| name.to_string_lossy().into_owned())
                                    .unwrap_or_else(|| "file".to_string());
                                crate::status_log::log_error(&format!(
                                    "Failed to read {}: {}",
                                    filename, e
                                ));
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
                            utils::update_file_list(
                                &file_list_box_clone,
                                &current_dir_clone.borrow(),
                                &active_tab_path_ref_for_response.borrow(),
                                utils::FileSelectionSource::TabSwitch,
                            );
                        }

                        // Log successful image opening
                        let filename = file_to_open
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "image".to_string());
                        crate::status_log::log_success(&format!("Opened {}", filename));
                    } else if mime_type.type_() == "audio" {
                        // For audio files, use open_or_focus_tab with empty content
                        open_or_focus_tab(
                            &editor_notebook_clone,
                            &file_to_open,
                            "", // Empty content for audio files
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
                            utils::update_file_list(
                                &file_list_box_clone,
                                &current_dir_clone.borrow(),
                                &active_tab_path_ref_for_response.borrow(),
                                utils::FileSelectionSource::TabSwitch,
                            );
                        }

                        // Log successful audio opening
                        let filename = file_to_open
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "audio".to_string());
                        crate::status_log::log_success(&format!("Opened {}", filename));
                    } else if mime_type.type_() == "video" {
                        // For video files, use open_or_focus_tab with empty content
                        open_or_focus_tab(
                            &editor_notebook_clone,
                            &file_to_open,
                            "", // Empty content for video files
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
                            utils::update_file_list(
                                &file_list_box_clone,
                                &current_dir_clone.borrow(),
                                &active_tab_path_ref_for_response.borrow(),
                                utils::FileSelectionSource::TabSwitch,
                            );
                        }

                        // Log successful video opening
                        let filename = file_to_open
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "video".to_string());
                        crate::status_log::log_success(&format!("Opened {}", filename));
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
                            utils::update_file_list(
                                &file_list_box_clone,
                                &current_dir_clone.borrow(),
                                &active_tab_path_ref_for_response.borrow(),
                                utils::FileSelectionSource::TabSwitch,
                            );
                        }

                        // Log successful file opening (even for unsupported types)
                        let filename = file_to_open
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "file".to_string());
                        crate::status_log::log_success(&format!("Opened {}", filename));
                    }
                } else {
                    // No file was selected despite Accept response
                    crate::status_log::log_error("No file selected");
                }
            } else if response == gtk4::ResponseType::Cancel {
                // Only log cancellation if it's explicitly a cancel response
                crate::status_log::log_info("File open cancelled");
            }
            // Don't log anything for other response types (like dialog close events)
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
    window: &impl IsA<ApplicationWindow>,
    file_list_box: &ListBox,            // To update selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list path
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();

    save_button.connect_clicked(move |_| {
        // Log save operation start
        crate::status_log::log_info("Saving file...");

        if let Some((_active_text_view, active_buffer)) =
            get_active_text_view_and_buffer(&editor_notebook)
        {
            // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() {
                crate::status_log::log_error("No active tab found");
                return;
            }
            let current_page_num = current_page_num_opt.unwrap();

            let path_to_save_opt = file_path_manager.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                let filename = path_to_save
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "file".to_string());

                let mut mime_type = mime_guess::from_path(&path_to_save).first_or_octet_stream();

                // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
                // but should be treated as TypeScript files (text/plain)
                if let Some(ext) = path_to_save.extension() {
                    if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                        // Override MIME type for TypeScript files
                        mime_type = mime_guess::mime::TEXT_PLAIN;
                    }
                }

                if utils::is_allowed_mime_type(&mime_type) {
                    match File::create(&path_to_save) {
                        Ok(mut file) => {
                            let text = active_buffer.text(
                                &active_buffer.start_iter(),
                                &active_buffer.end_iter(),
                                false,
                            );
                            match file.write_all(text.as_bytes()) {
                                Ok(_) => {
                                    // Update tab label (remove *)
                                    update_tab_label_after_save(
                                        &editor_notebook,
                                        current_page_num,
                                        Some(
                                            &path_to_save
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy(),
                                        ),
                                        false,
                                    );

                                    // Apply syntax highlighting based on file extension
                                    apply_syntax_highlighting_after_save(
                                        &editor_notebook,
                                        current_page_num,
                                        &path_to_save,
                                    );

                                    // Notify LSP that file was saved (for rust-analyzer)
                                    crate::linter::ui::notify_file_saved(&path_to_save);

                                    // Trigger git status update after save
                                    crate::ui::git_diff::trigger_git_status_update();

                                    crate::status_log::log_success(&format!("Saved {}", filename));
                                }
                                Err(e) => {
                                    crate::status_log::log_error(&format!(
                                        "Failed to write {}: {}",
                                        filename, e
                                    ));
                                }
                            }
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!(
                                "Failed to create {}: {}",
                                filename, e
                            ));
                        }
                    }
                } else {
                    crate::status_log::log_error("File type not supported for saving");
                }
            } else {
                // No path associated, treat as "Save As"
                // This logic should ideally call a shared "save_as" function
                crate::status_log::log_info("Opening Save As dialog...");

                let dialog = gtk4::FileChooserDialog::new(
                    Some("Save File"),
                    Some(window.as_ref().upcast_ref::<gtk4::Window>()),
                    gtk4::FileChooserAction::Save,
                    &[
                        ("Cancel", gtk4::ResponseType::Cancel),
                        ("Save", gtk4::ResponseType::Accept),
                    ],
                );

                dialog.set_default_response(gtk4::ResponseType::Cancel);

                // Set current folder to match the file manager's current directory
                let current_dialog_dir_path = current_dir.borrow().clone();
                let gio_file_result: Result<gtk4::gio::File, glib::Error> =
                    Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
                match gio_file_result {
                    Ok(gfile) => {
                        if current_dialog_dir_path.is_dir() {
                            let _ = dialog.set_current_folder(Some(&gfile));
                        } else if let Some(parent_gfile) = gfile.parent() {
                            let _ = dialog.set_current_folder(Some(&parent_gfile));
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "Failed to create GFile for path {:?}: {}",
                            current_dialog_dir_path, e
                        );
                    }
                }

                let editor_notebook_clone = editor_notebook.clone();
                let active_tab_path_ref_clone = active_tab_path_ref.clone();
                let file_path_manager_clone = file_path_manager.clone();
                let file_list_box_clone = file_list_box.clone();
                let current_dir_clone = current_dir.clone();

                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file().and_then(|f| f.path()) {
                            let filename = file
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_else(|| "file".to_string());

                            match File::create(&file) {
                                Ok(mut f_obj) => {
                                    let text = active_buffer.text(
                                        &active_buffer.start_iter(),
                                        &active_buffer.end_iter(),
                                        false,
                                    );
                                    match f_obj.write_all(text.as_bytes()) {
                                        Ok(_) => {
                                            file_path_manager_clone
                                                .borrow_mut()
                                                .insert(current_page_num, file.clone());
                                            *active_tab_path_ref_clone.borrow_mut() =
                                                Some(file.clone());
                                            // Update tab label
                                            update_tab_label_after_save(
                                                &editor_notebook_clone,
                                                current_page_num,
                                                Some(
                                                    &file
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy(),
                                                ),
                                                false,
                                            );

                                            // Apply syntax highlighting based on file extension
                                            apply_syntax_highlighting_after_save(
                                                &editor_notebook_clone,
                                                current_page_num,
                                                &file,
                                            );

                                            // Notify LSP that file was saved (for rust-analyzer)
                                            crate::linter::ui::notify_file_saved(&file);

                                            // Trigger git status update after save
                                            crate::ui::git_diff::trigger_git_status_update();

                                            // Update main window title potentially
                                            if let Some(parent) = file.parent() {
                                                *current_dir_clone.borrow_mut() =
                                                    parent.to_path_buf();
                                            }
                                            utils::update_file_list(
                                                &file_list_box_clone,
                                                &current_dir_clone.borrow(),
                                                &active_tab_path_ref_clone.borrow(),
                                                utils::FileSelectionSource::TabSwitch,
                                            );

                                            crate::status_log::log_success(&format!(
                                                "Saved as {}",
                                                filename
                                            ));
                                        }
                                        Err(e) => {
                                            crate::status_log::log_error(&format!(
                                                "Failed to write {}: {}",
                                                filename, e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    crate::status_log::log_error(&format!(
                                        "Failed to create {}: {}",
                                        filename, e
                                    ));
                                }
                            }
                        }
                    } else {
                        crate::status_log::log_info("Save cancelled");
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
    window: &impl IsA<ApplicationWindow>,
    current_dir: &Rc<RefCell<PathBuf>>, // To set initial dialog directory and update after save
    file_list_box: &ListBox,            // To update file list
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();

    save_as_button.connect_clicked(move |_| {
        crate::status_log::log_info("Opening Save As dialog...");

        if let Some((_active_text_view, active_buffer)) =
            get_active_text_view_and_buffer(&editor_notebook)
        {
            // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() {
                crate::status_log::log_error("No active tab found");
                return;
            }
            let current_page_num = current_page_num_opt.unwrap();

            let dialog = gtk4::FileChooserDialog::new(
                Some("Save File As"),
                Some(window.as_ref().upcast_ref::<gtk4::Window>()),
                gtk4::FileChooserAction::Save,
                &[
                    ("Cancel", gtk4::ResponseType::Cancel),
                    ("Save As", gtk4::ResponseType::Accept),
                ],
            );

            dialog.set_default_response(gtk4::ResponseType::Cancel);

            let current_dialog_dir_path = current_dir.borrow().clone();
            // Explicitly type annotation for gio_file_result and wrap the call in Ok()
            let gio_file_result: Result<gtk4::gio::File, glib::Error> =
                Ok(gtk4::gio::File::for_path(&current_dialog_dir_path));
            match gio_file_result {
                Ok(gfile) => {
                    if current_dialog_dir_path.is_dir() {
                        let _ = dialog.set_current_folder(Some(&gfile));
                    } else if let Some(parent_gfile) = gfile.parent() {
                        let _ = dialog.set_current_folder(Some(&parent_gfile));
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Failed to create GFile for path {:?}: {}",
                        current_dialog_dir_path, e
                    );
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
                        let filename = file_to_save
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "file".to_string());

                        let mut mime_type =
                            mime_guess::from_path(&file_to_save).first_or_octet_stream();

                        // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
                        // but should be treated as TypeScript files (text/plain)
                        if let Some(ext) = file_to_save.extension() {
                            if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                                // Override MIME type for TypeScript files
                                mime_type = mime_guess::mime::TEXT_PLAIN;
                            }
                        }

                        if utils::is_allowed_mime_type(&mime_type) {
                            match File::create(&file_to_save) {
                                Ok(mut f_obj) => {
                                    let text = active_buffer.text(
                                        &active_buffer.start_iter(),
                                        &active_buffer.end_iter(),
                                        false,
                                    );
                                    match f_obj.write_all(text.as_bytes()) {
                                        Ok(_) => {
                                            file_path_manager_clone
                                                .borrow_mut()
                                                .insert(current_page_num, file_to_save.clone());
                                            *active_tab_path_ref_clone.borrow_mut() =
                                                Some(file_to_save.clone());

                                            // Update tab label
                                            update_tab_label_after_save(
                                                &editor_notebook_clone,
                                                current_page_num,
                                                Some(
                                                    &file_to_save
                                                        .file_name()
                                                        .unwrap_or_default()
                                                        .to_string_lossy(),
                                                ),
                                                false,
                                            );

                                            // Apply syntax highlighting based on file extension
                                            apply_syntax_highlighting_after_save(
                                                &editor_notebook_clone,
                                                current_page_num,
                                                &file_to_save,
                                            );

                                            // Notify LSP that file was saved (for rust-analyzer)
                                            crate::linter::ui::notify_file_saved(&file_to_save);

                                            // Trigger git status update after save
                                            crate::ui::git_diff::trigger_git_status_update();

                                            if let Some(parent) = file_to_save.parent() {
                                                *current_dir_clone.borrow_mut() =
                                                    parent.to_path_buf();
                                            }
                                            utils::update_file_list(
                                                &file_list_box_clone,
                                                &current_dir_clone.borrow(),
                                                &active_tab_path_ref_clone.borrow(),
                                                utils::FileSelectionSource::TabSwitch,
                                            );

                                            crate::status_log::log_success(&format!(
                                                "Saved as {}",
                                                filename
                                            ));
                                        }
                                        Err(e) => {
                                            crate::status_log::log_error(&format!(
                                                "Failed to write {}: {}",
                                                filename, e
                                            ));
                                        }
                                    }
                                }
                                Err(e) => {
                                    crate::status_log::log_error(&format!(
                                        "Failed to create {}: {}",
                                        filename, e
                                    ));
                                }
                            }
                        } else {
                            crate::status_log::log_error("File type not supported for saving");
                        }
                    }
                } else {
                    crate::status_log::log_info("Save As cancelled");
                }
                d.close();
            });
            dialog.show();
        }
    });
}

/// Enum to track which area was last actively used
#[derive(Clone, Debug, PartialEq)]
pub enum LastActiveArea {
    TextEditor,
    FileManager,
}

// Global state to track the last active area
thread_local! {
    pub static LAST_ACTIVE_AREA: RefCell<LastActiveArea> = const { RefCell::new(LastActiveArea::TextEditor) };
}

/// Adds interaction tracking to a text view to detect when user actively uses it
pub fn setup_text_editor_interaction_tracking(text_view: &gtk4::TextView) {
    // Add click gesture to detect when user clicks in the text editor
    let click_gesture = GestureClick::new();
    click_gesture.set_button(1); // Left mouse button

    click_gesture.connect_pressed(move |_, _, _, _| {
        // Mark text editor as active when user clicks in it
        LAST_ACTIVE_AREA.with(|area| {
            let current_area = area.borrow().clone();
            if current_area != LastActiveArea::TextEditor {
                *area.borrow_mut() = LastActiveArea::TextEditor;
                println!("DEBUG: Text editor clicked - set as last active area");
            }
        });
    });

    text_view.add_controller(click_gesture);

    // Note: Removed key press handler as it was interfering with clipboard operations
    // Text editor will become active through:
    // 1. Clicking in the text editor (above)
    // 2. Typing/editing text (buffer change handlers)
}

/// Checks if any text editor in the notebook currently has focus
/// Returns true if a text editor (SourceView) has focus, false otherwise
#[allow(dead_code)]
fn is_text_editor_focused(notebook: &gtk4::Notebook) -> bool {
    if let Some(current_page_num) = notebook.current_page() {
        if let Some((text_view, _)) = get_text_view_and_buffer_for_page(notebook, current_page_num)
        {
            let has_focus = text_view.has_focus();
            let is_focus = text_view.is_focus();
            println!(
                "DEBUG: Text editor focus check - has_focus: {}, is_focus: {}",
                has_focus, is_focus
            );
            return has_focus;
        }
    }
    println!("DEBUG: No text view found or no current page");
    false
}

/// Checks if the file manager should handle clipboard operations
/// Returns true if file manager operations should be prioritized over text operations
#[allow(dead_code)]
fn should_handle_file_operations(notebook: &gtk4::Notebook, file_list_box: &gtk4::ListBox) -> bool {
    let text_editor_has_focus = is_text_editor_focused(notebook);
    let file_list_has_focus = file_list_box.has_focus();
    let file_selected = file_list_box.selected_row().is_some();

    // Get the last active area
    let last_active = LAST_ACTIVE_AREA.with(|area| area.borrow().clone());

    println!(
        "DEBUG: Focus check - Text editor: {}, File list: {}, File selected: {}, Last active: {:?}",
        text_editor_has_focus, file_list_has_focus, file_selected, last_active
    );

    // If the file manager was the last active area and has a selection, prioritize file operations
    // This handles the case where files are double-clicked (which opens them and gives focus to text editor)
    // but the user's intent was to work with files
    if last_active == LastActiveArea::FileManager && file_selected {
        println!(
            "DEBUG: File manager was last active with file selected - handling file operations"
        );
        return true;
    }

    // If the file list box has focus, definitely handle file operations
    if file_list_has_focus {
        println!("DEBUG: File list has focus - handling file operations");
        return true;
    }

    // If a text editor has focus and was the last active area, let text operations proceed
    if text_editor_has_focus && last_active == LastActiveArea::TextEditor {
        println!(
            "DEBUG: Text editor has focus and was last active - letting text operations proceed"
        );
        return false;
    }

    // If there's a selected row in the file list and no text editor has focus, handle file operations
    if file_selected && !text_editor_has_focus {
        println!("DEBUG: File selected but no text editor focus - handling file operations");
        return true;
    }

    // Default: if text editor has focus, let it handle operations
    if text_editor_has_focus {
        println!("DEBUG: Text editor has focus - letting text operations proceed");
        return false;
    }

    // Final default: handle file operations
    println!("DEBUG: Default case - handling file operations");
    true
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
    window: &impl IsA<ApplicationWindow>, // Added for NewTabDependencies
    _save_menu_button: Option<&MenuButton>, // Prefix with _ to acknowledge it's unused currently
    path_box: Option<&gtk4::Box>, // Optional path box for status bar with clickable segments
    current_selection_source: &Rc<RefCell<utils::FileSelectionSource>>, // Track selection source for click-outside detection
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
    let save_menu_button_option = _save_menu_button.cloned();
    // Clone the path box option
    let path_box_option = path_box.cloned();
    // Clone the selection source tracker
    let current_selection_source_clone = current_selection_source.clone();

    // Add keyboard support for file operations - attach to file list box so it only handles when file list has focus
    let key_controller = EventControllerKey::new();

    let file_list_box_for_key = file_list_box.clone();
    let editor_notebook_for_key = editor_notebook.clone();
    let active_tab_path_for_key = active_tab_path_ref.clone();
    let file_path_manager_for_key = file_path_manager.clone();
    let current_dir_for_key = current_dir.clone();
    let window_for_key = window.clone();

    key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
        let ctrl_pressed = state.contains(gtk4::gdk::ModifierType::CONTROL_MASK);

        // Handle Ctrl+C, Ctrl+X, Ctrl+V for file operations when file list has focus
        if ctrl_pressed {
            match keyval {
                // Ctrl+C: Copy file
                gtk4::gdk::Key::c => {
                    println!("DEBUG: Ctrl+C pressed in file list");

                    // First priority: Check if there's a selected row in the file list
                    if let Some(selected_row) = file_list_box_for_key.selected_row() {
                        if let Some(label) = selected_row
                            .child()
                            .and_then(|c| c.downcast::<Label>().ok())
                        {
                            let file_name = label.text();
                            let mut file_path = current_dir_for_key.borrow().clone();
                            file_path.push(file_name.as_str());

                            if file_path.is_file() {
                                crate::ui::file_manager::copy_file_to_clipboard(&file_path);
                                // Refresh file list to show visual changes
                                crate::utils::update_file_list(
                                    &file_list_box_for_key,
                                    &current_dir_for_key.borrow(),
                                    &active_tab_path_for_key.borrow(),
                                    crate::utils::FileSelectionSource::TabSwitch,
                                );
                            }
                        }
                    }
                    // Second priority: Check if there's an active tab with a file in current directory
                    else if let Some(active_file) = active_tab_path_for_key.borrow().as_ref() {
                        if active_file.parent() == Some(current_dir_for_key.borrow().as_path())
                            && active_file.is_file()
                        {
                            crate::ui::file_manager::copy_file_to_clipboard(active_file);
                            // Refresh file list to show visual changes
                            crate::utils::update_file_list(
                                &file_list_box_for_key,
                                &current_dir_for_key.borrow(),
                                &active_tab_path_for_key.borrow(),
                                crate::utils::FileSelectionSource::TabSwitch,
                            );
                        }
                    }
                    // Always stop propagation when file list has focus
                    return glib::Propagation::Stop;
                }
                // Ctrl+X: Cut file
                gtk4::gdk::Key::x => {
                    println!("DEBUG: Ctrl+X pressed in file list");

                    // First priority: Check if there's a selected row in the file list
                    if let Some(selected_row) = file_list_box_for_key.selected_row() {
                        if let Some(label) = selected_row
                            .child()
                            .and_then(|c| c.downcast::<Label>().ok())
                        {
                            let file_name = label.text();
                            let mut file_path = current_dir_for_key.borrow().clone();
                            file_path.push(file_name.as_str());

                            if file_path.is_file() {
                                crate::ui::file_manager::cut_file_to_clipboard(&file_path);
                                // Refresh file list to show visual changes (cut file opacity)
                                crate::utils::update_file_list(
                                    &file_list_box_for_key,
                                    &current_dir_for_key.borrow(),
                                    &active_tab_path_for_key.borrow(),
                                    crate::utils::FileSelectionSource::TabSwitch,
                                );
                            }
                        }
                    }
                    // Second priority: Check if there's an active tab with a file in current directory
                    else if let Some(active_file) = active_tab_path_for_key.borrow().as_ref() {
                        if active_file.parent() == Some(current_dir_for_key.borrow().as_path())
                            && active_file.is_file()
                        {
                            crate::ui::file_manager::cut_file_to_clipboard(active_file);
                            // Refresh file list to show visual changes (cut file opacity)
                            crate::utils::update_file_list(
                                &file_list_box_for_key,
                                &current_dir_for_key.borrow(),
                                &active_tab_path_for_key.borrow(),
                                crate::utils::FileSelectionSource::TabSwitch,
                            );
                        }
                    }
                    // Always stop propagation when file list has focus
                    return glib::Propagation::Stop;
                }
                // Ctrl+V: Paste file
                gtk4::gdk::Key::v => {
                    println!("DEBUG: Ctrl+V pressed in file list");

                    if crate::ui::file_manager::has_clipboard_content() {
                        crate::ui::file_manager::paste_file_from_clipboard(
                            &current_dir_for_key.borrow(),
                            window_for_key.upcast_ref::<ApplicationWindow>(),
                            &file_list_box_for_key,
                            &current_dir_for_key,
                            &active_tab_path_for_key,
                        );
                        return glib::Propagation::Stop;
                    } else {
                        crate::status_log::log_error("No file in clipboard to paste");
                        return glib::Propagation::Stop;
                    }
                }
                _ => {}
            }
        }

        // Handle Delete key for file deletion
        if keyval == gtk4::gdk::Key::Delete
            && !state.contains(gtk4::gdk::ModifierType::CONTROL_MASK)
        {
            println!("DEBUG: Delete key pressed in file handler");

            // First priority: Check if there's a selected row in the file list
            if let Some(selected_row) = file_list_box_for_key.selected_row() {
                if let Some(label) = selected_row
                    .child()
                    .and_then(|c| c.downcast::<Label>().ok())
                {
                    let file_name = label.text();
                    let mut file_path = current_dir_for_key.borrow().clone();
                    file_path.push(file_name.as_str());
                    println!("DEBUG: File from selected row: {:?}", file_path);

                    // Only delete files, not directories
                    if file_path.is_file() {
                        println!("DEBUG: Deleting selected file");
                        handle_file_deletion(
                            &file_path,
                            &window_for_key,
                            &file_list_box_for_key,
                            &current_dir_for_key,
                            &active_tab_path_for_key,
                            &editor_notebook_for_key,
                            &file_path_manager_for_key,
                        );
                        return glib::Propagation::Stop;
                    } else {
                        println!("DEBUG: Selected item is not a file: {:?}", file_path);
                    }
                }
            }
            // Second priority: Check if there's an active tab with a file in current directory
            else if let Some(active_file) = active_tab_path_for_key.borrow().as_ref() {
                println!("DEBUG: Checking active tab file: {:?}", active_file);
                if active_file.parent() == Some(current_dir_for_key.borrow().as_path())
                    && active_file.is_file()
                {
                    println!("DEBUG: Deleting active tab file");
                    handle_file_deletion(
                        active_file,
                        &window_for_key,
                        &file_list_box_for_key,
                        &current_dir_for_key,
                        &active_tab_path_for_key,
                        &editor_notebook_for_key,
                        &file_path_manager_for_key,
                    );
                    return glib::Propagation::Stop;
                } else {
                    println!(
                        "DEBUG: Active file is not in current directory: {:?} vs {:?}",
                        active_file.parent(),
                        current_dir_for_key.borrow().as_path()
                    );
                }
            }

            println!("DEBUG: No file found to delete");
        }
        glib::Propagation::Proceed
    });

    // Attach to file list box so it only handles when file list has focus
    file_list_box.add_controller(key_controller);

    // Add right-click context menu support
    let right_click_gesture = GestureClick::new();
    right_click_gesture.set_button(3); // Right mouse button

    let file_list_box_for_context = file_list_box.clone();
    let editor_notebook_for_context = editor_notebook.clone();
    let active_tab_path_for_context = active_tab_path_ref.clone();
    let file_path_manager_for_context = file_path_manager.clone();
    let current_dir_for_context = current_dir.clone();
    let window_for_context = window.clone();
    let save_button_for_context = save_button.clone();
    let save_as_button_for_context = save_as_button.clone();

    right_click_gesture.connect_pressed(move |gesture, _n_press, x, y| {
        // Find which row was clicked
        if let Some(row) = file_list_box_for_context.row_at_y(y as i32) {
            // Select the row that was right-clicked
            file_list_box_for_context.select_row(Some(&row));

            if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
                let file_name = label.text();
                let mut file_path = current_dir_for_context.borrow().clone();
                file_path.push(file_name.as_str());

                // Only show context menu for files, not directories
                if file_path.is_file() {
                    // Get the widget that triggered the gesture for proper coordinate conversion
                    if let Some(widget) = gesture.widget() {
                        show_file_context_menu(
                            &file_path,
                            &window_for_context,
                            &file_list_box_for_context,
                            &current_dir_for_context,
                            &active_tab_path_for_context,
                            &editor_notebook_for_context,
                            &file_path_manager_for_context,
                            &widget,
                            &row,
                            x,
                            y,
                        );
                    }
                }
            }
        } else {
            // Right-clicked on empty space - show background context menu
            show_file_manager_background_context_menu(
                &window_for_context,
                &file_list_box_for_context,
                &current_dir_for_context,
                &active_tab_path_for_context,
                &editor_notebook_for_context,
                &file_path_manager_for_context,
                &save_button_for_context,
                &save_as_button_for_context,
                x,
                y,
            );
        }
    });

    file_list_box.add_controller(right_click_gesture);

    // Add left-click gesture to ensure file list box grabs focus when clicked
    let left_click_gesture = GestureClick::new();
    left_click_gesture.set_button(1); // Left mouse button

    let file_list_box_for_focus = file_list_box.clone();
    left_click_gesture.connect_pressed(move |_gesture, _n_press, _x, y| {
        // Always grab focus when the file list box is clicked (including empty areas)
        println!("DEBUG: Left click on file manager - grabbing focus");
        file_list_box_for_focus.grab_focus();

        // Force focus by also setting can_focus and making sure it's focusable
        file_list_box_for_focus.set_can_focus(true);
        file_list_box_for_focus.set_focusable(true);

        // Mark file manager as the last active area
        LAST_ACTIVE_AREA.with(|area| {
            *area.borrow_mut() = LastActiveArea::FileManager;
        });
        println!("DEBUG: Set last active area to FileManager");

        // Also handle selection logic for clicks on empty areas
        if let Some(row_at_position) = file_list_box_for_focus.row_at_y(y as i32) {
            // Clicking on a row - let the normal selection happen
            file_list_box_for_focus.select_row(Some(&row_at_position));
            println!("DEBUG: Selected row at position y={}", y);
        } else {
            // Clicking on empty area - clear selection
            file_list_box_for_focus.unselect_all();
            println!("DEBUG: Clicked empty area - cleared selection");
        }

        // Verify focus was grabbed
        println!(
            "DEBUG: File list has_focus after grab: {}",
            file_list_box_for_focus.has_focus()
        );
    });

    file_list_box.add_controller(left_click_gesture);

    // Ensure focus is grabbed when selection changes (for keyboard navigation)
    let file_list_box_for_selection = file_list_box.clone();
    file_list_box.connect_selected_rows_changed(move |_| {
        println!("DEBUG: Selection changed - grabbing focus");
        file_list_box_for_selection.grab_focus();
    });

    file_list_box.connect_row_activated(move |list_box, row| {
        // Grab focus to ensure file operations work correctly
        println!("DEBUG: Row activated - grabbing focus");
        list_box.set_can_focus(true);
        list_box.set_focusable(true);
        list_box.grab_focus();

        // Verify focus was grabbed
        println!(
            "DEBUG: File list has_focus after row activation: {}",
            list_box.has_focus()
        );

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
        // Clone the selection source tracker for this closure
        let current_selection_source_for_handler = current_selection_source_clone.clone();

        if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
            let file_name = label.text();
            let mut path_from_list = current_dir_for_handler.borrow().clone(); // Use cloned current_dir
            path_from_list.push(file_name.as_str());

            // If it's a file (not a directory), close any empty untitled tabs before opening
            if path_from_list.is_file() {
                close_empty_untitled_tabs(
                    &editor_notebook_for_handler,
                    &file_path_manager_for_handler,
                );
            }

            if path_from_list.is_dir() {
                *current_dir_for_handler.borrow_mut() = path_from_list.clone();
                utils::update_file_list(
                    &file_list_box_for_handler_update,
                    &current_dir_for_handler.borrow(),
                    &active_tab_path_for_handler.borrow(),
                    utils::FileSelectionSource::TabSwitch,
                );
                file_list_box_for_handler_update.grab_focus(); // Add this line to shift focus

                // Update the path buttons if provided
                if let Some(box_widget) = &path_box_option {
                    if let Some(path_box) = box_widget.downcast_ref::<gtk4::Box>() {
                        utils::update_path_buttons(
                            path_box,
                            &current_dir_for_handler,
                            &file_list_box_for_handler_update,
                            &active_tab_path_for_handler,
                        );
                    }
                }

                // Check for Rust files and update linter UI visibility
                crate::linter::ui::check_and_update_rust_ui(&path_from_list);
            } else if path_from_list.is_file() {
                let mut mime_type = mime_guess::from_path(&path_from_list).first_or_octet_stream();

                // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
                // but should be treated as TypeScript files (text/plain)
                if let Some(ext) = path_from_list.extension() {
                    if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                        // Override MIME type for TypeScript files
                        mime_type = mime_guess::mime::TEXT_PLAIN;
                    }
                }

                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(content) = std::fs::read_to_string(&path_from_list) {
                        open_or_focus_tab(
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
                        // Ensure the list reflects the newly opened file as active with DirectClick styling
                        // and update the selection source tracker
                        *current_selection_source_for_handler.borrow_mut() =
                            utils::FileSelectionSource::DirectClick;
                        utils::update_file_list(
                            &file_list_box_for_handler_update,
                            &current_dir_for_handler.borrow(),
                            &active_tab_path_for_handler.borrow(),
                            utils::FileSelectionSource::DirectClick,
                        );
                    }
                } else if mime_type.type_() == "image" {
                    // Check if it's an SVG file (which needs content for split view)
                    let is_svg = path_from_list
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase() == "svg")
                        .unwrap_or(false);

                    let content = if is_svg {
                        // SVG files need content for the code editor
                        std::fs::read_to_string(&path_from_list).unwrap_or_default()
                    } else {
                        // Other images don't need content
                        String::new()
                    };

                    // Use open_or_focus_tab for images
                    open_or_focus_tab(
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
                    // Ensure the list reflects the newly opened file as active with DirectClick styling
                    // and update the selection source tracker
                    *current_selection_source_for_handler.borrow_mut() =
                        utils::FileSelectionSource::DirectClick;
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow(),
                        utils::FileSelectionSource::DirectClick,
                    );
                } else if mime_type.type_() == "audio" {
                    // Use open_or_focus_tab for audio files
                    open_or_focus_tab(
                        &editor_notebook_for_handler,
                        &path_from_list,
                        "", // Empty content for audio files
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
                    // Ensure the list reflects the newly opened file as active with DirectClick styling
                    // and update the selection source tracker
                    *current_selection_source_for_handler.borrow_mut() =
                        utils::FileSelectionSource::DirectClick;
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow(),
                        utils::FileSelectionSource::DirectClick,
                    );
                } else if mime_type.type_() == "video" {
                    // Use open_or_focus_tab for video files
                    open_or_focus_tab(
                        &editor_notebook_for_handler,
                        &path_from_list,
                        "", // Empty content for video files
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
                    // Ensure the list reflects the newly opened file as active with DirectClick styling
                    // and update the selection source tracker
                    *current_selection_source_for_handler.borrow_mut() =
                        utils::FileSelectionSource::DirectClick;
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow(),
                        utils::FileSelectionSource::DirectClick,
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
                    // Ensure the list reflects the newly opened file as active with DirectClick styling
                    // and update the selection source tracker
                    *current_selection_source_for_handler.borrow_mut() =
                        utils::FileSelectionSource::DirectClick;
                    utils::update_file_list(
                        &file_list_box_for_handler_update,
                        &current_dir_for_handler.borrow(),
                        &active_tab_path_for_handler.borrow(),
                        utils::FileSelectionSource::DirectClick,
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
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>, // Changed from file_path
    path_box: Option<&gtk4::Box>,                   // Optional path box for status bar
) {
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let active_tab_path = active_tab_path.clone(); // Clone Rc for closure
    let path_box = path_box.cloned(); // Clone the optional Box widget

    up_button.connect_clicked(move |_| {
        let mut path = current_dir.borrow().clone();
        if path.pop() {
            *current_dir.borrow_mut() = path.clone();
            // Pass the active tab\'s path for selection highlighting
            utils::update_file_list(
                &file_list_box_clone,
                &current_dir.borrow(),
                &active_tab_path.borrow(),
                utils::FileSelectionSource::TabSwitch,
            );

            // Update the path buttons if provided
            if let Some(path_box) = &path_box {
                utils::update_path_buttons(
                    path_box,
                    &current_dir,
                    &file_list_box_clone,
                    &active_tab_path,
                );
            }

            // Check for Rust files and update linter UI visibility
            crate::linter::ui::check_and_update_rust_ui(&path);
        }
    });
}

/// Helper function to close default empty untitled tabs
///
/// This function checks if there's an empty untitled tab and closes it
/// when opening a new file or creating a new tab.
pub fn close_empty_untitled_tabs(
    notebook: &Notebook,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
) {
    // Only proceed if there are pages to check
    if notebook.n_pages() == 0 {
        return;
    }

    // Collect tabs to remove - we'll store their indices
    let mut tabs_to_remove = Vec::new();

    // Check all tabs
    for page_num in 0..notebook.n_pages() {
        // Skip if this tab has a file associated with it in the path manager
        if file_path_manager.borrow().contains_key(&page_num) {
            continue;
        }

        // Check if this tab is an untitled tab with no content
        if let Some((_, buffer)) = get_text_view_and_buffer_for_page(notebook, page_num) {
            // Get the tab label to verify it's "Untitled" (not "Untitled*")
            if let Some(page) = notebook.nth_page(Some(page_num)) {
                if let Some(tab_label_widget) = notebook.tab_label(&page) {
                    if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                        if let Some(label) = tab_box
                            .first_child()
                            .and_then(|w| w.downcast::<Label>().ok())
                        {
                            let label_text = label.text();

                            // Check if this is an empty untitled tab
                            // This covers both cases: "Untitled" AND "Untitled*" with empty content
                            if (label_text == "Untitled" || label_text == "*Untitled")
                                && buffer
                                    .text(&buffer.start_iter(), &buffer.end_iter(), false)
                                    .is_empty()
                            {
                                tabs_to_remove.push(page_num);
                            }
                        }
                    }
                }
            }
        }
    }

    // Remove the tabs in reverse order to avoid index shifting problems
    tabs_to_remove.sort_unstable();
    tabs_to_remove.reverse();

    for page_num in tabs_to_remove {
        notebook.remove_page(Some(page_num));
    }
}

/// Shows a confirmation dialog and deletes a file if confirmed
///
/// This function displays a warning dialog to the user asking for confirmation
/// before deleting the specified file. If confirmed, the file is deleted and
/// the file list is refreshed.
pub fn handle_file_deletion(
    file_path: &PathBuf,
    window: &impl IsA<ApplicationWindow>,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    editor_notebook: &Notebook,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
) {
    let file_name = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "Unknown file".to_string());

    // First, check if the file is open in a tab and handle it properly
    // Clone variables for the closure that will be called after tab handling
    let file_path_clone = file_path.clone();
    let file_list_box_clone = file_list_box.clone();
    let current_dir_clone = current_dir.clone();
    let active_tab_path_clone = active_tab_path.clone();
    let window_clone = window.clone();
    let file_name_clone = file_name.clone();

    close_tab_if_file_open_with_save_prompt(
        editor_notebook,
        file_path,
        file_path_manager,
        active_tab_path,
        window,
        current_dir,
        file_list_box,
        move |tab_handled_successfully| {
            if !tab_handled_successfully {
                // User canceled the tab close operation, don't proceed with deletion
                return;
            }

            // Tab was successfully closed (or wasn't open), proceed with file deletion confirmation
            let dialog = MessageDialog::new(
                Some(window_clone.as_ref().upcast_ref::<gtk4::Window>()),
                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                MessageType::Warning,
                ButtonsType::None,
                format!(
                    "Are you sure you want to delete '{}'?\n\nThis action cannot be undone.",
                    file_name_clone
                ),
            );

            dialog.add_buttons(&[
                ("Cancel", ResponseType::Cancel),
                ("Delete", ResponseType::Accept),
            ]);

            dialog.set_default_response(ResponseType::Cancel);

            // Clone variables again for the inner closure
            let file_path_inner = file_path_clone.clone();
            let file_list_box_inner = file_list_box_clone.clone();
            let current_dir_inner = current_dir_clone.clone();
            let active_tab_path_inner = active_tab_path_clone.clone();
            let window_inner = window_clone.clone();

            dialog.connect_response(move |d, response| {
                if response == ResponseType::Accept {
                    // User confirmed deletion
                    match std::fs::remove_file(&file_path_inner) {
                        Ok(()) => {
                            println!("Successfully deleted file: {:?}", file_path_inner);

                            // Refresh the file list
                            utils::update_file_list(
                                &file_list_box_inner,
                                &current_dir_inner.borrow(),
                                &active_tab_path_inner.borrow(),
                                utils::FileSelectionSource::TabSwitch,
                            );
                        }
                        Err(e) => {
                            eprintln!("Failed to delete file: {:?}, error: {}", file_path_inner, e);

                            // Show error dialog
                            let error_dialog = MessageDialog::new(
                                Some(window_inner.as_ref().upcast_ref::<gtk4::Window>()),
                                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                                MessageType::Error,
                                ButtonsType::Ok,
                                format!("Failed to delete file: {}", e),
                            );
                            error_dialog.show();
                        }
                    }
                }
                d.close();
            });

            dialog.show();
        },
    );
}

/// Closes a tab if the specified file is currently open, with proper save handling
///
/// This helper function checks all open tabs to see if any contain the specified file,
/// and if so, closes that tab with proper save prompts if the file has unsaved changes.
/// Returns true if the tab was closed (or no tab was open for this file), false if user canceled.
fn close_tab_if_file_open_with_save_prompt(
    notebook: &Notebook,
    file_path: &PathBuf,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &impl IsA<ApplicationWindow>,
    _current_dir: &Rc<RefCell<PathBuf>>,
    _file_list_box: &ListBox,
    callback: impl Fn(bool) + 'static, // Callback to indicate success/cancellation
) {
    let manager = file_path_manager.borrow();

    // Find if the file is open in any tab
    let mut found_page_num = None;
    for (&page_num, path) in manager.iter() {
        if path == file_path {
            found_page_num = Some(page_num);
            break;
        }
    }

    drop(manager); // Release the borrow

    match found_page_num {
        Some(page_num) => {
            // File is open in a tab - check if it has unsaved changes
            if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
                if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
                    let mut is_dirty = false;
                    if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                        if let Some(label) = tab_box
                            .first_child()
                            .and_then(|w| w.downcast::<Label>().ok())
                        {
                            if label.text().starts_with('*') {
                                is_dirty = true;
                            }
                        }
                    }

                    if !is_dirty {
                        // Not dirty, close directly and proceed
                        actually_close_tab(
                            notebook,
                            page_num,
                            file_path_manager,
                            active_tab_path,
                            None,
                        );
                        callback(true);
                        return;
                    }

                    // Has unsaved changes - show confirmation dialog
                    let filename_str = file_path
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| "Unknown file".to_string());

                    let dialog = MessageDialog::new(
                        Some(window.as_ref().upcast_ref::<gtk4::Window>()),
                        DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                        MessageType::Question,
                        ButtonsType::None,
                        format!("The file '{}' has unsaved changes.\n\nSave changes before closing and deleting?", filename_str)
                    );

                    dialog.add_buttons(&[
                        ("Cancel", ResponseType::Cancel),
                        ("Don't Save", ResponseType::No),
                        ("Save", ResponseType::Yes),
                    ]);

                    dialog.set_default_response(ResponseType::Cancel);

                    // Clone variables for the closure - need to own the path
                    let notebook_clone = notebook.clone();
                    let file_path_manager_clone = file_path_manager.clone();
                    let active_tab_path_clone = active_tab_path.clone();
                    let file_path_owned = file_path.clone(); // Own the path

                    dialog.connect_response(move |d, response| {
                        match response {
                            ResponseType::Yes => {
                                // Save first, then close
                                if let Some((_tv, buffer)) =
                                    get_text_view_and_buffer_for_page(&notebook_clone, page_num)
                                {
                                    let text = buffer.text(
                                        &buffer.start_iter(),
                                        &buffer.end_iter(),
                                        false,
                                    );
                                    match File::create(&file_path_owned) {
                                        Ok(mut file) => {
                                            if file.write_all(text.as_bytes()).is_ok() {
                                                // Update tab label to show saved state
                                                update_tab_label_after_save(
                                                    &notebook_clone,
                                                    page_num,
                                                    Some(&filename_str),
                                                    false,
                                                );

                                                // Close the tab
                                                actually_close_tab(
                                                    &notebook_clone,
                                                    page_num,
                                                    &file_path_manager_clone,
                                                    &active_tab_path_clone,
                                                    None,
                                                );
                                                callback(true);
                                            } else {
                                                eprintln!(
                                                    "Error writing to file: {:?}",
                                                    file_path_owned
                                                );
                                                callback(false);
                                            }
                                        }
                                        Err(e) => {
                                            eprintln!(
                                                "Error creating file for writing: {:?}, error: {}",
                                                file_path_owned, e
                                            );
                                            callback(false);
                                        }
                                    }
                                } else {
                                    callback(false);
                                }
                            }
                            ResponseType::No => {
                                // Don't save, just close
                                actually_close_tab(
                                    &notebook_clone,
                                    page_num,
                                    &file_path_manager_clone,
                                    &active_tab_path_clone,
                                    None,
                                );
                                callback(true);
                            }
                            ResponseType::Cancel | _ => {
                                // User canceled
                                callback(false);
                            }
                        }
                        d.close();
                    });

                    dialog.show();
                } else {
                    // Could not get tab label widget, close without prompts
                    actually_close_tab(
                        notebook,
                        page_num,
                        file_path_manager,
                        active_tab_path,
                        None,
                    );
                    callback(true);
                }
            } else {
                // Could not get page widget, close without prompts
                actually_close_tab(notebook, page_num, file_path_manager, active_tab_path, None);
                callback(true);
            }
        }
        None => {
            // File is not open in any tab, proceed directly
            callback(true);
        }
    }
}

/// Shows a context menu for file operations
///
/// This function creates and displays a context menu when a user right-clicks
/// on a file in the file manager. Currently supports file deletion.
fn show_file_context_menu(
    file_path: &PathBuf,
    window: &impl IsA<ApplicationWindow>,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    editor_notebook: &Notebook,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    _gesture_widget: &gtk4::Widget,
    clicked_row: &gtk4::ListBoxRow,
    x: f64,
    y: f64,
) {
    println!("DEBUG: Creating context menu for file: {:?}", file_path);

    // Create a simple button in a popover instead of using menu model
    let popover = gtk4::Popover::new();

    // Create a box to hold the button
    let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    menu_box.add_css_class("menu");

    // Create copy button
    let copy_button = Button::with_label("Copy");
    copy_button.add_css_class("flat");
    copy_button.set_hexpand(true);
    if let Some(child) = copy_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }

    // Create cut button
    let cut_button = Button::with_label("Cut");
    cut_button.add_css_class("flat");
    cut_button.set_hexpand(true);
    if let Some(child) = cut_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }

    // Create paste button (if there's content in clipboard)
    let paste_button = Button::with_label("Paste");
    paste_button.add_css_class("flat");
    paste_button.set_hexpand(true);
    if let Some(child) = paste_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }
    paste_button.set_sensitive(crate::ui::file_manager::has_clipboard_content());

    // Add separator
    let separator1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);

    // Create delete button
    let delete_button = Button::with_label("Delete");
    delete_button.add_css_class("flat");
    delete_button.add_css_class("destructive-action");
    delete_button.set_hexpand(true);
    if let Some(child) = delete_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }

    // Clone variables for the copy button closure
    let file_path_copy = file_path.clone();
    let popover_copy_weak = popover.downgrade();

    copy_button.connect_clicked(move |_| {
        println!("DEBUG: Copy button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_copy_weak.upgrade() {
            popover.popdown();
        }

        crate::ui::file_manager::copy_file_to_clipboard(&file_path_copy);
    });

    // Clone variables for the cut button closure
    let file_path_cut = file_path.clone();
    let popover_cut_weak = popover.downgrade();

    cut_button.connect_clicked(move |_| {
        println!("DEBUG: Cut button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_cut_weak.upgrade() {
            popover.popdown();
        }

        crate::ui::file_manager::cut_file_to_clipboard(&file_path_cut);
    });

    // Clone variables for the paste button closure
    let window_paste = window.clone();
    let file_list_box_paste = file_list_box.clone();
    let current_dir_paste = current_dir.clone();
    let active_tab_path_paste = active_tab_path.clone();
    let popover_paste_weak = popover.downgrade();

    paste_button.connect_clicked(move |_| {
        println!("DEBUG: Paste button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_paste_weak.upgrade() {
            popover.popdown();
        }

        crate::ui::file_manager::paste_file_from_clipboard(
            &current_dir_paste.borrow(),
            window_paste.upcast_ref::<ApplicationWindow>(),
            &file_list_box_paste,
            &current_dir_paste,
            &active_tab_path_paste,
        );
    });

    // Clone variables for the delete button closure
    let file_path_clone = file_path.clone();
    let window_clone = window.clone();
    let file_list_box_clone = file_list_box.clone();
    let current_dir_clone = current_dir.clone();
    let active_tab_path_clone = active_tab_path.clone();
    let editor_notebook_clone = editor_notebook.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let popover_weak = popover.downgrade();

    delete_button.connect_clicked(move |_| {
        println!("DEBUG: Delete button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }

        // Show deletion confirmation
        handle_file_deletion(
            &file_path_clone,
            &window_clone,
            &file_list_box_clone,
            &current_dir_clone,
            &active_tab_path_clone,
            &editor_notebook_clone,
            &file_path_manager_clone,
        );
    });

    // Add all buttons to the menu
    menu_box.append(&copy_button);
    menu_box.append(&cut_button);
    menu_box.append(&paste_button);
    menu_box.append(&separator1);
    menu_box.append(&delete_button);
    popover.set_child(Some(&menu_box));

    // Set the parent to the clicked row for proper positioning
    popover.set_parent(clicked_row);

    // Convert coordinates from gesture widget to the row widget
    let row_allocation = clicked_row.allocation();

    // Position the menu relative to the clicked row
    // Use a small rectangle at the click position within the row
    let relative_x = x.max(0.0).min(row_allocation.width() as f64 - 1.0);
    let relative_y = y.max(0.0).min(row_allocation.height() as f64 - 1.0);

    popover.set_pointing_to(Some(&gtk4::gdk::Rectangle::new(
        relative_x as i32,
        relative_y as i32,
        1,
        1,
    )));

    // Properly handle cleanup when the popover is closed
    let popover_weak_cleanup = popover.downgrade();
    popover.connect_closed(move |_| {
        if let Some(popover) = popover_weak_cleanup.upgrade() {
            popover.unparent();
        }
    });

    // Show the popover
    println!("DEBUG: Showing context menu popover");
    popover.popup();
}

/// Shows a context menu when right-clicking in empty space of the file manager
///
/// This function creates and displays a context menu for general file manager actions
/// like creating new files when clicking in empty space.
fn show_file_manager_background_context_menu(
    window: &impl IsA<ApplicationWindow>,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    editor_notebook: &Notebook,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    save_button: &Button,
    save_as_button: &Button,
    x: f64,
    y: f64,
) {
    println!("DEBUG: Creating background context menu for file manager");

    // Create a popover for the context menu
    let popover = gtk4::Popover::new();

    // Create a box to hold the menu items
    let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    menu_box.add_css_class("menu");

    // Create "New File" button
    let new_file_button = Button::with_label("New File");
    new_file_button.add_css_class("flat");
    new_file_button.set_hexpand(true);
    if let Some(child) = new_file_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }

    // Create "Paste" button (if there's content in clipboard)
    let paste_button = Button::with_label("Paste");
    paste_button.add_css_class("flat");
    paste_button.set_hexpand(true);
    if let Some(child) = paste_button.child() {
        if let Ok(label) = child.downcast::<gtk4::Label>() {
            label.set_xalign(0.0);
        }
    }
    paste_button.set_sensitive(crate::ui::file_manager::has_clipboard_content());

    // Add separator
    let separator = gtk4::Separator::new(gtk4::Orientation::Horizontal);

    // Clone variables for the paste button closure
    let window_paste = window.clone();
    let file_list_box_paste = file_list_box.clone();
    let current_dir_paste = current_dir.clone();
    let active_tab_path_paste = active_tab_path.clone();
    let popover_paste_weak = popover.downgrade();

    paste_button.connect_clicked(move |_| {
        println!("DEBUG: Background Paste button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_paste_weak.upgrade() {
            popover.popdown();
        }

        crate::ui::file_manager::paste_file_from_clipboard(
            &current_dir_paste.borrow(),
            window_paste.upcast_ref::<ApplicationWindow>(),
            &file_list_box_paste,
            &current_dir_paste,
            &active_tab_path_paste,
        );
    });

    // Clone variables for the button closure
    let editor_notebook_clone = editor_notebook.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let active_tab_path_clone = active_tab_path.clone();
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    let window_clone = window.clone();
    let current_dir_clone = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let popover_weak = popover.downgrade();

    new_file_button.connect_clicked(move |_| {
        println!("DEBUG: New File button clicked!");

        // Hide the context menu first
        if let Some(popover) = popover_weak.upgrade() {
            popover.popdown();
        }

        // Create new empty tab
        let new_tab_deps = NewTabDependencies {
            editor_notebook: editor_notebook_clone.clone(),
            window: window_clone.clone().upcast::<ApplicationWindow>(),
            file_list_box: file_list_box_clone.clone(),
            active_tab_path: active_tab_path_clone.clone(),
            file_path_manager: file_path_manager_clone.clone(),
            current_dir: current_dir_clone.clone(),
            save_button: save_button_clone.clone(),
            save_as_button: save_as_button_clone.clone(),
            _save_menu_button: None,
        };

        create_new_empty_tab(&new_tab_deps);
    });

    // Add button to menu box
    menu_box.append(&new_file_button);
    if crate::ui::file_manager::has_clipboard_content() {
        menu_box.append(&separator);
        menu_box.append(&paste_button);
    }

    // Set the menu box as the popover's child
    popover.set_child(Some(&menu_box));

    // Set the popover's parent and position
    popover.set_parent(file_list_box);

    // Set position
    let rect = gtk4::gdk::Rectangle::new(x as i32, y as i32, 1, 1);
    popover.set_pointing_to(Some(&rect));

    // Properly handle cleanup when the popover is closed
    let popover_weak_cleanup = popover.downgrade();
    popover.connect_closed(move |_| {
        if let Some(popover) = popover_weak_cleanup.upgrade() {
            popover.unparent();
        }
    });

    // Show the popover
    println!("DEBUG: Showing background context menu popover");
    popover.popup();
}

use once_cell::sync::Lazy;
use std::sync::Mutex as StdMutex;

// Type alias for complex callback type
type OpenFileCallbackType = StdMutex<Option<Box<dyn Fn(PathBuf, usize, usize) + Send + Sync>>>;

/// Global callback for opening files and jumping to locations
/// This is set by main.rs and used by the diagnostics panel
pub static OPEN_FILE_CALLBACK: Lazy<OpenFileCallbackType> = Lazy::new(|| StdMutex::new(None));

/// Open a file and jump to a specific line and column
/// This is used by the diagnostics panel to navigate to error locations
pub fn open_file_and_jump_to_location(file_path: PathBuf, line: usize, column: usize) {
    println!(
        "open_file_and_jump_to_location: {} at {}:{}",
        file_path.display(),
        line,
        column
    );

    // Call the global callback if it's set
    if let Ok(guard) = OPEN_FILE_CALLBACK.lock() {
        if let Some(callback) = guard.as_ref() {
            callback(file_path, line, column);
        } else {
            eprintln!("OPEN_FILE_CALLBACK not set!");
        }
    }
}

/// Jump to a specific line and column in a source view
pub fn jump_to_line_and_column(source_view: &sourceview5::View, line: usize, column: usize) {
    let buffer = source_view.buffer();

    // Convert 1-based line to 0-based
    let line_index = if line > 0 { line - 1 } else { 0 };
    let column_index = if column > 0 { column - 1 } else { 0 };

    // Get the iterator at the specified line and column
    let mut iter = buffer
        .iter_at_line(line_index as i32)
        .unwrap_or_else(|| buffer.start_iter());

    // Move to the column
    for _ in 0..column_index {
        if !iter.forward_char() {
            break;
        }
    }

    // Place cursor at the position (don't select to avoid UI issues)
    buffer.place_cursor(&iter);

    // Grab focus to ensure the view is active
    source_view.grab_focus();

    // Scroll to the cursor position with a slight delay to ensure view is ready
    // This is especially important for the first click on each file
    let source_view_clone = source_view.clone();
    let iter_for_scroll = iter;
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        let mut scroll_iter = iter_for_scroll;
        source_view_clone.scroll_to_iter(&mut scroll_iter, 0.0, true, 0.5, 0.5);
    });

    println!("Jumped to line {}, column {}", line, column);
}
