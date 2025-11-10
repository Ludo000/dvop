// Module declarations for the application components
mod ui;        // User interface components and layout
mod handlers;  // Event handlers and business logic
mod utils;     // Utility functions used across the application
mod syntax;    // Syntax highlighting functionality
mod settings;  // User settings and preferences
mod completion; // Code completion functionality
mod file_cache; // File content caching for performance optimization
mod status_log; // Status logging system
mod audio;     // Audio file playback functionality
mod video;     // Video file playback functionality
mod search;    // Find and replace functionality
mod linter;    // Code linting and diagnostics
mod lsp;       // Language Server Protocol integration

// GTK and standard library imports
use gtk4::prelude::*;   // GTK trait imports for widget functionality
use gtk4::{Application, ApplicationWindow, Label};  // Main GTK application classes
use gtk4::gio;          // GIO for menu and action support
use gtk4::glib;         // GLib for clone macro and other utilities
use std::rc::Rc;        // Reference counting for shared ownership
use std::cell::RefCell; // Interior mutability pattern
use std::collections::HashMap;   // For efficient mapping of tab indices to file paths
use std::path::PathBuf;        // File system path representation
use std::io::Write;            // File writing capabilities

/// Menu command structure for consistent command definitions
#[derive(Clone)]
struct MenuCommand {
    label: &'static str,
    action: &'static str,
    keywords: Vec<&'static str>,
}

/// Get the list of all menu commands
/// This single source of truth is used by both the menu search and can be used for menu generation
fn get_menu_commands() -> Vec<MenuCommand> {
    vec![
        MenuCommand { label: "New File", action: "win.new-file", keywords: vec!["new", "file", "create"] },
        MenuCommand { label: "Open...", action: "win.open", keywords: vec!["open", "file", "load"] },
        MenuCommand { label: "Save", action: "win.save", keywords: vec!["save", "write"] },
        MenuCommand { label: "Save As...", action: "win.save-as", keywords: vec!["save", "as", "copy"] },
        MenuCommand { label: "Close Tab", action: "win.close-tab", keywords: vec!["close", "tab"] },
        MenuCommand { label: "Close All Tabs", action: "win.close-all-tabs", keywords: vec!["close", "all", "tabs"] },
        MenuCommand { label: "Quit", action: "app.quit", keywords: vec!["quit", "exit", "close"] },
        MenuCommand { label: "Find...", action: "win.find", keywords: vec!["find", "search"] },
        MenuCommand { label: "Find and Replace...", action: "win.find-replace", keywords: vec!["find", "replace", "search"] },
        MenuCommand { label: "Preferences...", action: "win.preferences", keywords: vec!["preferences", "settings", "options"] },
        MenuCommand { label: "Explorer", action: "win.toggle-explorer", keywords: vec!["explorer", "files", "sidebar"] },
        MenuCommand { label: "Search", action: "win.toggle-search", keywords: vec!["search", "find", "sidebar"] },
        MenuCommand { label: "Source Control", action: "win.toggle-git", keywords: vec!["git", "source", "control", "version"] },
        MenuCommand { label: "Refresh File List", action: "win.refresh", keywords: vec!["refresh", "reload", "files"] },
        MenuCommand { label: "New Terminal", action: "win.new-terminal", keywords: vec!["new", "terminal", "console", "create"] },
        MenuCommand { label: "Toggle Terminal", action: "win.toggle-terminal", keywords: vec!["terminal", "toggle", "console"] },
        MenuCommand { label: "About Dvop", action: "win.about", keywords: vec!["about", "info", "version"] },
    ]
}

/// Sets up the menu search functionality
/// 
/// This function creates a searchable command palette that allows users to quickly
/// find and execute menu commands by typing their names
fn setup_menu_search(search_entry: &gtk4::SearchEntry, window: &ApplicationWindow) {
    // Get the shared list of menu commands
    let menu_commands = get_menu_commands();
    
    // Create a popover for showing search results
    let popover = gtk4::Popover::new();
    popover.set_parent(search_entry);
    popover.set_autohide(false);
    popover.set_can_focus(false);
    
    let listbox = gtk4::ListBox::new();
    listbox.add_css_class("navigation-sidebar");
    listbox.set_can_focus(false);
    listbox.set_focus_on_click(false);
    listbox.set_size_request(300, -1); // Fixed width, natural height
    popover.set_child(Some(&listbox));
    
    let window_weak = window.downgrade();
    let commands_clone = menu_commands.clone();
    let popover_clone = popover.clone();
    let entry_clone = search_entry.clone();
    
    // Handle item selection from the list
    let window_weak_for_activate = window_weak.clone();
    let popover_clone_for_activate = popover_clone.clone();
    let entry_clone_for_activate = entry_clone.clone();
    listbox.connect_row_activated(move |_, row| {
        if let Some(label) = row.child().and_then(|w| w.downcast::<gtk4::Label>().ok()) {
            let text = label.text();
            
            // Find and execute the matching command
            for cmd in &commands_clone {
                if cmd.label == text.as_str() {
                    if let Some(window) = window_weak_for_activate.upgrade() {
                        if cmd.action.starts_with("app.") {
                            if let Some(app) = window.application() {
                                app.activate_action(&cmd.action[4..], None);
                            }
                        } else if cmd.action.starts_with("win.") {
                            gtk4::prelude::ActionGroupExt::activate_action(&window, &cmd.action[4..], None);
                        }
                    }
                    entry_clone_for_activate.set_text("");
                    popover_clone_for_activate.popdown();
                    break;
                }
            }
        }
    });
    
    let commands_for_search = menu_commands.clone();
    let popover_for_search = popover.clone();
    let listbox_for_search = listbox.clone();
    
    // Update search results as user types
    search_entry.connect_search_changed(move |entry| {
        let search_text = entry.text().to_lowercase();
        
        // Clear previous results
        while let Some(child) = listbox_for_search.first_child() {
            listbox_for_search.remove(&child);
        }
        
        // Reset size to allow natural sizing
        listbox_for_search.set_size_request(300, -1);
        
        if search_text.is_empty() {
            popover_for_search.popdown();
            return;
        }
        
        // Find matching commands
        let mut found_any = false;
        for cmd in &commands_for_search {
            let name_lower = cmd.label.to_lowercase();
            let matches = name_lower.contains(&search_text) || 
                         cmd.keywords.iter().any(|k| k.contains(&search_text));
            
            if matches {
                let label = gtk4::Label::new(Some(cmd.label));
                label.set_xalign(0.0);
                label.set_margin_start(8);
                label.set_margin_end(8);
                label.set_margin_top(4);
                label.set_margin_bottom(4);
                
                let row = gtk4::ListBoxRow::new();
                row.set_child(Some(&label));
                listbox_for_search.append(&row);
                found_any = true;
            }
        }
        
        if found_any {
            // Force popover to close and reopen to recalculate size
            popover_for_search.popdown();
            glib::idle_add_local_once({
                let popover = popover_for_search.clone();
                move || {
                    popover.popup();
                }
            });
        } else {
            popover_for_search.popdown();
        }
    });
    
    // Handle Enter key to execute first result
    let popover_for_activate = popover.clone();
    let listbox_for_activate = listbox.clone();
    let commands_for_enter = menu_commands.clone();
    let window_weak_for_enter = window_weak.clone();
    search_entry.connect_activate(move |entry| {
        // Manually trigger the first row's action
        if let Some(first_row) = listbox_for_activate.first_child().and_then(|w| w.downcast::<gtk4::ListBoxRow>().ok()) {
            // Select and activate the first row
            listbox_for_activate.select_row(Some(&first_row));
            // Trigger the row-activated signal
            if let Some(label) = first_row.child().and_then(|w| w.downcast::<gtk4::Label>().ok()) {
                let text = label.text();
                
                // Find and execute the matching command
                for cmd in &commands_for_enter {
                    if cmd.label == text.as_str() {
                        if let Some(window) = window_weak_for_enter.upgrade() {
                            if cmd.action.starts_with("app.") {
                                if let Some(app) = window.application() {
                                    app.activate_action(&cmd.action[4..], None);
                                }
                            } else if cmd.action.starts_with("win.") {
                                gtk4::prelude::ActionGroupExt::activate_action(&window, &cmd.action[4..], None);
                            }
                        }
                        break;
                    }
                }
            }
        }
        entry.set_text("");
        popover_for_activate.popdown();
    });
    
    // Close popover when search is stopped
    let popover_for_stop = popover.clone();
    search_entry.connect_stop_search(move |_| {
        popover_for_stop.popdown();
    });
}

/// Application entry point - initializes the GTK application and runs the main loop
fn main() {
    // Initialize user settings first
    settings::initialize_settings();
    
    // Load log history from previous sessions
    status_log::load_log_history();
    
    // Create the main GTK application with a unique application ID
    // Set flags to handle file opening
    let app = Application::builder()
        .application_id("com.example.Dvop")
        .flags(gio::ApplicationFlags::HANDLES_OPEN)
        .build();
    
    // Force GTK to respect system dark mode settings
    app.connect_startup(|_| {
        if let Some(settings) = gtk4::Settings::default() {
            // Use our comprehensive dark mode detection function
            // This is more reliable than ad-hoc checks
            let prefer_dark = syntax::is_dark_mode_enabled();
            
            // Set dark mode preference
            settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                    
            // Double check that the setting took effect
            if settings.is_gtk_application_prefer_dark_theme() != prefer_dark {
                println!("Warning: GTK dark mode setting didn't match our preference! Trying again...");
                settings.set_gtk_application_prefer_dark_theme(prefer_dark);
                settings.notify("gtk-application-prefer-dark-theme");
            }
        }
        
        // Initialize completion system with JSON data loading
        completion::initialize_completion();
    });
    
    // Add application-level quit action
    let quit_action = gio::SimpleAction::new("quit", None);
    let app_for_quit = app.clone();
    quit_action.connect_activate(move |_, _| {
        app_for_quit.quit();
    });
    app.add_action(&quit_action);
    
    // Connect the activate signal to the build_ui function
    app.connect_activate(move |app| {
        println!("activate signal called!");
        build_ui(app, None);
    });

    // Connect the open signal to handle file opening from command line
    app.connect_open(move |app, files, _hint| {
        println!("open signal called with {} files", files.len());
        if let Some(file) = files.first() {
            if let Some(path) = file.path() {
                println!("Opening file from command line: {:?}", path);
                build_ui(app, Some(path));
            } else {
                println!("File has no path, opening without file");
                build_ui(app, None);
            }
        } else {
            println!("No files provided to open signal");
            build_ui(app, None);
        }
    });
    
    // Add startup signal for debugging
    app.connect_startup(|_| {
        println!("startup signal called!");
    });
    
    // Start the GTK main loop
    println!("Starting app.run()");
    app.run();
}

