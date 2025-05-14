mod ui;
mod handlers;
mod utils;

use gtk4::prelude::*;
use gtk4::Application;
use gtk4::gio;  // Added for menu actions
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap; // Added for HashMap
use std::path::PathBuf; // Added for PathBuf
use std::io::Write;     // For file writing

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ui::create_window(app);
    let (header, new_button, open_button, save_main_button, save_menu_button, save_as_button) = ui::create_header();

    // Create a separate hidden button to act as the target for handlers
    // This prevents stack overflow from circular references
    let save_button = gtk4::Button::new();
    
    let (
        _initial_scrolled_window, // ScrolledWindow content of the first tab
        _initial_text_view,       // TextView in the first tab
        initial_text_buffer,      // TextBuffer for the first tab
        _initial_tab_file_path_rc,// Rc<RefCell<Option<PathBuf>>> for the initial tab's content
        error_label,              // General error label
        picture,                  // General picture widget
        current_dir,              // Rc<RefCell<PathBuf>> for current directory
        editor_notebook,          // The main Notebook widget
        _initial_tab_widget,      // GtkBox for the initial tab's custom label
        initial_tab_actual_label, // Label within the initial tab's custom widget
        initial_tab_close_button  // Button within the initial tab's custom widget
    ) = ui::create_text_view();

    let file_path_manager = Rc::new(RefCell::new(HashMap::<u32, PathBuf>::new()));
    let active_tab_path = Rc::new(RefCell::new(None::<PathBuf>));

    // Define file_list_box and related UI elements before they are used in NewTabDependencies
    let (file_list_box, file_list_scrolled_window, nav_box, up_button, refresh_button) =
        ui::create_file_manager_panel();
    let file_manager_panel =
        ui::create_file_manager_panel_container(nav_box, file_list_scrolled_window);

    // Setup actions for the save menu
    let save_action = gio::SimpleAction::new("save", None);
    let save_as_action = gio::SimpleAction::new("save-as", None);
    
    // Clone necessary variables for the action closures
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    
    // Connect save action to be triggered from the menu
    let save_button_clone_for_action = save_button_clone.clone();
    save_action.connect_activate(move |_, _| {
        save_button_clone_for_action.emit_clicked();
    });
    
    // Connect save-as action to be triggered from the menu
    let save_as_button_clone_for_action = save_as_button_clone.clone();
    save_as_action.connect_activate(move |_, _| {
        save_as_button_clone_for_action.emit_clicked();
    });
    
    // Add the actions to the window
    window.add_action(&save_action);
    window.add_action(&save_as_action);
    
    // Instead of connecting the buttons to each other which causes recursion,
    // we'll connect them both to the same handler
    
    // Connect the save_main_button directly to the same handler that the hidden save_button uses
    let editor_notebook_clone = editor_notebook.clone();
    let _active_tab_path_clone = active_tab_path.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let _window_clone = window.clone();
    let _file_list_box_clone = file_list_box.clone();
    let _current_dir_clone = current_dir.clone();
    let save_as_button_clone = save_as_button.clone();
    
    save_main_button.connect_clicked(move |_| {
        // Call save functionality directly
        if let Some((_active_text_view, active_buffer)) = handlers::get_active_text_view_and_buffer(&editor_notebook_clone) {
            let current_page_num_opt = editor_notebook_clone.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            let path_to_save_opt = file_path_manager_clone.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                let mime_type = mime_guess::from_path(&path_to_save).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(mut file) = std::fs::File::create(&path_to_save) {
                        let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                        if file.write_all(text.as_bytes()).is_ok() {
                            // Update tab label (remove *)
                            handlers::update_tab_label_after_save(&editor_notebook_clone, current_page_num, Some(&path_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                        }
                    }
                }
            } else {
                // No path associated, trigger Save As
                save_as_button_clone.emit_clicked();
            }
        }
    });

    // Dirty tracking for the initial "Untitled" tab
    let initial_tab_actual_label_clone = initial_tab_actual_label.clone();
    let initial_buffer_clone_for_dirty_track = initial_text_buffer.clone();
    initial_text_buffer.connect_changed(move |_buffer| {
        let text_content = initial_buffer_clone_for_dirty_track.text(
            &initial_buffer_clone_for_dirty_track.start_iter(),
            &initial_buffer_clone_for_dirty_track.end_iter(),
            false
        );
        let label_text = initial_tab_actual_label_clone.text();
        if label_text == "Untitled" && !text_content.is_empty() {
            initial_tab_actual_label_clone.set_text("Untitled*");
        } else if label_text.ends_with('*') && text_content.is_empty() && label_text == "Untitled*" {
            initial_tab_actual_label_clone.set_text("Untitled");
        }
    });

    // Dependencies for creating a new tab if the last one is closed
    let deps_for_new_tab_creation = handlers::NewTabDependencies {
        editor_notebook: editor_notebook.clone(),
        active_tab_path: active_tab_path.clone(),
        file_path_manager: file_path_manager.clone(),
        window: window.clone(),
        file_list_box: file_list_box.clone(), // Now defined
        current_dir: current_dir.clone(),
        save_button: save_button.clone(),
        save_as_button: save_as_button.clone(),
        _save_menu_button: Some(save_menu_button.clone()), // Added underscore to acknowledge it's unused
    };

    let initial_tab_close_button_clone = initial_tab_close_button.clone();
    let editor_notebook_clone_for_initial_close = editor_notebook.clone();
    let window_clone_for_initial_close = window.clone();
    let file_path_manager_clone_for_initial_close = file_path_manager.clone();
    let active_tab_path_clone_for_initial_close = active_tab_path.clone();
    let current_dir_clone_for_initial_close = current_dir.clone(); // Clone for initial close
    let file_list_box_clone_for_initial_close = file_list_box.clone(); // Clone for initial close

    initial_tab_close_button_clone.connect_clicked(move |_| {
        // For the initial tab, its page number is almost always 0 unless manipulated before this signal.
        // However, to be robust, it's better to check if it still exists.
        if editor_notebook_clone_for_initial_close.n_pages() > 0 { 
            // Assuming the first tab (index 0) is the one we mean by "initial tab"
            // If other tabs could have been added *before* it, this logic would need to find the specific widget.
             if let Some(_page_widget) = editor_notebook_clone_for_initial_close.nth_page(Some(0)) {
                handlers::handle_close_tab_request(
                    &editor_notebook_clone_for_initial_close,
                    0, // Assuming page 0 is the initial tab
                    &window_clone_for_initial_close,
                    &file_path_manager_clone_for_initial_close,
                    &active_tab_path_clone_for_initial_close,
                    &current_dir_clone_for_initial_close, // Pass cloned current_dir
                    &file_list_box_clone_for_initial_close, // Pass cloned file_list_box
                    Some(deps_for_new_tab_creation.clone())
                );
             }
        }
    });

    let terminal = ui::create_terminal();
    let terminal_box = ui::create_terminal_box(&terminal);
    // file_list_box and related elements are already created above

    let paned = ui::create_paned(&file_manager_panel, &editor_notebook, &terminal_box);

    window.set_titlebar(Some(&header));

    // Update file list initially, possibly with no active file selection
    utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow());
    
    // Initialize save menu button visibility
    utils::update_save_menu_button_visibility(&save_menu_button, Some(mime_guess::mime::TEXT_PLAIN_UTF_8));
    
    window.set_child(Some(&paned));

    // Connect notebook's switch-page signal
    let file_path_manager_clone_for_switch = file_path_manager.clone();
    let active_tab_path_clone_for_switch = active_tab_path.clone();
    let file_list_box_clone_for_switch = file_list_box.clone();
    let current_dir_clone_for_switch = current_dir.clone();
    let save_button_clone_for_switch = save_button.clone();
    let save_as_button_clone_for_switch = save_as_button.clone();
    let save_menu_button_clone_for_switch = save_menu_button.clone(); // Added for menu button

    editor_notebook.connect_switch_page(move |notebook, _page, page_num| {
        let new_active_path = { // New scope to drop borrow on file_path_manager_clone_for_switch
            file_path_manager_clone_for_switch.borrow().get(&page_num).cloned()
        };

        *active_tab_path_clone_for_switch.borrow_mut() = new_active_path.clone();

        let current_dir_path_clone = current_dir_clone_for_switch.borrow().clone(); // Clone PathBuf to drop borrow on current_dir
        utils::update_file_list(&file_list_box_clone_for_switch, &current_dir_path_clone, &new_active_path);

        // Update save buttons based on the new active tab's content type (if any)
        if let Some((_, _buffer)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) { // Prefixed buffer with _        // For text views, determine mime type (assume text for now if path is None)
        let mime_type = new_active_path.as_ref()
            .map(|p| mime_guess::from_path(p).first_or_octet_stream())
            .unwrap_or(mime_guess::mime::TEXT_PLAIN_UTF_8); // Default to text for "Untitled"
        utils::update_save_buttons_visibility(&save_button_clone_for_switch, &save_as_button_clone_for_switch, Some(mime_type.clone()));
        utils::update_save_menu_button_visibility(&save_menu_button_clone_for_switch, Some(mime_type)); // Update menu button
            
            // Also, re-evaluate dirty status for save button if needed, though dirty flag on tab label is primary
            // For simplicity, save button visibility is mostly based on type, actual save action checks dirty state.
        } else {
            // If not a text view (e.g. placeholder for image), disable save buttons or handle appropriately
            utils::update_save_buttons_visibility(&save_button_clone_for_switch, &save_as_button_clone_for_switch, None);
            utils::update_save_menu_button_visibility(&save_menu_button_clone_for_switch, None); // Update menu button
        }
    });

    handlers::setup_button_handlers(
        &new_button,
        &open_button,
        &save_button,
        &save_as_button,
        &initial_text_buffer, 
        &file_path_manager,
        &active_tab_path,
        &window,
        &current_dir,
        &file_list_box, 
        &editor_notebook,
        &error_label,
        &picture,
        &up_button,
        &refresh_button,
        &file_list_box, // file_list_box_clone is the same as file_list_box here
        Some(&save_menu_button), // Pass the save menu button with the renamed parameter
    );

    window.show();
}
