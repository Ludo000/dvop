// UI module for the Basado Text Editor
// Contains all UI component creation and layout functions

pub mod terminal;
pub mod file_manager;
pub mod css;
pub mod settings;
pub mod global_search;

use gtk4::prelude::*;
use gtk4::{
    // Main application and window components
    Application, ApplicationWindow, 
    
    // Layout containers
    Box as GtkBox, Notebook,
    
    // Common UI elements
    Button, HeaderBar, Label, Picture, TextView, Image, TextBuffer,
    
    // Menu components for split button functionality
    MenuButton, PopoverMenu, gio,
    
    // Layout orientation for containers
    Orientation
};

// Import our modules
use crate::syntax;
use std::cell::RefCell;  // For interior mutability pattern
use std::rc::Rc;         // For shared ownership
use std::path::PathBuf;  // For file paths

// Home directory detection


/// Creates the main application window with default settings
pub fn create_window(app: &Application) -> ApplicationWindow {
    // Apply our custom CSS styling before building the window
    css::apply_custom_css();
    
    // Get saved window dimensions from settings
    let settings = crate::settings::get_settings();
    let window_width = settings.get_window_width();
    let window_height = settings.get_window_height();
    
    ApplicationWindow::builder()
        .application(app)      // Associate with the GTK application
        .default_width(window_width)    // Use saved window width
        .default_height(window_height)   // Use saved window height
        .title("Basado Text Editor")
        .build()
}

/// Creates the application header bar with action buttons
///
/// This function creates the application's header bar with buttons for core functionality.
/// Returns the header bar and the action buttons for connecting event handlers.
pub fn create_header() -> (HeaderBar, Button, Button, Button, MenuButton, Button, Button, Button, Button) {
    // Create the main header bar
    let header = HeaderBar::new();

    // Create a Settings button with icon only (no label)
    let settings_button = Button::new();
    let settings_button_icon = Image::from_icon_name("preferences-system-symbolic");
    settings_button.set_child(Some(&settings_button_icon));
    settings_button.set_tooltip_text(Some("Editor Settings"));
    header.pack_start(&settings_button);

    // Create the Open File button with icon and label
    let open_button = Button::new();
    let open_button_icon = Image::from_icon_name("document-open-symbolic");
    let open_button_label = Label::new(Some("Open"));
    let open_button_box = GtkBox::new(Orientation::Horizontal, 5);
    open_button_box.append(&open_button_icon);
    open_button_box.append(&open_button_label);
    open_button.set_child(Some(&open_button_box));
    open_button.set_tooltip_text(Some("Open a file"));
    header.pack_start(&open_button);

    // Create a split button for Save functionality that combines:
    // 1. A main Save button (left side)
    // 2. A dropdown menu button (right side) with additional options
    
    // Create a container box for the split button with "linked" style
    // This makes both parts of the split button appear as a single unit
    let save_split_box = GtkBox::new(Orientation::Horizontal, 0);
    save_split_box.add_css_class("linked"); // Makes the buttons appear connected
    
    // Create the main Save button (left side) with icon and label
    let save_main_button = Button::new();
    let save_button_icon = Image::from_icon_name("document-save-symbolic");
    let save_button_label = Label::new(Some("Save"));
    let save_main_button_box = GtkBox::new(Orientation::Horizontal, 5);
    save_main_button_box.append(&save_button_icon);
    save_main_button_box.append(&save_button_label);
    save_main_button.set_child(Some(&save_main_button_box));
    save_main_button.set_tooltip_text(Some("Save the current file"));
    
    // Create the dropdown button (right side) with a downward arrow icon
    let save_menu_button = MenuButton::builder()
        .icon_name("pan-down-symbolic")
        .tooltip_text("Additional save options")
        .build();
    
    // Set minimum width for the dropdown button to make it compact
    save_menu_button.set_size_request(20, -1);
    
    // Create the menu that will appear when clicking the dropdown
    let menu = gio::Menu::new();
    let save_as_item = gio::MenuItem::new(Some("Save As..."), Some("win.save-as"));
    menu.append_item(&save_as_item);
    
    // Create a popover menu from the menu model and attach it to the button
    let popover = PopoverMenu::from_model(Some(&menu));
    save_menu_button.set_popover(Some(&popover));
    
    // Assemble the split button by adding both parts to the container
    save_split_box.append(&save_main_button);
    save_split_box.append(&save_menu_button);
    
    // Add the complete split button to the right side of the header
    header.pack_end(&save_split_box);

    // Create a Global Search button with a looking glass icon (no label)
    // Place it before the Save button (pack_end adds from right to left, so this comes before save)
    let global_search_button = Button::new();
    let global_search_icon = Image::from_icon_name("system-search-symbolic");
    global_search_button.set_child(Some(&global_search_icon));
    global_search_button.set_tooltip_text(Some("Global Search"));
    header.pack_end(&global_search_button);

    // Create a hidden Save As button that will be triggered programmatically from the menu
    // This approach allows reusing the same handler logic for both menu and direct button clicks
    let save_as_button = Button::new();
    let save_as_button_icon = Image::from_icon_name("document-save-as-symbolic");
    let save_as_button_label = Label::new(Some("Save As"));
    let save_as_button_box = GtkBox::new(Orientation::Horizontal, 5);
    save_as_button_box.append(&save_as_button_icon);
    save_as_button_box.append(&save_as_button_label);
    save_as_button.set_child(Some(&save_as_button_box));
    save_as_button.set_tooltip_text(Some("Save the current file with a new name"));
    save_as_button.set_visible(false); // Hidden since it's only triggered programmatically

    // Create a hidden regular save button for programmatic access
    // This avoids circular reference issues when connecting signals
    let save_button = Button::new();
    save_button.set_visible(false);

    // Create a hidden new button for backward compatibility with existing handler code
    let new_button = Button::new();
    new_button.set_visible(false);

    // Return the header and all action buttons (new_button is now hidden for compatibility)
    (header, new_button, open_button, save_main_button, save_menu_button, save_as_button, save_button, settings_button, global_search_button)
}