/// Updates the style scheme of all editor buffers when the system theme changes
pub fn update_all_buffer_themes(window: &impl IsA<gtk4::Widget>) {
    println!("Beginning comprehensive theme update for all buffers...");

    // First, let's try a more comprehensive search for notebooks
    fn find_all_notebooks(widget: &gtk4::Widget) -> Vec<gtk4::Notebook> {
        let mut notebooks = Vec::new();
        
        // Check if this widget is a notebook
        if let Some(notebook) = widget.downcast_ref::<gtk4::Notebook>() {
            notebooks.push(notebook.clone());
        }
        
        // Recursively search children
        let mut child = widget.first_child();
        while let Some(current_child) = child {
            notebooks.extend(find_all_notebooks(&current_child));
            child = current_child.next_sibling();
        }
        
        notebooks
    }

    let notebooks = find_all_notebooks(window.upcast_ref::<gtk4::Widget>());
    println!("Found {} notebooks in the window", notebooks.len());

    for (notebook_idx, notebook) in notebooks.iter().enumerate() {
        let n_pages = notebook.n_pages();
        println!("Notebook {}: Updating {} pages...", notebook_idx, n_pages);
        
        // Iterate through all notebook pages
        for page_num in 0..n_pages {
            if let Some(page) = notebook.nth_page(Some(page_num)) {
                println!("Processing notebook {} page {}", notebook_idx, page_num);
                
                // Try to find any SourceView in this page (could be nested)
                fn find_source_views(widget: &gtk4::Widget) -> Vec<sourceview5::View> {
                    let mut views = Vec::new();
                    
                    if let Some(source_view) = widget.downcast_ref::<sourceview5::View>() {
                        views.push(source_view.clone());
                    }
                    
                    let mut child = widget.first_child();
                    while let Some(current_child) = child {
                        views.extend(find_source_views(&current_child));
                        child = current_child.next_sibling();
                    }
                    
                    views
                }
                
                let source_views = find_source_views(&page);
                println!("Found {} source views in page {}", source_views.len(), page_num);
                
                for (view_idx, source_view) in source_views.iter().enumerate() {
                    let buffer = source_view.buffer();
                    if let Some(source_buffer) = buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
                        println!("Updating source buffer {} in page {}", view_idx, page_num);
                        syntax::update_buffer_style_scheme(source_buffer);
                        source_view.queue_draw();
                    }
                }
                
                // Force the page to redraw
                page.queue_draw();
            }
        }
        
        // Force the notebook to redraw
        notebook.queue_draw();
    }

    // Let's also print the current dark mode setting to help with debugging
    if let Some(settings) = gtk4::Settings::default() {
        let is_dark = settings.is_gtk_application_prefer_dark_theme();
        println!("Dark mode is now: {}", if is_dark { "enabled" } else { "disabled" });
        
        // If dark mode setting doesn't match our detection, try to fix it
        let detected_dark_mode = syntax::is_dark_mode_enabled();
        if detected_dark_mode != is_dark {
            println!("Warning: Dark mode setting ({}) doesn't match detected preference ({}), fixing...",
                     if is_dark { "enabled" } else { "disabled" },
                     if detected_dark_mode { "enabled" } else { "disabled" });
            settings.set_gtk_application_prefer_dark_theme(detected_dark_mode);
        }
    }

    // Force UI to update after a short delay
    let window_clone = window.clone();
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
        window_clone.queue_draw();
    });
}



