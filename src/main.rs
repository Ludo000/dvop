mod ui;
mod handlers;
mod utils;

use gtk4::prelude::*;
use gtk4::Application;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap; // Added for HashMap
use std::path::PathBuf; // Added for PathBuf

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ui::create_window(app);
    let (header, new_button, open_button, save_button, save_as_button) = ui::create_header();
    // Updated to reflect changes in create_text_view
    let (editor_notebook, initial_text_buffer, _file_path_old, error_label, picture, current_dir) =
        ui::create_text_view(); // _file_path_old is no longer the primary path manager

    // Initialize new state for tab management
    let file_path_manager = Rc::new(RefCell::new(HashMap::<u32, PathBuf>::new()));
    let active_tab_path = Rc::new(RefCell::new(None::<PathBuf>));

    // If there's an initial tab, its path (if any) could be added to file_path_manager
    // For a new "Untitled" tab, it might not have a path initially.
    // Let's assume the first tab (index 0) is the initial one from create_text_view.
    // If it's meant to be "Untitled", no path is associated yet.
    // If create_text_view pre-opens a file, that logic would need to populate these.
    // For now, we assume the first tab is new/empty.

    let terminal = ui::create_terminal();
    let terminal_box = ui::create_terminal_box(&terminal);
    let (file_list_box, file_list_scrolled_window, nav_box, up_button, refresh_button) =
        ui::create_file_manager_panel();
    let file_manager_panel =
        ui::create_file_manager_panel_container(nav_box, file_list_scrolled_window);
    // Pass editor_notebook to create_paned
    let paned = ui::create_paned(&file_manager_panel, &editor_notebook, &terminal_box);

    window.set_titlebar(Some(&header));

    // Update file list initially, possibly with no active file selection
    utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow());

    window.set_child(Some(&paned));

    // No longer need to extract initial_text_view here as handlers will get it from notebook
    // let initial_scrolled_window = editor_notebook.current_page().and_then(|i| editor_notebook.nth_page(Some(i))).unwrap();
    // let initial_text_view = initial_scrolled_window.child().unwrap().downcast::<gtk4::TextView>().unwrap();

    handlers::setup_button_handlers(
        &new_button,
        &open_button,
        &save_button,
        &save_as_button,
        &initial_text_buffer, // Buffer for the first/initial tab (or handle dynamically)
        &file_path_manager,   // New: Manages paths for all tabs
        &active_tab_path,     // New: Path of the currently active tab
        &window,
        &current_dir,
        &file_list_box,
        &editor_notebook, // Pass the notebook
        // &initial_text_view, // text_view is now fetched dynamically from the notebook in handlers
        &error_label,
        &picture,
        &up_button,
        &refresh_button,
        &file_list_box, // file_list_box_clone, ensure this is the correct one (it's passed twice)
    );

    window.show();
}