/// Creates the main text editor view components
/// 
/// Returns a tuple containing:
/// - ScrolledWindow: Container for the text view with scrolling capabilities
/// - TextView: The main text editing widget (actually a SourceView for syntax highlighting)
/// - TextBuffer: The buffer holding the text content (actually a SourceBuffer)
/// - Rc<RefCell<Option<PathBuf>>>: Optional file path for the current document
/// - Label: Error message display label
/// - Picture: Widget for displaying images when opening image files
/// - Rc<RefCell<PathBuf>>: Current working directory
/// - Notebook: Main tabbed container for managing multiple documents
/// - GtkBox: Custom tab widget for the initial tab
/// - Label: Text label for the initial tab
/// - Button: Close button for the initial tab
/// - Button: Add new file tab button
pub fn create_text_view() -> (
    gtk4::ScrolledWindow,
    gtk4::TextView,
    gtk4::TextBuffer,
    Rc<RefCell<Option<PathBuf>>>, // file_path
    Label,                        // error_label
    Picture,                      // picture for images
    Rc<RefCell<PathBuf>>,         // current_dir
    Notebook,                     // editor_notebook
    GtkBox,                       // tab_widget for the initial tab
    Label,                        // tab_label for the initial tab
    Button,                       // tab_close_button for the initial tab
    Button                        // add_file_button for creating new tabs
) {
    // Create the tabbed notebook container with scrollable tabs
    let editor_notebook = Notebook::new();
    editor_notebook.set_scrollable(true);
    editor_notebook.set_show_border(true);
    
    // Add CSS class for better tab styling
    editor_notebook.add_css_class("basado-notebook");

    // Create an "Add File" button similar to the terminal's add button
    let add_file_button = Button::from_icon_name("list-add-symbolic");
    add_file_button.set_tooltip_text(Some("Create a new file"));
    add_file_button.set_margin_end(8); // Add right padding

    // Create the first "Untitled" tab
    let (tab_widget, tab_label, tab_close_button) = create_tab_widget("Untitled");
    
    // Add middle mouse click support for the tab
    setup_tab_middle_click(&tab_widget, &tab_close_button);
    
    // Create a source view with syntax highlighting instead of a standard text view
    let (source_view, source_buffer) = syntax::create_source_view();
    
    // Clone source_view before upcast to avoid ownership move
    let text_view = source_view.clone().upcast::<TextView>();
    let buffer = source_buffer.upcast::<TextBuffer>();
    
    // Set up interaction tracking for the initial text editor
    crate::handlers::setup_text_editor_interaction_tracking(&text_view);

    // Place the source view in a scrolled window
    let scrolled_window = syntax::create_source_view_scrolled(&source_view);

    // Add the scrolled window as a page in the notebook with our custom tab widget
    editor_notebook.append_page(&scrolled_window, Some(&tab_widget));
    editor_notebook.set_tab_label(&scrolled_window, Some(&tab_widget));

    // Initialize shared state objects
    let file_path = Rc::new(RefCell::new(None)); // No file associated with initial tab
    let error_label = Label::new(None);          // Empty error label
    let picture = Picture::new();                // Empty picture widget for showing images
    
    // Set current directory to the last used folder from settings, or fallback to home directory
    let last_folder = crate::settings::get_settings().get_last_folder();
    let current_dir = Rc::new(RefCell::new(
        if last_folder.exists() && last_folder.is_dir() {
            last_folder
        } else {
            home::home_dir().unwrap_or_else(|| PathBuf::from("/"))
        }
    ));

    // Return all components needed by the application
    (
        scrolled_window,   // Container for the text view
        text_view,         // Main editing widget
        buffer,            // Text content buffer
        file_path,         // Optional file path for the current document
        error_label,       // For displaying error messages
        picture,           // For displaying images
        current_dir,       // Current working directory
        editor_notebook,   // Main tabbed container for multiple documents
        tab_widget,        // Container for tab components
        tab_label,         // Label showing filename in tab
        tab_close_button,  // Button to close the tab
        add_file_button    // Button to add new file tabs
    )
}