/// Builds the user interface and sets up event handlers
fn build_ui(app: &Application, file_to_open: Option<PathBuf>) {
    // Debug output
    println!("build_ui called with file_to_open: {:?}", file_to_open);
    
    // Create the main application window using the template
    let window = ui::create_window(app);
    
    // Get references to the header bar action buttons
    let (new_button, open_button, save_main_button, save_menu_button, save_as_button, save_button, settings_button) = ui::create_header(&window);

    // Get references to UI components from the template
    let imp = window.imp();
    let menu_search_entry = imp.menu_search_entry.get();
    let terminal_button = imp.terminal_button.get();
    let up_button = imp.up_button.get();
    let path_box = imp.path_box.get();
    let volume_control_box = imp.volume_control_box.get();
    let volume_icon = imp.volume_icon.get();
    let global_volume_scale = imp.global_volume_scale.get();
    let volume_label = imp.volume_label.get();
    let file_list_box = imp.file_list_box.get();
    let _search_panel = imp.search_panel.get();
    let _search_bar_template = imp.search_bar.get();  // Template search bar (will be replaced)
    let refresh_button = imp.refresh_button.get();
    
    // Initialize the in-file search/replace UI
    // The search state creates its own SearchBar with all the controls
    let search_state = crate::search::get_search_state();
    
    // We need to replace the template's search_bar with the one from search_state
    // First, get the parent container (editor_notebook_box)
    if let Some(parent) = _search_bar_template.parent() {
        if let Some(editor_box) = parent.downcast_ref::<gtk4::Box>() {
            // Remove the template's empty search bar
            editor_box.remove(&_search_bar_template);
            // Insert the functional search bar at the beginning (before the notebook)
            editor_box.prepend(&search_state.search_bar);
        }
    }
    
    // Terminal setup will be done after editor_paned is created (see below)
    
    // Initialize the text editor components
    // Returns multiple widgets and associated state for the editor UI
    let (
        _initial_scrolled_window, // Container for the first tab's TextView with scrolling capability
        _initial_text_view,       // The editable text view widget for the first tab
        initial_text_buffer,      // Buffer holding the text content for the first tab
        _initial_tab_file_path_rc,// Reference-counted path for the first tab's file
        error_label,              // Label for displaying error messages to the user
        picture,                  // Widget for displaying images when opening image files
        current_dir,              // Current working directory for file operations
        editor_notebook,          // Tabbed container for managing multiple open files
        _initial_tab_widget,      // Container for custom tab label components
        initial_tab_actual_label, // Text label showing the file name in the tab
        initial_tab_close_button, // Button for closing the tab
        add_file_button           // Button for adding new file tabs
    ) = ui::create_text_view(&window);
    
    // Debug theme detection at startup
    println!("=== Theme Detection at Startup ===");
    syntax::debug_theme_detection();
    
    // Ensure the initial buffer gets the correct theme based on dark mode setting
    if let Some(source_buffer) = initial_text_buffer.dynamic_cast_ref::<sourceview5::Buffer>() {
        syntax::update_buffer_style_scheme(source_buffer);
        println!("Applied initial theme to first tab buffer");
    }

    // Create a mapping between notebook tab indexes and their corresponding file paths
    // This allows tracking which file is open in each tab - optimized for efficiency
    let file_path_manager = Rc::new(RefCell::new(HashMap::<u32, PathBuf>::new()));
    
    // Track the file path of the currently active tab
    let active_tab_path = Rc::new(RefCell::new(None::<PathBuf>));

    // Set up window close event handler to check for unsaved changes
    let window_clone_for_close = window.clone();
    let editor_notebook_clone_for_close = editor_notebook.clone();
    let file_path_manager_clone_for_close = file_path_manager.clone();
    let current_dir_clone_for_close = current_dir.clone();
    let app_for_close = app.clone();
    
    window.connect_close_request(move |_| {
        // Shutdown rust-analyzer before closing
        crate::linter::ui::shutdown_rust_analyzer();
        
        // Save the current folder before closing
        let folder = current_dir_clone_for_close.borrow().clone();
        let mut settings = settings::get_settings_mut();
        settings.set_last_folder(&folder);
        if let Err(e) = settings.save() {
            eprintln!("Failed to save settings: {}", e);
        }
        drop(settings); // Release the lock
        
        // Check if any tabs have unsaved changes (indicated by '*' in tab labels)
        let notebook = &editor_notebook_clone_for_close;
        let mut unsaved_files = Vec::new();
        
        // Iterate through all tabs to check for unsaved changes
        let num_pages = notebook.n_pages();
        for page_num in 0..num_pages {
            if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
                if let Some(tab_label_widget) = notebook.tab_label(&page_widget) {
                    if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                        if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<Label>().ok()) {
                            if label.text().starts_with('*') {
                                // Found an unsaved file - get its name
                                let filename = file_path_manager_clone_for_close.borrow()
                                    .get(&page_num)
                                    .and_then(|p| p.file_name().map(|s| s.to_string_lossy().into_owned()))
                                    .unwrap_or_else(|| "Untitled".to_string());
                                unsaved_files.push(filename);
                            }
                        }
                    }
                }
            }
        }
        
        // If there are unsaved files, show confirmation dialog
        if !unsaved_files.is_empty() {
            let message = if unsaved_files.len() == 1 {
                format!("You have unsaved changes in {}.\n\nAre you sure you want to close the application without saving?", unsaved_files[0])
            } else {
                format!("You have unsaved changes in {} files:\n• {}\n\nAre you sure you want to close the application without saving?", 
                        unsaved_files.len(), 
                        unsaved_files.join("\n• "))
            };
            
            let dialog = gtk4::MessageDialog::new(
                Some(&window_clone_for_close),
                gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                gtk4::MessageType::Warning,
                gtk4::ButtonsType::None,
                &message
            );
            
            dialog.add_buttons(&[
                ("Cancel", gtk4::ResponseType::Cancel),
                ("Close Anyway", gtk4::ResponseType::Yes),
            ]);
            
            dialog.set_default_response(gtk4::ResponseType::Cancel);
            
            let window_clone_for_dialog = window_clone_for_close.clone();
            let app_for_dialog = app_for_close.clone();
            
            dialog.connect_response(move |d, response| {
                d.close();
                match response {
                    gtk4::ResponseType::Yes => {
                        // User chose "Close Anyway" - shutdown rust-analyzer and quit
                        crate::linter::ui::shutdown_rust_analyzer();
                        println!("Closing anyway, quitting application...");
                        app_for_dialog.quit();
                        
                        // Force exit after a short delay
                        std::thread::spawn(|| {
                            std::thread::sleep(std::time::Duration::from_millis(300));
                            println!("Force exiting application");
                            std::process::exit(0);
                        });
                    }
                    _ => {
                        // User chose "Cancel" or closed dialog - close was already stopped
                        // Do nothing, the close request was already stopped
                    }
                }
            });
            
            dialog.present();
            return glib::Propagation::Stop; // Prevent window from closing until user decides
        }
        
        // No unsaved changes - quit the application
        println!("No unsaved changes, quitting application...");
        app_for_close.quit();
        
        // Force exit after a short delay if app doesn't quit normally
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(300));
            println!("Force exiting application");
            std::process::exit(0);
        });
        
        glib::Propagation::Stop
    });

    // Get references to UI components from template (already initialized earlier)
    // file_list_box, path_box, volume_control_box, etc. are already assigned
    
    // Get status bar components from the template
    let (_status_bar, status_label, linter_status_label, secondary_status_label) = ui::create_status_bar(&window);

    // Register linter status callback so the linter module can update the label
    let linter_status_label_clone = linter_status_label.clone();
    crate::linter::ui::set_linter_status_callback(move |text| {
        linter_status_label_clone.set_text(text);
    });
    
    // Set up volume control handler
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

    // Set up periodic checking for volume control visibility
    let volume_control_clone = volume_control_box.clone();
    let active_tab_path_for_volume = active_tab_path.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
        let current_path = active_tab_path_for_volume.borrow().clone();
        ui::update_volume_control_visibility_for_tab(&volume_control_clone, &current_path);
        glib::ControlFlow::Continue
    });

    // Set up click handler for the status label to show log history
    // We need to get the parent button from the status label
    let status_button = imp.status_button.get();
    let window_clone_for_status_click = window.clone();
    status_button.connect_clicked(move |_| {
        ui::show_log_history_popup(&window_clone_for_status_click);
    });

    // Register both status labels with the logging system
    crate::status_log::register_status_labels(&status_label, &secondary_status_label);

    // Initialize secondary status for the initial untitled tab with cursor position
    if let Some(text_view) = _initial_text_view.downcast_ref::<sourceview5::View>() {
        update_cursor_position_status(text_view, &secondary_status_label, "Untitled", None);
    } else {
        // For non-source views, just show empty status initially
        secondary_status_label.set_text("");
    }

    // Get references to paned components from the template
    let (_paned_content, paned, editor_paned, explorer_button, search_button, git_diff_button, sidebar_stack) = ui::create_paned(&window);
    
    // Setup terminal notebook now that we have editor_paned
    let terminal_notebook_template = imp.terminal_notebook.get();
    let add_terminal_button = imp.add_terminal_button.get();
    
    // Create the first terminal tab directly in the template notebook with paned support
    ui::terminal::add_terminal_tab_with_toggle(&terminal_notebook_template, None, &editor_paned);
    
    // Add diagnostics panel as a tab
    let diagnostics_panel = linter::diagnostics_panel::create_diagnostics_panel();
    let diagnostics_label = Label::new(Some("Diagnostics"));
    terminal_notebook_template.append_page(&diagnostics_panel, Some(&diagnostics_label));
    
    // Hide diagnostics panel by default (will be shown when Rust files are opened)
    diagnostics_panel.set_visible(false);
    
    // Set up callback to show/hide diagnostics panel based on linting activity
    let diagnostics_panel_weak = diagnostics_panel.downgrade();
    linter::ui::set_diagnostics_panel_callback(move |show| {
        if let Some(panel) = diagnostics_panel_weak.upgrade() {
            panel.set_visible(show);
        }
    });
    
    // Set up callback to update linter status label
    let linter_status_weak = linter_status_label.downgrade();
    linter::ui::set_linter_status_callback(move |status| {
        if let Some(label) = linter_status_weak.upgrade() {
            label.set_text(status);
        }
    });
    
    // Set up callback to show/hide linter status widget based on Rust file presence
    let linter_status_weak_for_visibility = linter_status_label.downgrade();
    linter::ui::set_linter_status_visibility_callback(move |show| {
        if let Some(label) = linter_status_weak_for_visibility.upgrade() {
            label.set_visible(show);
        }
    });
    
    // Initialize rust-analyzer at startup (it will keep running in background)
    linter::ui::initialize_rust_analyzer();
    
    // Check current directory for Rust files and update UI accordingly
    linter::ui::check_and_update_rust_ui(&current_dir.borrow());
    
    // Restore terminal visibility state from settings
    let saved_terminal_visible = settings::get_settings().get_terminal_visible();
    if !saved_terminal_visible {
        // Hide the terminal if it was hidden in the last session
        if let Some(end_child) = editor_paned.end_child() {
            end_child.set_visible(false);
            let max_pos = editor_paned.allocation().height();
            editor_paned.set_position(max_pos);
        }
    }
    
    // Connect the add terminal button click handler with paned support for toggling
    let terminal_notebook_for_button = terminal_notebook_template.clone();
    let editor_paned_for_button = editor_paned.clone();
    add_terminal_button.connect_clicked(move |_| {
        ui::terminal::add_terminal_tab_with_toggle(&terminal_notebook_for_button, None, &editor_paned_for_button);
    });
    
    // Set up theme settings based on system preferences
    if let Some(settings) = gtk4::Settings::default() {
        // Don't override the system preference - let GTK handle it naturally
        // This allows the app to respond to system theme changes automatically
        
        // Clone references to update editor views when theme changes
        let window_clone = window.clone();
        let terminal_notebook_clone = terminal_notebook_template.clone();
        
        // Connect to multiple theme-related signals to catch all possible theme changes
        let window_clone_2 = window_clone.clone();
        let terminal_notebook_clone_2 = terminal_notebook_clone.clone();
        let window_clone_3 = window_clone.clone();
        let terminal_notebook_clone_3 = terminal_notebook_clone.clone();
        
        // Primary signal for dark theme preference changes
        settings.connect_notify_local(
            Some("gtk-application-prefer-dark-theme"),
            move |_, _| {
                println!("Theme changed via gtk-application-prefer-dark-theme signal");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone);
                ui::terminal::update_all_terminal_themes(&terminal_notebook_clone);
            }
        );
        
        // Secondary signal for general theme name changes (catches more theme switches)
        settings.connect_notify_local(
            Some("gtk-theme-name"),
            move |_, _| {
                println!("Theme changed via gtk-theme-name signal");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone_2);
                ui::terminal::update_all_terminal_themes(&terminal_notebook_clone_2);
            }
        );
        
        // Monitor icon theme changes which often accompany theme switches
        settings.connect_notify_local(
            Some("gtk-icon-theme-name"),
            move |_, _| {
                println!("Icon theme changed - may indicate system theme change");
                syntax::sync_gtk_with_system_theme();
                update_all_buffer_themes(&window_clone_3);
                ui::terminal::update_all_terminal_themes(&terminal_notebook_clone_3);
            }
        );
        
        // Set up a GSettings monitor for GNOME/Ubuntu theme changes
        setup_gsettings_monitor(&window, &terminal_notebook_template);
    }
    
    // Restore active sidebar tab from settings
    let saved_sidebar_tab = settings::get_settings().get_active_sidebar_tab();
    let saved_sidebar_visible = settings::get_settings().get_sidebar_visible();
    
    if saved_sidebar_tab == "search" {
        search_button.set_active(saved_sidebar_visible);
        sidebar_stack.set_visible_child_name("search");
    } else if saved_sidebar_tab == "git-diff" {
        git_diff_button.set_active(saved_sidebar_visible);
        sidebar_stack.set_visible_child_name("git-diff");
    } else {
        explorer_button.set_active(saved_sidebar_visible);
        sidebar_stack.set_visible_child_name("explorer");
    }
    
    // Apply the sidebar visibility state
    if !saved_sidebar_visible {
        if let Some(start_child) = paned.start_child() {
            start_child.set_visible(false);
            paned.set_position(0);
        }
    }
    
    // Save sidebar tab when it changes
    let sidebar_stack_for_save = sidebar_stack.clone();
    sidebar_stack.connect_visible_child_notify(move |_| {
        let visible_child_name = sidebar_stack_for_save.visible_child_name();
        if let Some(name) = visible_child_name {
            let mut settings = settings::get_settings_mut();
            settings.set_active_sidebar_tab(&name);
            let _ = settings.save();
        }
    });
    
    // Setup explorer and search button toggle behavior
    let search_button_clone = search_button.clone();
    let git_diff_button_clone = git_diff_button.clone();
    let sidebar_stack_clone = sidebar_stack.clone();
    let paned_clone = paned.clone();
    explorer_button.connect_toggled(move |button| {
        if button.is_active() {
            search_button_clone.set_active(false);
            git_diff_button_clone.set_active(false);
            sidebar_stack_clone.set_visible_child_name("explorer");
            // Show the sidebar by setting the first child visible
            if let Some(start_child) = paned_clone.start_child() {
                start_child.set_visible(true);
                let settings = crate::settings::get_settings();
                let width = settings.get_file_panel_width();
                paned_clone.set_position(width);
            }
            // Save sidebar visible state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(true);
            let _ = settings.save();
        } else {
            // If the button is deactivated, hide the sidebar completely
            if let Some(start_child) = paned_clone.start_child() {
                // Save current width before hiding
                let current_width = paned_clone.position();
                if current_width > 0 {
                    let mut settings = crate::settings::get_settings_mut();
                    settings.set_file_panel_width(current_width);
                    let _ = settings.save();
                }
                start_child.set_visible(false);
                paned_clone.set_position(0);
            }
            // Save sidebar hidden state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(false);
            let _ = settings.save();
        }
    });
    
    let explorer_button_clone = explorer_button.clone();
    let git_diff_button_clone2 = git_diff_button.clone();
    let sidebar_stack_clone2 = sidebar_stack.clone();
    let paned_clone2 = paned.clone();
    search_button.connect_toggled(move |button| {
        if button.is_active() {
            explorer_button_clone.set_active(false);
            git_diff_button_clone2.set_active(false);
            sidebar_stack_clone2.set_visible_child_name("search");
            // Show the sidebar by setting the first child visible
            if let Some(start_child) = paned_clone2.start_child() {
                start_child.set_visible(true);
                let settings = crate::settings::get_settings();
                let width = settings.get_file_panel_width();
                paned_clone2.set_position(width);
            }
            // Save sidebar visible state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(true);
            let _ = settings.save();
        } else {
            // If the button is deactivated, hide the sidebar completely
            if let Some(start_child) = paned_clone2.start_child() {
                // Save current width before hiding
                let current_width = paned_clone2.position();
                if current_width > 0 {
                    let mut settings = crate::settings::get_settings_mut();
                    settings.set_file_panel_width(current_width);
                    let _ = settings.save();
                }
                start_child.set_visible(false);
                paned_clone2.set_position(0);
            }
            // Save sidebar hidden state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(false);
            let _ = settings.save();
        }
    });
    
    let explorer_button_clone2 = explorer_button.clone();
    let search_button_clone2 = search_button.clone();
    let sidebar_stack_clone3 = sidebar_stack.clone();
    let paned_clone3 = paned.clone();
    git_diff_button.connect_toggled(move |button| {
        if button.is_active() {
            explorer_button_clone2.set_active(false);
            search_button_clone2.set_active(false);
            sidebar_stack_clone3.set_visible_child_name("git-diff");
            // Show the sidebar by setting the first child visible
            if let Some(start_child) = paned_clone3.start_child() {
                start_child.set_visible(true);
                let settings = crate::settings::get_settings();
                let width = settings.get_file_panel_width();
                paned_clone3.set_position(width);
            }
            // Save sidebar visible state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(true);
            let _ = settings.save();
        } else {
            // If the button is deactivated, hide the sidebar completely
            if let Some(start_child) = paned_clone3.start_child() {
                // Save current width before hiding
                let current_width = paned_clone3.position();
                if current_width > 0 {
                    let mut settings = crate::settings::get_settings_mut();
                    settings.set_file_panel_width(current_width);
                    let _ = settings.save();
                }
                start_child.set_visible(false);
                paned_clone3.set_position(0);
            }
            // Save sidebar hidden state
            let mut settings = crate::settings::get_settings_mut();
            settings.set_sidebar_visible(false);
            let _ = settings.save();
        }
    });
    
    // Get the search panel from the sidebar stack and populate it with global search UI
    if let Some(search_panel_widget) = sidebar_stack.child_by_name("search") {
        if let Some(search_panel_box) = search_panel_widget.downcast_ref::<gtk4::Box>() {
            // Create the full global search panel
            let global_search_panel = ui::global_search::create_global_search_panel(
                window.upcast_ref::<ApplicationWindow>(),
                &current_dir,
                &editor_notebook,
                &file_path_manager,
                &active_tab_path,
                &save_button,
                &save_as_button,
                &file_list_box,
            );
            
            // Clear placeholder and add the real search panel
            while let Some(child) = search_panel_box.first_child() {
                search_panel_box.remove(&child);
            }
            search_panel_box.append(&global_search_panel);
        }
    }
    
    // Get the git diff panel from the sidebar stack and populate it
    if let Some(git_diff_panel_widget) = sidebar_stack.child_by_name("git-diff") {
        if let Some(git_diff_panel_box) = git_diff_panel_widget.downcast_ref::<gtk4::Box>() {
            // Create the full git diff panel
            let git_diff_panel = ui::git_diff::create_git_diff_panel(
                window.upcast_ref::<ApplicationWindow>(),
                &current_dir,
                &editor_notebook,
                &file_path_manager,
                &active_tab_path,
            );
            
            // Clear placeholder and add the real git diff panel
            while let Some(child) = git_diff_panel_box.first_child() {
                git_diff_panel_box.remove(&child);
            }
            git_diff_panel_box.append(&git_diff_panel);
        }
    }
    
    // Set up keyboard shortcuts for common operations (including Ctrl+B, Ctrl+Shift+E/F/G, and Ctrl+L for path editing)
    utils::setup_keyboard_shortcuts(
        &window, 
        &save_button, 
        &open_button, 
        &new_button, 
        &save_as_button, 
        None,
        Some(&editor_notebook),
        Some(&path_box),
        Some(&current_dir),
        Some(&file_list_box),
        Some(&active_tab_path),
        Some(&file_path_manager),
        Some(&explorer_button),
        Some(&search_button),
        Some(&git_diff_button),
        Some(&sidebar_stack),
        Some(&editor_paned),
        Some(&terminal_notebook_template),
    );
    
    // Set up modification tracking for the initial tab
    // This adds a "*" indicator to the tab label when content has been modified
    let initial_tab_actual_label_clone = initial_tab_actual_label.clone();
    let initial_buffer_clone_for_dirty_track = initial_text_buffer.clone();
    
    // Connect to the buffer's changed signal to detect modifications
    initial_text_buffer.connect_changed(move |_buffer| {
        // Mark text editor as active when user actually types/modifies content
        handlers::LAST_ACTIVE_AREA.with(|area| {
            *area.borrow_mut() = handlers::LastActiveArea::TextEditor;
        });
        
        // Get the current text content from the buffer
        let text_content = initial_buffer_clone_for_dirty_track.text(
            &initial_buffer_clone_for_dirty_track.start_iter(),
            &initial_buffer_clone_for_dirty_track.end_iter(),
            false
        );
        
        // Get the current tab label text
        let label_text = initial_tab_actual_label_clone.text();
        
        // If the file was previously unmodified and now has content, mark as modified
        if label_text == "Untitled" && !text_content.is_empty() {
            initial_tab_actual_label_clone.set_text("*Untitled");
        } 
        // If the file was previously modified but now is empty, remove the modified indicator
        else if label_text.starts_with('*') && text_content.is_empty() && label_text == "*Untitled" {
            initial_tab_actual_label_clone.set_text("Untitled");
        }
    });

    // Set up cursor position tracking for the initial tab
    if let Some(text_view) = _initial_text_view.downcast_ref::<sourceview5::View>() {
        let text_view_clone = text_view.clone();
        let secondary_status_clone = secondary_status_label.clone();
        
        // Connect to buffer's cursor position changed signal
        let buffer = text_view.buffer();
        buffer.connect_notify_local(Some("cursor-position"), move |_, _| {
            update_cursor_position_status(&text_view_clone, &secondary_status_clone, "Untitled", None);
        });
        
        // Also connect to mark-set signal which fires when cursor moves
        let text_view_clone_2 = text_view.clone();
        let secondary_status_clone_2 = secondary_status_label.clone();
        buffer.connect_mark_set(move |_, _, mark| {
            if mark.name().as_deref() == Some("insert") { // "insert" is the cursor mark
                update_cursor_position_status(&text_view_clone_2, &secondary_status_clone_2, "Untitled", None);
            }
        });
        
        // Initial cursor position update
        update_cursor_position_status(text_view, &secondary_status_label, "Untitled", None);
    }

    // Prepare dependencies needed for creating a new tab
    // This structure holds references to all components needed when creating or managing tabs
    // It's particularly used when closing tabs to ensure a new one is created if the last tab is closed
    let deps_for_new_tab_creation = handlers::NewTabDependencies {
        editor_notebook: editor_notebook.clone(),      // The main tabbed container
        active_tab_path: active_tab_path.clone(),      // Currently active file path
        file_path_manager: file_path_manager.clone(),  // Tab-to-path mapping
        window: window.clone().upcast::<ApplicationWindow>(),  // Main application window (upcast from DvopWindow)
        file_list_box: file_list_box.clone(),          // File browser list
        current_dir: current_dir.clone(),              // Current directory for file operations
        save_button: save_button.clone(),              // Save button reference
        save_as_button: save_as_button.clone(),        // Save As button reference
        _save_menu_button: Some(save_menu_button.clone()), // Split button menu component (currently unused)
    };

    // Now that dependencies are available, set up right-click menu for the initial tab
    ui::setup_tab_right_click(
        &_initial_tab_widget,
        &editor_notebook,
        &window.clone().upcast::<ApplicationWindow>(),
        &file_path_manager,
        &active_tab_path,
        &current_dir,
        &file_list_box,
        Some(deps_for_new_tab_creation.clone()),
    );

    // Set up the close button handler for the initial tab
    // Clone all necessary references for the closure
    let initial_tab_close_button_clone = initial_tab_close_button.clone();
    let editor_notebook_clone_for_initial_close = editor_notebook.clone();
    let window_clone_for_initial_close = window.clone();
    let file_path_manager_clone_for_initial_close = file_path_manager.clone();
    let active_tab_path_clone_for_initial_close = active_tab_path.clone();
    let current_dir_clone_for_initial_close = current_dir.clone();
    let file_list_box_clone_for_initial_close = file_list_box.clone();

    // Connect to the close button's clicked signal
    initial_tab_close_button_clone.connect_clicked(move |_| {
        // Verify the notebook still has pages before attempting to close one
        if editor_notebook_clone_for_initial_close.n_pages() > 0 { 
            // Check if the first tab (usually the initial one) exists
            if let Some(_page_widget) = editor_notebook_clone_for_initial_close.nth_page(Some(0)) {
                // Handle the tab close request with proper cleanup and potential new tab creation
                handlers::handle_close_tab_request(
                    &editor_notebook_clone_for_initial_close,
                    0, // Tab index 0 (first tab)
                    &window_clone_for_initial_close,
                    &file_path_manager_clone_for_initial_close,
                    &active_tab_path_clone_for_initial_close,
                    &current_dir_clone_for_initial_close,
                    &file_list_box_clone_for_initial_close,
                    Some(deps_for_new_tab_creation.clone()) // Dependencies for creating a new tab if needed
                );
            }
        }
    });

    // Track the current file selection source for click-outside detection
    let current_selection_source = Rc::new(RefCell::new(utils::FileSelectionSource::TabSwitch));
    
    // Note: Removed click-outside detection as it was interfering with normal file selection
    // The file manager highlighting will revert naturally when tabs are switched

    // The main container, paned content, and status bar are now part of the template, no need to append them

    // Define GIO actions for save operations to be used by the menu
    let save_action = gio::SimpleAction::new("save", None);
    let save_as_action = gio::SimpleAction::new("save-as", None);
    
    // Prepare button references for the action handlers
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    
    // Connect the save action to trigger the save button's click event
    // This allows menu items to reuse existing save functionality
    let save_button_clone_for_action = save_button_clone.clone();
    save_action.connect_activate(move |_, _| {
        save_button_clone_for_action.emit_clicked();
    });
    
    // Connect the save-as action to trigger the save-as button's click event
    let save_as_button_clone_for_action = save_as_button_clone.clone();
    save_as_action.connect_activate(move |_, _| {
        save_as_button_clone_for_action.emit_clicked();
    });
    
    // Define additional menu actions
    let new_file_action = gio::SimpleAction::new("new-file", None);
    let open_action = gio::SimpleAction::new("open", None);
    let close_tab_action = gio::SimpleAction::new("close-tab", None);
    let close_all_tabs_action = gio::SimpleAction::new("close-all-tabs", None);
    let find_action = gio::SimpleAction::new("find", None);
    let find_replace_action = gio::SimpleAction::new("find-replace", None);
    let preferences_action = gio::SimpleAction::new("preferences", None);
    let toggle_explorer_action = gio::SimpleAction::new("toggle-explorer", None);
    let toggle_search_action = gio::SimpleAction::new("toggle-search", None);
    let toggle_git_action = gio::SimpleAction::new("toggle-git", None);
    let refresh_action = gio::SimpleAction::new("refresh", None);
    let new_terminal_action = gio::SimpleAction::new("new-terminal", None);
    let toggle_terminal_action = gio::SimpleAction::new("toggle-terminal", None);
    let about_action = gio::SimpleAction::new("about", None);
    
    // Clone necessary references for action handlers
    let add_file_button_clone = add_file_button.clone();
    let open_button_clone = open_button.clone();
    let editor_notebook_clone_for_close = editor_notebook.clone();
    let file_path_manager_clone_for_close = file_path_manager.clone();
    let editor_notebook_clone_for_close_all = editor_notebook.clone();
    let file_path_manager_clone_for_close_all = file_path_manager.clone();
    let settings_button_clone = settings_button.clone();
    let explorer_button_clone_for_action = explorer_button.clone();
    let search_button_clone_for_action = search_button.clone();
    let git_diff_button_clone_for_action = git_diff_button.clone();
    let refresh_button_clone = refresh_button.clone();
    let terminal_notebook_clone_for_new_terminal = terminal_notebook_template.clone();
    let editor_paned_clone_for_new_terminal = editor_paned.clone();
    let window_clone_for_about = window.clone();
    
    // Connect new file action
    new_file_action.connect_activate(move |_, _| {
        add_file_button_clone.emit_clicked();
    });
    
    // Connect open action
    open_action.connect_activate(move |_, _| {
        open_button_clone.emit_clicked();
    });
    
    // Connect close tab action
    close_tab_action.connect_activate(move |_, _| {
        if let Some(current_page) = editor_notebook_clone_for_close.current_page() {
            // Remove from file path manager
            file_path_manager_clone_for_close.borrow_mut().remove(&current_page);
            // Close the tab
            editor_notebook_clone_for_close.remove_page(Some(current_page));
        }
    });
    
    // Connect close all tabs action
    close_all_tabs_action.connect_activate(move |_, _| {
        let num_pages = editor_notebook_clone_for_close_all.n_pages();
        // Close all tabs from last to first to avoid index issues
        for _ in 0..num_pages {
            if editor_notebook_clone_for_close_all.n_pages() > 0 {
                let last_page = editor_notebook_clone_for_close_all.n_pages() - 1;
                file_path_manager_clone_for_close_all.borrow_mut().remove(&last_page);
                editor_notebook_clone_for_close_all.remove_page(Some(last_page));
            }
        }
        // Clear the file path manager
        file_path_manager_clone_for_close_all.borrow_mut().clear();
    });
    
    // Connect find action - opens in-file find (without replace)
    let editor_notebook_for_find = editor_notebook.clone();
    find_action.connect_activate(move |_, _| {
        if let Some((text_view, text_buffer)) = handlers::get_active_text_view_and_buffer(&editor_notebook_for_find) {
            if let Ok(source_buffer) = text_buffer.downcast::<sourceview5::Buffer>() {
                if let Ok(source_view) = text_view.downcast::<sourceview5::View>() {
                    crate::search::show_find_only_for_buffer(Some(&source_buffer), Some(&source_view));
                } else {
                    crate::search::show_find_only_for_buffer(Some(&source_buffer), None);
                }
            } else {
                crate::search::show_find_only_for_buffer(None, None);
            }
        } else {
            crate::search::show_find_only_for_buffer(None, None);
        }
    });
    
    // Connect find and replace action - opens in-file find/replace (same as find now)
    let editor_notebook_for_replace = editor_notebook.clone();
    find_replace_action.connect_activate(move |_, _| {
        if let Some((text_view, text_buffer)) = handlers::get_active_text_view_and_buffer(&editor_notebook_for_replace) {
            if let Ok(source_buffer) = text_buffer.downcast::<sourceview5::Buffer>() {
                if let Ok(source_view) = text_view.downcast::<sourceview5::View>() {
                    crate::search::show_search_for_buffer(Some(&source_buffer), Some(&source_view));
                } else {
                    crate::search::show_search_for_buffer(Some(&source_buffer), None);
                }
            } else {
                crate::search::show_search_for_buffer(None, None);
            }
        } else {
            crate::search::show_search_for_buffer(None, None);
        }
    });
    
    // Connect preferences action
    preferences_action.connect_activate(move |_, _| {
        settings_button_clone.emit_clicked();
    });
    
    // Connect toggle explorer action
    toggle_explorer_action.connect_activate(move |_, _| {
        explorer_button_clone_for_action.set_active(!explorer_button_clone_for_action.is_active());
    });
    
    // Connect toggle search action
    toggle_search_action.connect_activate(move |_, _| {
        search_button_clone_for_action.set_active(!search_button_clone_for_action.is_active());
    });
    
    // Connect toggle git action
    toggle_git_action.connect_activate(move |_, _| {
        git_diff_button_clone_for_action.set_active(!git_diff_button_clone_for_action.is_active());
    });
    
    // Connect refresh action
    refresh_action.connect_activate(move |_, _| {
        refresh_button_clone.emit_clicked();
    });
    
    // Connect new terminal action
    new_terminal_action.connect_activate(move |_, _| {
        // Show terminal if hidden
        if let Some(end_child) = editor_paned_clone_for_new_terminal.end_child() {
            if !end_child.is_visible() {
                end_child.set_visible(true);
                let max_pos = editor_paned_clone_for_new_terminal.allocation().height();
                editor_paned_clone_for_new_terminal.set_position((max_pos as f64 * 0.6) as i32);
                // Save terminal visible state
                let mut settings = crate::settings::get_settings_mut();
                settings.set_terminal_visible(true);
                let _ = settings.save();
            }
        }
        // Add a new terminal tab
        ui::terminal::add_terminal_tab_with_toggle(&terminal_notebook_clone_for_new_terminal, None, &editor_paned_clone_for_new_terminal);
    });
    
    // Connect toggle terminal action
    let terminal_notebook_for_toggle = terminal_notebook_template.clone();
    let editor_paned_for_toggle = editor_paned.clone();
    toggle_terminal_action.connect_activate(move |_, _| {
        if let Some(end_child) = editor_paned_for_toggle.end_child() {
            if end_child.is_visible() {
                // Terminal is visible, hide it completely
                end_child.set_visible(false);
                let max_pos = editor_paned_for_toggle.allocation().height();
                editor_paned_for_toggle.set_position(max_pos);
                // Save terminal hidden state
                let mut settings = crate::settings::get_settings_mut();
                settings.set_terminal_visible(false);
                let _ = settings.save();
            } else {
                // Terminal is hidden, show it
                end_child.set_visible(true);
                let max_pos = editor_paned_for_toggle.allocation().height();
                editor_paned_for_toggle.set_position((max_pos as f64 * 0.6) as i32);
                // If there are no terminals, create one
                if terminal_notebook_for_toggle.n_pages() == 0 {
                    ui::terminal::add_terminal_tab_with_toggle(&terminal_notebook_for_toggle, None, &editor_paned_for_toggle);
                }
                // Save terminal visible state
                let mut settings = crate::settings::get_settings_mut();
                settings.set_terminal_visible(true);
                let _ = settings.save();
            }
        }
    });
    
    // Connect about action
    about_action.connect_activate(move |_, _| {
        let about_dialog = gtk4::AboutDialog::builder()
            .program_name("Dvop")
            .version("0.1.0")
            .comments("A modern, lightweight code editor")
            .website("https://github.com/Ludo000/dvop")
            .authors(vec!["Ludovic Scholz".to_string()])
            .copyright("Copyright © 2024-2025 Ludovic Scholz")
            .license_type(gtk4::License::Gpl30)
            .transient_for(&window_clone_for_about)
            .modal(true)
            .build();
        about_dialog.present();
    });
    
    // Register the actions with the application window (upcast first)
    // This makes them available to be triggered by menu items  
    let window_as_app_window: &ApplicationWindow = window.upcast_ref();
    window_as_app_window.add_action(&save_action);
    window_as_app_window.add_action(&save_as_action);
    window_as_app_window.add_action(&new_file_action);
    window_as_app_window.add_action(&open_action);
    window_as_app_window.add_action(&close_tab_action);
    window_as_app_window.add_action(&close_all_tabs_action);
    window_as_app_window.add_action(&find_action);
    window_as_app_window.add_action(&find_replace_action);
    window_as_app_window.add_action(&preferences_action);
    window_as_app_window.add_action(&toggle_explorer_action);
    window_as_app_window.add_action(&toggle_search_action);
    window_as_app_window.add_action(&toggle_git_action);
    window_as_app_window.add_action(&refresh_action);
    window_as_app_window.add_action(&new_terminal_action);
    window_as_app_window.add_action(&toggle_terminal_action);
    window_as_app_window.add_action(&about_action);
    
    // Set up menu search functionality
    setup_menu_search(&menu_search_entry, window_as_app_window);
    
    // Set up direct save functionality for the main save button
    // Instead of circular references between buttons, implement the save logic directly here
    
    // Clone references needed for the save operation
    let editor_notebook_clone = editor_notebook.clone();
    let _active_tab_path_clone = active_tab_path.clone(); // Unused but kept for potential future use
    let file_path_manager_clone = file_path_manager.clone();
    let _window_clone = window.clone(); // Unused but kept for potential future use
    let _file_list_box_clone = file_list_box.clone(); // Unused but kept for potential future use
    let _current_dir_clone = current_dir.clone(); // Unused but kept for potential future use
    let save_as_button_clone = save_as_button.clone();
    
    save_main_button.connect_clicked(move |_| {
        // Log save operation start
        crate::status_log::log_info("Saving file...");
        
        // Implementation of the save functionality
        if let Some((_active_text_view, active_buffer)) = handlers::get_active_text_view_and_buffer(&editor_notebook_clone) {
            // Get the current tab index
            let current_page_num_opt = editor_notebook_clone.current_page();
            if current_page_num_opt.is_none() { 
                crate::status_log::log_error("No active tab found");
                return; 
            }
            let current_page_num = current_page_num_opt.unwrap();

            // Look up the file path associated with this tab
            let path_to_save_opt = file_path_manager_clone.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                // Check if this is a supported file type for saving
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
                    // Attempt to save the file
                    match std::fs::File::create(&path_to_save) {
                        Ok(mut file) => {
                            // Extract the text content from the buffer
                            let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                            
                            // Write the content to the file and update UI if successful
                            match file.write_all(text.as_bytes()) {
                                Ok(_) => {
                                    // Update tab label to remove the modified indicator (*)
                                    handlers::update_tab_label_after_save(&editor_notebook_clone, current_page_num, Some(&path_to_save.file_name().unwrap_or_default().to_string_lossy()), false);
                                    
                                    let filename = path_to_save.file_name()
                                        .map(|name| name.to_string_lossy().into_owned())
                                        .unwrap_or_else(|| "file".to_string());
                                    crate::status_log::log_success(&format!("Saved {}", filename));
                                }
                                Err(e) => {
                                    crate::status_log::log_error(&format!("Failed to write file: {}", e));
                                }
                            }
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!("Failed to create file: {}", e));
                        }
                    }
                } else {
                    crate::status_log::log_error("File type not supported for saving");
                }
            } else {
                // If no path is associated with this tab (new unsaved file),
                // redirect to the Save As functionality
                crate::status_log::log_info("Opening Save As dialog...");
                save_as_button_clone.emit_clicked();
            }
        } else {
            crate::status_log::log_error("No active text view found");
        }
    });

    // The window titlebar and main container are now part of the template, no need to set them

    // Initialize the file browser panel with the current directory contents
    // Initially there's no active file selection since we start with an empty "Untitled" tab
    utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
    
    // Initialize the path bar to show the current directory
    utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
    
    // Set up the save menu button visibility for the default text plain content type
    // This is appropriate for the initial empty "Untitled" document
    utils::update_save_menu_button_visibility(&save_menu_button, Some(mime_guess::mime::TEXT_PLAIN_UTF_8));

    // Set up the tab switching handler to update UI state when changing tabs
    // Clone all required references for use in the closure
    let file_path_manager_clone_for_switch = file_path_manager.clone();
    let active_tab_path_clone_for_switch = active_tab_path.clone();
    let file_list_box_clone_for_switch = file_list_box.clone();
    let current_dir_clone_for_switch = current_dir.clone();
    let save_button_clone_for_switch = save_button.clone();
    let save_as_button_clone_for_switch = save_as_button.clone();
    let save_menu_button_clone_for_switch = save_menu_button.clone();
    let path_box_clone_for_switch = path_box.clone();
    let secondary_status_label_clone = secondary_status_label.clone();
    let current_selection_source_clone_for_switch = current_selection_source.clone();
    let volume_control_clone_for_switch = volume_control_box.clone();

    // Connect to the notebook's switch-page signal
    editor_notebook.connect_switch_page(move |notebook, _page, page_num| {
        // Reset selection source to TabSwitch when switching tabs
        // This ensures file manager highlighting reverts to subtle style
        *current_selection_source_clone_for_switch.borrow_mut() = utils::FileSelectionSource::TabSwitch;
        
        // Refresh diagnostics panel when switching tabs
        crate::linter::ui::refresh_diagnostics_panel();
        
        // Retrieve the file path associated with the newly selected tab
        let new_active_path = { 
            // Use a separate scope to limit the borrow duration
            file_path_manager_clone_for_switch.borrow().get(&page_num).cloned()
        };

        // Update the active tab path reference
        *active_tab_path_clone_for_switch.borrow_mut() = new_active_path.clone();

        // Update the secondary status label with file information
        if let Some(file_path) = &new_active_path {
            let filename = file_path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "Unknown".to_string());
            
            // Try to get the text view for cursor position
            if let Some((text_view, _)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) {
                if let Some(source_view) = text_view.downcast_ref::<sourceview5::View>() {
                    update_cursor_position_status(source_view, &secondary_status_label_clone, &filename, Some(file_path));
                    
                    // Set up cursor position tracking for this tab
                    let secondary_status_clone = secondary_status_label_clone.clone();
                    let filename_clone = filename.clone();
                    let file_path_clone = file_path.clone();
                    let source_view_clone = source_view.clone();
                    
                    let buffer = source_view.buffer();
                    buffer.connect_notify_local(Some("cursor-position"), move |_, _| {
                        update_cursor_position_status(&source_view_clone, &secondary_status_clone, &filename_clone, Some(&file_path_clone));
                    });
                    
                    // Also connect to mark-set signal for more reliable cursor tracking
                    let secondary_status_clone_2 = secondary_status_label_clone.clone();
                    let filename_clone_2 = filename.clone();
                    let file_path_clone_2 = file_path.clone();
                    let source_view_clone_2 = source_view.clone();
                    buffer.connect_mark_set(move |_, _, mark| {
                        if mark.name().as_deref() == Some("insert") {
                            update_cursor_position_status(&source_view_clone_2, &secondary_status_clone_2, &filename_clone_2, Some(&file_path_clone_2));
                        }
                    });
                } else {
                    // Fallback for non-source views
                    secondary_status_label_clone.set_text(&filename);
                }
            } else {
                // No text view (e.g., image file)
                secondary_status_label_clone.set_text(&filename);
            }
            
            crate::status_log::log_info(&format!("Switched to {}", filename));
        } else {
            // Handle untitled tab
            if let Some((text_view, _)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) {
                if let Some(source_view) = text_view.downcast_ref::<sourceview5::View>() {
                    update_cursor_position_status(source_view, &secondary_status_label_clone, "Untitled", None);
                    
                    // Set up cursor position tracking for this tab
                    let secondary_status_clone = secondary_status_label_clone.clone();
                    let source_view_clone = source_view.clone();
                    
                    let buffer = source_view.buffer();
                    buffer.connect_notify_local(Some("cursor-position"), move |_, _| {
                        update_cursor_position_status(&source_view_clone, &secondary_status_clone, "Untitled", None);
                    });
                    
                    // Also connect to mark-set signal for more reliable cursor tracking
                    let secondary_status_clone_2 = secondary_status_label_clone.clone();
                    let source_view_clone_2 = source_view.clone();
                    buffer.connect_mark_set(move |_, _, mark| {
                        if mark.name().as_deref() == Some("insert") {
                            update_cursor_position_status(&source_view_clone_2, &secondary_status_clone_2, "Untitled", None);
                        }
                    });
                } else {
                    // For non-source views, just show empty status
                    secondary_status_label_clone.set_text("");
                }
            } else {
                // For non-source views, just show empty status
                secondary_status_label_clone.set_text("");
            }
            
            crate::status_log::log_info("Switched to Untitled");
        }

        // If the focused tab has a file, update current directory to match the file's directory
        if let Some(file_path) = &new_active_path {
            if let Some(parent_dir) = file_path.parent() {
                let parent_path = parent_dir.to_path_buf();
                // Only update if the directory is different from current
                if *current_dir_clone_for_switch.borrow() != parent_path {
                    *current_dir_clone_for_switch.borrow_mut() = parent_path.clone();
                    
                    // Update the file list to show the new directory
                    utils::update_file_list(&file_list_box_clone_for_switch, &current_dir_clone_for_switch.borrow(), &new_active_path, utils::FileSelectionSource::TabSwitch);
                    
                    // Update the path buttons to reflect the new current directory
                    utils::update_path_buttons(&path_box_clone_for_switch, &current_dir_clone_for_switch, &file_list_box_clone_for_switch, &active_tab_path_clone_for_switch);
                    
                    // Check for Rust files and update linter UI visibility
                    crate::linter::ui::check_and_update_rust_ui(&parent_path);
                    
                    return; // Exit early since we've already updated the file list
                }
            }
        }

        // Rebind global search context to the new tab's buffer if search UI is currently visible
        {
            let search_state = crate::search::get_search_state();
            if search_state.search_bar.is_search_mode() {
                if let Some((text_view, text_buffer)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) {
                    if let (Ok(source_view), Ok(source_buffer)) = (text_view.downcast::<sourceview5::View>(), text_buffer.downcast::<sourceview5::Buffer>()) {
                        search_state.rebind_buffer(&source_buffer, Some(&source_view));
                    }
                }
            }
        }

        // Update file list highlighting to show the current file (only if directory didn't change)
        let current_dir_path_clone = current_dir_clone_for_switch.borrow().clone(); 
        utils::update_file_list(&file_list_box_clone_for_switch, &current_dir_path_clone, &new_active_path, utils::FileSelectionSource::TabSwitch);

        // Determine the MIME type from the file path
        let mut mime_type = new_active_path.as_ref()
            .map(|p| mime_guess::from_path(p).first_or_octet_stream())
            .unwrap_or(mime_guess::mime::TEXT_PLAIN_UTF_8); // Default to plain text for unsaved files
        
        // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
        // but should be treated as TypeScript files (text/plain)
        if let Some(file_path) = &new_active_path {
            if let Some(ext) = file_path.extension() {
                if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                    // Override MIME type for TypeScript files
                    mime_type = mime_guess::mime::TEXT_PLAIN;
                }
            }
        }
        
        // Check if the current tab has a text view (editable content) or is an image tab
        if let Some((_, _)) = handlers::get_text_view_and_buffer_for_page(notebook, page_num) {
            // This is a text tab - enable save functionality
            utils::update_save_buttons_visibility(
                &save_button_clone_for_switch, 
                &save_as_button_clone_for_switch, 
                Some(mime_type.clone())
            );
            
            utils::update_save_menu_button_visibility(
                &save_menu_button_clone_for_switch, 
                Some(mime_type)
            );
        } else if let Some(page) = notebook.nth_page(Some(page_num)) {
            // Handle cases where the tab contains non-text content (e.g., image)
            if let Some(scrolled_window) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                if let Some(child) = scrolled_window.child() {
                    // Check if the child is a Picture widget (image content)
                    if child.is::<gtk4::Picture>() || mime_type.type_() == "image" {
                        // This is an image tab - disable save functionality
                        utils::update_save_buttons_visibility(
                            &save_button_clone_for_switch, 
                            &save_as_button_clone_for_switch, 
                            Some(mime_guess::mime::IMAGE_PNG) // Use any image MIME type to trigger hiding
                        );
                        
                        utils::update_save_menu_button_visibility(
                            &save_menu_button_clone_for_switch, 
                            Some(mime_guess::mime::IMAGE_PNG)
                        );
                    } else {
                        // Other non-text content, use default behavior based on MIME type
                        utils::update_save_buttons_visibility(
                            &save_button_clone_for_switch, 
                            &save_as_button_clone_for_switch, 
                            Some(mime_type.clone())
                        );
                        
                        utils::update_save_menu_button_visibility(
                            &save_menu_button_clone_for_switch, 
                            Some(mime_type)
                        );
                    }
                }
            }
        } else {
            // Fallback: disable save functionality if we can't determine content type
            utils::update_save_buttons_visibility(
                &save_button_clone_for_switch, 
                &save_as_button_clone_for_switch, 
                None
            );
            utils::update_save_menu_button_visibility(
                &save_menu_button_clone_for_switch, 
                None
            );
        }
        
        // Update volume control visibility based on the new active tab
        ui::update_volume_control_visibility_for_tab(&volume_control_clone_for_switch, &new_active_path);
    });

    // Set up all button event handlers and their associated functionality
    handlers::setup_button_handlers(
        &new_button,           // New file button
        &open_button,          // Open file button
        &save_button,          // Save button (hidden, used programmatically)
        &save_as_button,       // Save As button
        &initial_text_buffer,  // Text buffer for the initial tab
        &file_path_manager,    // Mapping of tabs to file paths
        &active_tab_path,      // Currently active file path
        &window,               // Main application window
        &current_dir,          // Current working directory
        &file_list_box,        // File browser list box
        &editor_notebook,      // Tabbed notebook for editor
        &error_label,          // Label for displaying errors
        &picture,              // Widget for displaying images
        &up_button,            // Navigation button for parent directory
        &file_list_box,        // File list box (duplicate param for historical reasons)
        Some(&save_menu_button), // Split button menu component
        Some(&path_box),        // Path box for the status bar with clickable segments
        &current_selection_source, // Track selection source for click-outside detection
    );

    // Set up file list refresh callback for drag and drop operations
    let file_list_box_for_refresh = file_list_box.clone();
    let current_dir_for_refresh = current_dir.clone();
    let active_tab_path_for_refresh = active_tab_path.clone();
    utils::set_file_list_refresh_callback(move || {
        utils::update_file_list(
            &file_list_box_for_refresh,
            &current_dir_for_refresh.borrow(),
            &active_tab_path_for_refresh.borrow(),
            utils::FileSelectionSource::TabSwitch
        );
    });

    // Set up tab path update callback for when files are moved
    let file_path_manager_for_path_update = file_path_manager.clone();
    let active_tab_path_for_path_update = active_tab_path.clone();
    let editor_notebook_for_path_update = editor_notebook.clone();
    utils::set_tab_path_update_callback(move |old_path: &PathBuf, new_path: &PathBuf| {
        // Update file_path_manager entries that match the old path
        let mut manager = file_path_manager_for_path_update.borrow_mut();
        let mut updated_entries = Vec::new();
        
        for (&page_num, path) in manager.iter() {
            if path == old_path {
                updated_entries.push(page_num);
            }
        }
        
        // Update the entries
        for page_num in updated_entries {
            manager.insert(page_num, new_path.clone());
            
            // Update tab label to reflect the new file name if needed
            if let Some(new_file_name) = new_path.file_name() {
                handlers::update_tab_label_after_save(
                    &editor_notebook_for_path_update, 
                    page_num, 
                    Some(&new_file_name.to_string_lossy()), 
                    false
                );
            }
        }
        
        // Update active_tab_path if it matches the old path
        {
            let mut active_path = active_tab_path_for_path_update.borrow_mut();
            if let Some(ref current_active) = *active_path {
                if current_active == old_path {
                    *active_path = Some(new_path.clone());
                }
            }
        }
    });

    // Set up the refresh button handler to update the file list
    let file_list_box_for_refresh = file_list_box.clone();
    let current_dir_for_refresh = current_dir.clone();
    let active_tab_path_for_refresh = active_tab_path.clone();
    let refresh_button = imp.refresh_button.get();
    refresh_button.connect_clicked(move |_| {
        utils::update_file_list(
            &file_list_box_for_refresh,
            &current_dir_for_refresh.borrow(),
            &active_tab_path_for_refresh.borrow(),
            utils::FileSelectionSource::TabSwitch
        );
        crate::status_log::log_info("File list refreshed");
    });

    // Set up the terminal button handler to open a new terminal in the current directory
    let terminal_notebook_clone_for_terminal_button = imp.terminal_notebook.get();
    let current_dir_clone_for_terminal_button = current_dir.clone();
    let editor_paned_clone_for_terminal_button = editor_paned.clone();
    let editor_paned_clone_for_terminal_button2 = editor_paned.clone();
    terminal_button.connect_clicked(move |_| {
        // Show terminal if hidden
        if let Some(end_child) = editor_paned_clone_for_terminal_button.end_child() {
            if !end_child.is_visible() {
                end_child.set_visible(true);
                let max_pos = editor_paned_clone_for_terminal_button.allocation().height();
                editor_paned_clone_for_terminal_button.set_position((max_pos as f64 * 0.6) as i32);
                // Save terminal visible state
                let mut settings = crate::settings::get_settings_mut();
                settings.set_terminal_visible(true);
                let _ = settings.save();
            }
        }
        // Add a new terminal tab in the current directory (with auto-hide)
        ui::terminal::add_terminal_tab_with_toggle(&terminal_notebook_clone_for_terminal_button, Some(current_dir_clone_for_terminal_button.borrow().clone()), &editor_paned_clone_for_terminal_button2);
    });

    // Set up the add file button handler to create a new file (same as new button functionality)
    let editor_notebook_clone_for_add_file = editor_notebook.clone();
    let active_tab_path_clone_for_add_file = active_tab_path.clone();
    let file_path_manager_clone_for_add_file = file_path_manager.clone();
    let file_list_box_clone_for_add_file = file_list_box.clone();
    let current_dir_clone_for_add_file = current_dir.clone();
    let save_button_clone_for_add_file = save_button.clone();
    let save_as_button_clone_for_add_file = save_as_button.clone();
    let window_clone_for_add_file = window.clone();
    
    add_file_button.connect_clicked(move |_| {
        // Use the same logic as the new button - create a new empty tab
        let deps_for_new_tab_creation = handlers::NewTabDependencies {
            editor_notebook: editor_notebook_clone_for_add_file.clone(),
            active_tab_path: active_tab_path_clone_for_add_file.clone(),
            file_path_manager: file_path_manager_clone_for_add_file.clone(),
            window: window_clone_for_add_file.clone().upcast::<ApplicationWindow>(),
            file_list_box: file_list_box_clone_for_add_file.clone(),
            current_dir: current_dir_clone_for_add_file.clone(),
            save_button: save_button_clone_for_add_file.clone(),
            save_as_button: save_as_button_clone_for_add_file.clone(),
            _save_menu_button: None, // We don't need the menu button for this handler
        };
        
        // Create the new tab using the same system as the new button
        handlers::create_new_empty_tab(&deps_for_new_tab_creation);
    });

    // Handle file opening from command line arguments
    println!("Checking file_to_open: {:?}", file_to_open);
    if let Some(ref file_path) = file_to_open {
        println!("Processing file argument: {:?}", file_path);
        // Check if the file exists and is readable
        if file_path.exists() {
            if file_path.is_file() {
                // Close any empty untitled tabs before opening the file
                handlers::close_empty_untitled_tabs(&editor_notebook, &file_path_manager);
                
                let mut mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
                
                // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
                // but should be treated as TypeScript files (text/plain)
                if let Some(ext) = file_path.extension() {
                    if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                        // Override MIME type for TypeScript files
                        mime_type = mime_guess::mime::TEXT_PLAIN;
                    }
                }
                
                if utils::is_allowed_mime_type(&mime_type) {
                    // Try to read the file content
                    match std::fs::read_to_string(&file_path) {
                        Ok(content) => {
                            // Open the file in a new tab
                            handlers::open_or_focus_tab(
                                &editor_notebook,
                                &file_path,
                                &content,
                                &active_tab_path,
                                &file_path_manager,
                                &save_button,
                                &save_as_button,
                                &mime_type,
                                &window,
                                &file_list_box,
                                &current_dir,
                                Some(&save_menu_button),
                            );
                            
                            // Update current directory to the file's parent directory
                            if let Some(parent) = file_path.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                                utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                            }
                            
                            println!("Successfully opened file: {:?}", file_path);
                        }
                        Err(e) => {
                            eprintln!("Error reading file {:?}: {}", file_path, e);
                            // Could show an error dialog here in the future
                        }
                    }
                } else if mime_type.type_() == "image" {
                    // Handle image files
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for images
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                    
                    // Update current directory to the file's parent directory
                    if let Some(parent) = file_path.parent() {
                        *current_dir.borrow_mut() = parent.to_path_buf();
                        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                        utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                    }
                    
                    println!("Successfully opened image file: {:?}", file_path);
                } else {
                    // Handle unsupported file types by opening them with empty content
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for unsupported files
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                    
                    // Update current directory to the file's parent directory
                    if let Some(parent) = file_path.parent() {
                        *current_dir.borrow_mut() = parent.to_path_buf();
                        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow(), utils::FileSelectionSource::TabSwitch);
                        utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
                    }
                    
                    println!("Opened unsupported file type: {:?}", file_path);
                }
            } else {
                eprintln!("Error: {:?} is not a file", file_path);
            }
        } else {
            eprintln!("Error: File {:?} does not exist", file_path);
        }
    }

    // Set up the callback for opening files from diagnostics panel using a channel
    {
        let (sender, receiver) = std::sync::mpsc::channel::<(PathBuf, usize, usize)>();
        
        // Set up the receiver to handle file open requests on the main thread
        let notebook_clone = editor_notebook.clone();
        let file_path_manager_clone = file_path_manager.clone();
        let active_tab_path_clone = active_tab_path.clone();
        let save_button_clone = save_button.clone();
        let save_as_button_clone = save_as_button.clone();
        let window_clone = window.clone();
        let file_list_box_clone = file_list_box.clone();
        let current_dir_clone = current_dir.clone();
        
        glib::idle_add_local(move || {
            // Process all pending file open requests
            while let Ok((file_path, line, column)) = receiver.try_recv() {
                println!("Opening file from diagnostics: {} at {}:{}", file_path.display(), line, column);
                
                // Check if file is already open
                let mut page_to_focus = None;
                let num_pages = notebook_clone.n_pages();
                for i in 0..num_pages {
                    if let Some(path) = file_path_manager_clone.borrow().get(&i) {
                        if path == &file_path {
                            page_to_focus = Some(i);
                            break;
                        }
                    }
                }
                
                // If file is already open, focus it and jump to line
                if let Some(page_num) = page_to_focus {
                    notebook_clone.set_current_page(Some(page_num));
                    
                    // Get the source view and jump to line
                    if let Some(page) = notebook_clone.nth_page(Some(page_num)) {
                        if let Some(scrolled) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                            if let Some(source_view) = scrolled.child().and_then(|c| c.downcast::<sourceview5::View>().ok()) {
                                handlers::jump_to_line_and_column(&source_view, line, column);
                            }
                        }
                    }
                } else {
                    // File not open - open it first, then jump
                    if let Ok(content) = std::fs::read_to_string(&file_path) {
                        let mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
                        
                        handlers::open_or_focus_tab(
                            &notebook_clone,
                            &file_path,
                            &content,
                            &active_tab_path_clone,
                            &file_path_manager_clone,
                            &save_button_clone,
                            &save_as_button_clone,
                            &mime_type,
                            &window_clone,
                            &file_list_box_clone,
                            &current_dir_clone,
                            None,
                        );
                        
                        // After opening, jump to the line
                        let current_page = notebook_clone.current_page().unwrap_or(0);
                        if let Some(page) = notebook_clone.nth_page(Some(current_page)) {
                            if let Some(scrolled) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                                if let Some(source_view) = scrolled.child().and_then(|c| c.downcast::<sourceview5::View>().ok()) {
                                    // Use idle_add to ensure the view is fully loaded before jumping
                                    let source_view_clone = source_view.clone();
                                    glib::idle_add_local_once(move || {
                                        handlers::jump_to_line_and_column(&source_view_clone, line, column);
                                    });
                                }
                            }
                        }
                    }
                }
            }
            
            glib::ControlFlow::Continue
        });
        
        // Store the sender in the global callback
        *handlers::OPEN_FILE_CALLBACK.lock().unwrap() = Some(Box::new(move |file_path: PathBuf, line: usize, column: usize| {
            let _ = sender.send((file_path, line, column));
        }));
    }

    // Set up periodic cleanup of file cache to prevent memory bloat
    glib::timeout_add_seconds_local(300, || { // Every 5 minutes
        file_cache::cleanup_file_cache();
        glib::ControlFlow::Continue
    });
    
    // Show the main window to display the application
    window.show();
    
    // Restore previously opened files from settings (after window is shown)
    let saved_files = settings::get_settings().get_opened_files();
    if !saved_files.is_empty() {
        println!("Restoring {} previously opened file(s)", saved_files.len());
        for file_path in saved_files {
            if file_path.exists() && file_path.is_file() {
                let mut mime_type = mime_guess::from_path(&file_path).first_or_octet_stream();
                
                // Special case: .ts files are detected as video/mp2t (MPEG transport stream)
                // but should be treated as TypeScript files (text/plain)
                if let Some(ext) = file_path.extension() {
                    if ext.to_str() == Some("ts") || ext.to_str() == Some("tsx") {
                        // Override MIME type for TypeScript files
                        mime_type = mime_guess::mime::TEXT_PLAIN;
                    }
                }
                
                // Handle different file types appropriately
                if mime_type.type_() == "video" {
                    // For video files, open with empty content (don't try to read as text)
                    println!("Restoring video file: {}", file_path.display());
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for video
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                } else if mime_type.type_() == "audio" {
                    // For audio files, open with empty content
                    println!("Restoring audio file: {}", file_path.display());
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        "", // Empty content for audio
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                } else if mime_type.type_() == "image" {
                    // Check if it's an SVG file (which needs content for split view)
                    let is_svg = file_path.extension()
                        .and_then(|e| e.to_str())
                        .map(|s| s.to_lowercase() == "svg")
                        .unwrap_or(false);
                    
                    if is_svg {
                        // SVG files need content for the code editor
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            println!("Restoring SVG file: {}", file_path.display());
                            handlers::open_or_focus_tab(
                                &editor_notebook,
                                &file_path,
                                &content,
                                &active_tab_path,
                                &file_path_manager,
                                &save_button,
                                &save_as_button,
                                &mime_type,
                                &window,
                                &file_list_box,
                                &current_dir,
                                Some(&save_menu_button),
                            );
                        }
                    } else {
                        // For other image files, open with empty content
                        println!("Restoring image file: {}", file_path.display());
                        handlers::open_or_focus_tab(
                            &editor_notebook,
                            &file_path,
                            "", // Empty content for images
                            &active_tab_path,
                            &file_path_manager,
                            &save_button,
                            &save_as_button,
                            &mime_type,
                            &window,
                            &file_list_box,
                            &current_dir,
                            Some(&save_menu_button),
                        );
                    }
                } else if let Ok(content) = std::fs::read_to_string(&file_path) {
                    // For text files, read the content
                    println!("Restoring text file: {}", file_path.display());
                    handlers::open_or_focus_tab(
                        &editor_notebook,
                        &file_path,
                        &content,
                        &active_tab_path,
                        &file_path_manager,
                        &save_button,
                        &save_as_button,
                        &mime_type,
                        &window,
                        &file_list_box,
                        &current_dir,
                        Some(&save_menu_button),
                    );
                }
            }
        }
    }

    // Set up window close handler to save window size and pane positions
    let paned_for_close = paned.clone();
    let editor_paned_for_close = editor_paned.clone();
    let editor_notebook_for_close = editor_notebook.clone();
    let file_path_manager_for_close = file_path_manager.clone();
    window.connect_close_request(move |window| {
        // Shutdown rust-analyzer before closing
        crate::linter::ui::shutdown_rust_analyzer();
        
        // Get the current window size - use width() and height() for actual size
        let width = window.width();
        let height = window.height();
        
        // Get the current pane positions
        let file_panel_width = paned_for_close.position();
        let terminal_height = editor_paned_for_close.position();
        
        // Collect all opened file paths from the notebook
        let n_pages = editor_notebook_for_close.n_pages();
        let mut opened_files = Vec::new();
        for i in 0..n_pages {
            if editor_notebook_for_close.nth_page(Some(i)).is_some() {
                let page_num = i as u32;
                if let Some(path) = file_path_manager_for_close.borrow().get(&page_num) {
                    opened_files.push(path.clone());
                }
            }
        }
        
        // Save all state to settings
        let mut settings = settings::get_settings_mut();
        settings.set_window_size(width, height);
        settings.set_pane_dimensions(file_panel_width, terminal_height);
        settings.set_opened_files(&opened_files);
        
        // Save settings to disk
        if let Err(e) = settings.save() {
            eprintln!("Failed to save window dimensions: {}", e);
        } else {
            println!("Saved window size: {}x{}", width, height);
            println!("Saved file panel width: {}", file_panel_width);
            println!("Saved terminal height: {}", terminal_height);
            println!("Saved {} opened file(s)", opened_files.len());
        }
        
        // Allow the window to close
        glib::Propagation::Proceed
    });
    
    // Add destroy handler to ensure application exits cleanly
    let app_for_destroy = app.clone();
    window.connect_destroy(move |_| {
        println!("Window destroyed, quitting application");
        app_for_destroy.quit();
        
        // Force exit after a short delay if app doesn't quit normally
        std::thread::spawn(|| {
            std::thread::sleep(std::time::Duration::from_millis(500));
            println!("Force exiting application");
            std::process::exit(0);
        });
    });

    // Set up the settings button handler
    let window_clone_for_settings = window.clone();
    settings_button.connect_clicked(move |_| {
        // Create and show the settings dialog
        let dialog = ui::settings::create_settings_dialog(&window_clone_for_settings);
        
        // When the dialog is closed, update all buffer themes
        let window_ref = window_clone_for_settings.clone();
        dialog.connect_close(move |_| {
            // Apply the new theme settings to all buffers
            update_all_buffer_themes(&window_ref);
        });
        
        dialog.show();
    });
}

/// Sets up a GSettings monitor to detect Ubuntu/GNOME theme changes
/// This provides better integration with system theme switching on Ubuntu
fn setup_gsettings_monitor(window: &impl IsA<gtk4::Widget>, terminal_notebook: &gtk4::Notebook) {
    use gio::prelude::*;
    
    let window_clone = window.clone();
    let terminal_notebook_clone = terminal_notebook.clone();
    
    // Monitor the GNOME color-scheme setting which is the primary way Ubuntu switches themes
    match std::panic::catch_unwind(|| gio::Settings::new("org.gnome.desktop.interface")) {
        Ok(settings) => {
        let window_clone_2 = window_clone.clone();
        let terminal_notebook_clone_2 = terminal_notebook_clone.clone();
        
        // Monitor color-scheme changes (prefer-dark, prefer-light, default)
        // Only set up the monitor if the key exists (GNOME 42+/Ubuntu 22.04+)
        if let Some(schema) = settings.settings_schema() {
            if schema.has_key("color-scheme") {
                settings.connect_changed(Some("color-scheme"), move |_, _| {
                    println!("System color-scheme changed via GSettings");
                    // Small delay to ensure the change has propagated
                    let window_clone_inner = window_clone.clone();
                    let terminal_notebook_clone_inner = terminal_notebook_clone.clone();
                    glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                        update_all_buffer_themes(&window_clone_inner);
                        ui::terminal::update_all_terminal_themes(&terminal_notebook_clone_inner);
                    });
                });
            }
        }
        
        // Also monitor gtk-theme changes for additional coverage
        settings.connect_changed(Some("gtk-theme"), move |_, _| {
            println!("GTK theme changed via GSettings");
            let window_clone_inner = window_clone_2.clone();
            let terminal_notebook_clone_inner = terminal_notebook_clone_2.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(100), move || {
                update_all_buffer_themes(&window_clone_inner);
                ui::terminal::update_all_terminal_themes(&terminal_notebook_clone_inner);
            });
        });
        
        println!("GSettings monitor set up for org.gnome.desktop.interface");
        },
        Err(_) => {
            println!("Could not set up GSettings monitor - org.gnome.desktop.interface not available");
        }
    }
}

/// Updates the secondary status label with cursor position and file information
fn update_cursor_position_status(text_view: &sourceview5::View, status_label: &Label, filename: &str, file_path: Option<&std::path::Path>) {
    let buffer = text_view.buffer();
    let cursor_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&cursor_mark);
    
    let line = cursor_iter.line() + 1; // 1-based line numbers
    let column = cursor_iter.line_offset() + 1; // 1-based column numbers
    
    let status_text = if filename == "Untitled" {
        // For untitled documents, just show numbers: "X:Y"
        format!("{}:{}", line, column)
    } else {
        // For files, show: "filename | X:Y" (without path or labels)
        format!("{} | {}:{}", filename, line, column)
    };
    
    status_label.set_text(&status_text);
}