/// Creates the main application layout using paned containers
///
/// This function arranges the major UI components into a nested paned layout:
/// - Horizontal split between file manager (left) and editor+terminal (right)
/// - The right side has a vertical split between editor (top) and terminal (bottom)
/// 
/// Returns a tuple of:
/// - gtk4::Paned: The main horizontal paned container
/// - gtk4::Paned: The vertical paned container for editor+terminal
pub fn create_paned(
    file_manager_panel: &GtkBox,     // File browser sidebar
    editor_notebook_box: &GtkBox,    // Editor notebook container with add button
    terminal_box: &impl IsA<gtk4::Widget>,  // Terminal container (either ScrolledWindow or GtkBox)
) -> (gtk4::Paned, gtk4::Paned) {
    // Create the main horizontal split pane
    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);  // Use a wider drag handle for easier resizing
    paned.set_vexpand(true);      // Allow the paned area to expand vertically
    
    // Create the vertical split pane for the right side
    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true);
    
    // Place editor notebook box at the top of the vertical split
    editor_paned.set_start_child(Some(editor_notebook_box));
    
    // Place terminal at the bottom of the vertical split
    editor_paned.set_end_child(Some(terminal_box));
    
    // Make the editor paned expand vertically
    editor_paned.set_vexpand(true);
    
    // Place file manager on the left side of the horizontal split
    paned.set_start_child(Some(file_manager_panel));
    
    // Place the editor+terminal vertical split on the right side
    paned.set_end_child(Some(&editor_paned));
    
    // Get saved pane positions from settings
    let settings = crate::settings::get_settings();
    let file_panel_width = settings.get_file_panel_width();
    let terminal_height = settings.get_terminal_height();
    
    // Set initial split positions
    paned.set_position(file_panel_width);        // Width of file manager sidebar
    editor_paned.set_position(terminal_height); // Height of editor area
    
    (paned, editor_paned)
}

/// Creates a custom tab widget with a label and close button
/// 
/// Each tab in the notebook uses this custom widget instead of just text,
/// allowing for a close button directly in the tab.
///
/// Returns a tuple of:
/// - GtkBox: Container for the tab components
/// - Label: Text label displaying the filename
/// - Button: Close button to close the tab
pub fn create_tab_widget(tab_title: &str) -> (GtkBox, Label, Button) {
    // Create horizontal container for tab contents with comfortable spacing
    let tab_box = GtkBox::new(Orientation::Horizontal, 4);
    
    // Add CSS class for custom tab styling
    tab_box.add_css_class("tab-box");
    
    // Set comfortable margins
    tab_box.set_margin_top(2);
    tab_box.set_margin_bottom(2);
    tab_box.set_margin_start(4); 
    tab_box.set_margin_end(2);
    
    // Set a comfortable minimum width for the tab box
    tab_box.set_size_request(120, -1);
    
    // Create label with the provided title
    let label = Label::new(Some(tab_title));
    label.set_margin_start(3);
    label.set_width_chars(10); // Increased width for longer tabs
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End); // Add ellipsis if text overflows
    label.add_css_class("tab-label"); // Add custom CSS class for styling
    
    // Create close button with a standard X icon
    let close_button = Button::from_icon_name("window-close-symbolic");
    
    // Use a comfortably sized button
    close_button.add_css_class("circular"); // Make button more rounded
    close_button.set_valign(gtk4::Align::Center);
    
    // Set comfortable button margins
    close_button.set_margin_start(2);
    close_button.set_margin_end(1);
    
    // Make the button a comfortable size
    close_button.set_size_request(20, 20);
    
    // Assemble tab components
    tab_box.append(&label);
    tab_box.append(&close_button);
    
    (tab_box, label, close_button)
}

/// Adds middle mouse click support to a tab widget for closing tabs
///
/// This function sets up a gesture click controller that listens for
/// middle mouse button clicks on the tab and triggers the close action.
pub fn setup_tab_middle_click(tab_box: &GtkBox, close_button: &Button) {
    use gtk4::prelude::*;
    
    // Create a gesture click controller that responds to middle mouse button clicks
    let middle_click_gesture = gtk4::GestureClick::new();
    middle_click_gesture.set_button(2); // Middle mouse button
    
    // Clone the close button for the closure
    let close_button_clone = close_button.clone();
    
    // Connect the pressed signal to simulate a close button click
    middle_click_gesture.connect_pressed(move |_, _n_press, _x, _y| {
        // Log the middle mouse click for debugging
        crate::status_log::log_info("Middle mouse click detected on tab - closing");
        
        // Emit the clicked signal on the close button to trigger the existing close logic
        close_button_clone.emit_clicked();
    });
    
    // Add the gesture controller to the tab box
    tab_box.add_controller(middle_click_gesture);
}

/// Creates a container box for the editor notebook with the add button and search bar
/// 
/// The editor notebook is placed in a box with the search bar above it
/// The add button is placed as an action button in the notebook's tab bar area
pub fn create_editor_notebook_box(editor_notebook: &Notebook, add_file_button: &Button) -> GtkBox {
    let editor_box = GtkBox::new(Orientation::Vertical, 0);
    
    // Add the search bar at the top
    let search_bar = crate::search::get_search_bar();
    editor_box.append(search_bar);
    
    // Add the add button to the tab bar via the action widget feature
    // This places the button in the same row as the tabs
    editor_notebook.set_action_widget(add_file_button, gtk4::PackType::End);
    
    // Set the editor notebook to expand vertically
    editor_notebook.set_vexpand(true);
    
    // Pack the notebook into the container box
    editor_box.append(editor_notebook);
    
    // Make the entire container expand vertically
    editor_box.set_vexpand(true);
    
    editor_box
}

/// Creates a status bar with operation status display
///
/// Returns a tuple of:
/// - GtkBox: Container for the status bar
/// - Label: Main status text label
/// - Label: Secondary status information (current file, line/column, etc.)
pub fn create_status_bar() -> (GtkBox, Label, Label) {
    // Create horizontal container for status bar
    let status_bar = GtkBox::new(Orientation::Horizontal, 8);
    status_bar.add_css_class("status-bar");
    
    // Set padding and styling
    status_bar.set_margin_start(12);
    status_bar.set_margin_end(12);
    status_bar.set_margin_top(4);
    status_bar.set_margin_bottom(4);
    
    // Create main status label for current operation wrapped in a button for clickability
    let status_button = gtk4::Button::new();
    status_button.set_has_frame(false);
    status_button.add_css_class("status-button");
    
    let status_label = Label::new(Some("Ready"));
    status_label.set_halign(gtk4::Align::Start);
    status_label.set_hexpand(true);
    status_label.add_css_class("status-text");
    
    status_button.set_child(Some(&status_label));
    
    // Create secondary status label for file info (right-aligned)
    let secondary_label = Label::new(Some(""));
    secondary_label.set_halign(gtk4::Align::End);
    secondary_label.add_css_class("status-secondary");
    
    // Add widgets to status bar
    status_bar.append(&status_button);
    status_bar.append(&secondary_label);
    
    (status_bar, status_label, secondary_label)
}

/// Updates the visibility of the volume control based on active tab content
pub fn update_volume_control_visibility_for_tab(volume_control_box: &GtkBox, active_tab_path: &Option<std::path::PathBuf>) {
    let is_music_tab = active_tab_path
        .as_ref()
        .map(|path| crate::audio::is_music_file(path))
        .unwrap_or(false);
    volume_control_box.set_visible(is_music_tab);
}

/// Creates and shows a log history popup window
/// 
/// # Arguments
/// * `parent_window` - The parent window to center the popup on
pub fn show_log_history_popup(parent_window: &gtk4::ApplicationWindow) {
    use crate::status_log;
    
    // Create the dialog window as an ApplicationWindow for full window controls
    let dialog = gtk4::ApplicationWindow::new(parent_window.application().as_ref().unwrap());
    dialog.set_title(Some("Log History"));
    // Don't set as transient or modal to allow minimize/maximize
    dialog.set_default_size(600, 500);
    dialog.set_resizable(true);
    
    // Enable window controls and proper window manager hints
    dialog.set_deletable(true);
    dialog.set_decorated(true);
    
    // Set minimum size to prevent it from getting too small
    dialog.set_size_request(400, 300);
    
    // Create a header bar with the clear button
    let header_bar = gtk4::HeaderBar::new();
    header_bar.set_title_widget(Some(&Label::new(Some("Log History"))));
    
    // Show window control buttons explicitly
    header_bar.set_show_title_buttons(true);
    
    // Add clear button to header bar on the left side
    let clear_button = gtk4::Button::with_label("Clear History");
    clear_button.add_css_class("destructive-action");
    header_bar.pack_start(&clear_button);
    
    dialog.set_titlebar(Some(&header_bar));
    
    // Create the main container
    let main_box = GtkBox::new(Orientation::Vertical, 8);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);
    
    // Create scrollable area for log messages (no header box needed now)
    let scrolled_window = gtk4::ScrolledWindow::new();
    scrolled_window.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scrolled_window.set_vexpand(true);
    scrolled_window.set_hexpand(true);
    
    // Create list box for log entries
    let log_list = gtk4::ListBox::new();
    log_list.add_css_class("log-history-list");
    log_list.set_selection_mode(gtk4::SelectionMode::None);
    
    // Populate the list with log history
    let history = status_log::get_log_history();
    
    if history.is_empty() {
        let empty_label = Label::new(Some("No log messages yet."));
        empty_label.add_css_class("dim-label");
        empty_label.set_margin_top(20);
        empty_label.set_margin_bottom(20);
        log_list.append(&empty_label);
    } else {
        // Add messages in reverse order (newest first)
        for log_message in history.iter().rev() {
            let row = gtk4::ListBoxRow::new();
            row.set_activatable(false);
            
            let message_box = GtkBox::new(Orientation::Vertical, 4);
            message_box.set_margin_start(8);
            message_box.set_margin_end(8);
            message_box.set_margin_top(6);
            message_box.set_margin_bottom(6);
            
            // Format timestamp to show both real date/time and relative time
            let timestamp_str = {
                use chrono::{Datelike, Local, TimeZone};
                
                let relative_time = log_message.timestamp
                    .elapsed()
                    .map(|duration| {
                        let secs = duration.as_secs();
                        if secs < 60 {
                            format!("{}s ago", secs)
                        } else if secs < 3600 {
                            format!("{}m ago", secs / 60)
                        } else if secs < 86400 {
                            format!("{}h ago", secs / 3600)
                        } else {
                            format!("{}d ago", secs / 86400)
                        }
                    })
                    .unwrap_or_else(|_| "just now".to_string());
                
                match log_message.timestamp.duration_since(std::time::UNIX_EPOCH) {
                    Ok(duration) => {
                        let secs = duration.as_secs() as i64;
                        let nanos = duration.subsec_nanos();
                        
                        if let Some(datetime) = Local.timestamp_opt(secs, nanos).single() {
                            let now = Local::now();
                            let date_today = now.date_naive();
                            let date_message = datetime.date_naive();
                            
                            let formatted_time = if date_message == date_today {
                                // Today - show time only
                                datetime.format("%H:%M:%S").to_string()
                            } else if date_today.signed_duration_since(date_message).num_days() == 1 {
                                // Yesterday
                                datetime.format("Yesterday %H:%M").to_string()
                            } else if date_today.signed_duration_since(date_message).num_days() < 7 {
                                // This week - show day name and time
                                datetime.format("%A %H:%M").to_string()
                            } else if date_message.year() == date_today.year() {
                                // This year - show month, day and time
                                datetime.format("%b %d %H:%M").to_string()
                            } else {
                                // Different year - show full date and time
                                datetime.format("%Y-%m-%d %H:%M").to_string()
                            };
                            
                            // Combine both formats: "Aug 12 14:30 (2m ago)"
                            format!("{} ({})", formatted_time, relative_time)
                        } else {
                            format!("Invalid time ({})", relative_time)
                        }
                    }
                    Err(_) => format!("Unknown time ({})", relative_time)
                }
            };
            
            // Create level indicator and message
            let top_line = GtkBox::new(Orientation::Horizontal, 8);
            
            let level_label = Label::new(Some(&format!("[{}]", match log_message.level {
                status_log::LogLevel::Info => "INFO",
                status_log::LogLevel::Warning => "WARN", 
                status_log::LogLevel::Error => "ERROR",
                status_log::LogLevel::Success => "OK",
            })));
            
            let level_css_class = match log_message.level {
                status_log::LogLevel::Info => "log-level-info",
                status_log::LogLevel::Warning => "log-level-warning",
                status_log::LogLevel::Error => "log-level-error", 
                status_log::LogLevel::Success => "log-level-success",
            };
            level_label.add_css_class(level_css_class);
            level_label.add_css_class("log-level-badge");
            
            let time_label = Label::new(Some(&timestamp_str));
            time_label.add_css_class("log-timestamp");
            time_label.set_halign(gtk4::Align::End);
            time_label.set_hexpand(true);
            
            top_line.append(&level_label);
            top_line.append(&time_label);
            
            let message_label = Label::new(Some(&log_message.message));
            message_label.set_halign(gtk4::Align::Start);
            message_label.set_wrap(true);
            message_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
            message_label.add_css_class("log-message");
            
            message_box.append(&top_line);
            message_box.append(&message_label);
            
            row.set_child(Some(&message_box));
            log_list.append(&row);
        }
    }
    
    scrolled_window.set_child(Some(&log_list));
    
    // Assemble the dialog - just add the scrolled window since we have a header bar now
    main_box.append(&scrolled_window);
    
    dialog.set_child(Some(&main_box));
    
    // Connect clear button event
    let log_list_clone = log_list.clone();
    clear_button.connect_clicked(move |_| {
        // Clear the log history
        status_log::clear_log_history();
        
        // Clear all existing entries from the list
        while let Some(child) = log_list_clone.first_child() {
            log_list_clone.remove(&child);
        }
        
        // Add empty state message
        let empty_label = Label::new(Some("No log messages yet."));
        empty_label.add_css_class("dim-label");
        empty_label.set_margin_top(20);
        empty_label.set_margin_bottom(20);
        log_list_clone.append(&empty_label);
    });
    
    // Handle Escape key to close
    let key_controller = gtk4::EventControllerKey::new();
    let dialog_clone_for_key = dialog.clone();
    key_controller.connect_key_pressed(move |_, key, _, _| {
        if key == gdk4::Key::Escape {
            dialog_clone_for_key.close();
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });
    dialog.add_controller(key_controller);
    
    // Show the dialog
    dialog.present();
}


