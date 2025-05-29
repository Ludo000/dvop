// UI module for the Basado Text Editor
// Contains all UI component creation and layout functions

use gtk4::prelude::*;
use gtk4::{
    // Main application and window components
    Application, ApplicationWindow, 
    
    // Layout containers
    Box as GtkBox, Notebook, ScrolledWindow,
    
    // Common UI elements
    Button, HeaderBar, Label, ListBox, Picture, TextView, Image, TextBuffer,
    
    // Menu components for split button functionality
    MenuButton, PopoverMenu, gio,
    
    // Layout orientation for containers
    Orientation,
    
    // GDK graphics components
    gdk,
    
    // Dialog components
    Dialog
};

// Import our modules
use crate::syntax;
use crate::settings;
use sourceview5::StyleSchemeManager;
// Standard library imports
use std::cell::RefCell;  // For interior mutability pattern
use std::rc::Rc;         // For shared ownership
use std::env;            // For environment variables
use std::path::PathBuf;  // For file paths

// Terminal emulator support
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;
use vte4::TerminalExt;

// Home directory detection
use home;

/// Creates the main application window with default settings
pub fn create_window(app: &Application) -> ApplicationWindow {
    // Apply our custom CSS styling before building the window
    apply_custom_css();
    
    ApplicationWindow::builder()
        .application(app)      // Associate with the GTK application
        .default_width(800)    // Initial window width
        .default_height(600)   // Initial window height
        .title("Basado Text Editor")
        .build()
}

/// Creates the application header bar with action buttons
///
/// This function creates the application's header bar with buttons for core functionality.
/// Returns the header bar and the action buttons for connecting event handlers.
pub fn create_header() -> (HeaderBar, Button, Button, Button, MenuButton, Button, Button, Button) {
    // Create the main header bar
    let header = HeaderBar::new();

    // Create a Settings button with icon only (no label)
    let settings_button = Button::new();
    let settings_button_icon = Image::from_icon_name("preferences-system-symbolic");
    settings_button.set_child(Some(&settings_button_icon));
    settings_button.set_tooltip_text(Some("Editor Settings"));
    header.pack_start(&settings_button);

    // Create the New File button with icon and label
    let new_button = Button::new();
    let new_button_icon = Image::from_icon_name("document-new-symbolic");
    let new_button_label = Label::new(Some("New"));
    let new_button_box = GtkBox::new(Orientation::Horizontal, 5);
    new_button_box.append(&new_button_icon);
    new_button_box.append(&new_button_label);
    new_button.set_child(Some(&new_button_box));
    new_button.set_tooltip_text(Some("Create a new file"));
    header.pack_start(&new_button);

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

    // Return the header and all action buttons
    (header, new_button, open_button, save_main_button, save_menu_button, save_as_button, save_button, settings_button)
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
    Button                        // tab_close_button for the initial tab
) {
    // Create the tabbed notebook container with scrollable tabs
    let editor_notebook = Notebook::new();
    editor_notebook.set_scrollable(true);
    editor_notebook.set_show_border(true);
    
    // Add CSS class for better tab styling
    editor_notebook.add_css_class("basado-notebook");

    // Create the first "Untitled" tab
    let (tab_widget, tab_label, tab_close_button) = create_tab_widget("Untitled");
    
    // Create a source view with syntax highlighting instead of a standard text view
    let (source_view, source_buffer) = syntax::create_source_view();
    
    // Clone source_view before upcast to avoid ownership move
    let text_view = source_view.clone().upcast::<TextView>();
    let buffer = source_buffer.upcast::<TextBuffer>();

    // Place the source view in a scrolled window
    let scrolled_window = syntax::create_source_view_scrolled(&source_view);

    // Add the scrolled window as a page in the notebook with our custom tab widget
    editor_notebook.append_page(&scrolled_window, Some(&tab_widget));
    editor_notebook.set_tab_label(&scrolled_window, Some(&tab_widget));

    // Initialize shared state objects
    let file_path = Rc::new(RefCell::new(None)); // No file associated with initial tab
    let error_label = Label::new(None);          // Empty error label
    let picture = Picture::new();                // Empty picture widget for showing images
    
    // Set current directory to user's home directory or fallback to root
    let current_dir = Rc::new(RefCell::new(home::home_dir().unwrap_or_else(|| PathBuf::from("/"))));

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
        tab_close_button   // Button to close the tab
    )
}

/// Creates and initializes a terminal emulator
/// 
/// This function creates a VTE terminal widget and spawns the user's default shell in it
/// 
/// Parameters:
/// - working_dir: Optional working directory to start the terminal in. If None, uses the user's home directory
pub fn create_terminal(working_dir: Option<PathBuf>) -> VteTerminal {
    let terminal = VteTerminal::new();
    
    // Set terminal colors to match the editor's theme
    setup_terminal_theme(&terminal);
    
    // Get the user's default shell from environment variables
    if let Some(shell) = env::var("SHELL").ok() {
        // Use the provided working directory or fall back to user's home directory
        let dir = match working_dir {
            Some(dir) => dir,
            None => home::home_dir().expect("Could not find home directory")
        };
        
        if let Some(dir_str) = dir.to_str() {
            // Spawn the shell asynchronously in the terminal
            terminal.spawn_async(
                vte4::PtyFlags::DEFAULT,          // Default pseudo-terminal flags
                Some(dir_str),                    // Working directory
                &[&shell],                        // Command (user's shell)
                &[],                              // Environment variables (none added)
                glib::SpawnFlags::DEFAULT,        // Default spawn flags
                || {},                            // Setup function (none)
                -1,                               // Default timeout
                None::<&Cancellable>,             // No cancellation
                move |res| {
                    // Handle spawn errors
                    if let Err(err) = res {
                        eprintln!("Failed to spawn shell: {}", err);
                    }
                },
            );
        } else {
            eprintln!("Failed to convert directory path to string");
        }
    }
    terminal
}

/// Creates a scrollable container for the terminal
/// 
/// The terminal is placed in a scrolled window with appropriate sizing constraints
pub fn create_terminal_box(terminal: &VteTerminal) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(terminal)           // Set the terminal as the child widget
        .vexpand(true)             // Expand vertically to fill all available space
        .hexpand(true)             // Expand horizontally to fill available width
        .min_content_height(150)   // Set minimum height for usability
        .build()
}

/// Creates the file manager panel components
/// 
/// Returns a tuple containing:
/// - ListBox: The list of files and directories
/// - ScrolledWindow: Container for the file list with scrolling
/// - GtkBox: Navigation toolbar with buttons
/// - Button: Up button for navigating to parent directory
/// - Button: Refresh button for updating the file list
/// - Button: Open in Terminal button for opening the current directory in a terminal
pub fn create_file_manager_panel() -> (ListBox, ScrolledWindow, GtkBox, Button, Button, Button) {
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

    // Create a horizontal box for navigation buttons
    let nav_box = GtkBox::new(Orientation::Horizontal, 0); 
    nav_box.set_margin_top(5); // Add spacing at the top
    
    // Create the "Up" button with a standard icon
    let up_button_icon = Image::from_icon_name("go-up-symbolic");
    let up_button = Button::new();
    up_button.set_child(Some(&up_button_icon));
    up_button.set_margin_start(5); // Add left margin
    
    // Create the "Open in Terminal" button with a terminal icon
    let terminal_button_icon = Image::from_icon_name("utilities-terminal-symbolic");
    let terminal_button = Button::new();
    terminal_button.set_child(Some(&terminal_button_icon));
    terminal_button.set_tooltip_text(Some("Open current folder in a new terminal"));
    terminal_button.set_margin_start(5); // Add left margin for spacing from the Up button
    
    // Create the "Refresh" button with a standard icon
    let refresh_button_icon = Image::from_icon_name("view-refresh-symbolic");
    let refresh_button = Button::new();
    refresh_button.set_child(Some(&refresh_button_icon));
    refresh_button.set_margin_end(5); // Add right margin
    
    // Create an expanding spacer to push buttons to opposite sides
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true); // Make the spacer expand to push buttons apart
    
    // Assemble the navigation toolbar
    nav_box.append(&up_button);       // Up button on left
    nav_box.append(&terminal_button); // Terminal button next to up button
    nav_box.append(&spacer);          // Expanding space in middle
    nav_box.append(&refresh_button);  // Refresh button on right
    
    // Return the components for further assembly and event handling
    (file_list_box, scrolled_window, nav_box, up_button, refresh_button, terminal_button)
}

/// Assembles the file manager panel from its components
/// 
/// Takes the navigation buttons and file list and combines them into a single container
pub fn create_file_manager_panel_container(nav_box: GtkBox, file_list_scrolled_window: ScrolledWindow) -> GtkBox {
    // Create a vertical box to hold all file manager components
    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    
    // Add the navigation buttons at the top
    file_manager_panel.append(&nav_box);
    
    // Add the scrollable file list below
    file_manager_panel.append(&file_list_scrolled_window);
    
    // Make the panel expand vertically to use available space
    file_manager_panel.set_vexpand(true);
    
    file_manager_panel
}

/// Creates the main application layout using paned containers
///
/// This function arranges the major UI components into a nested paned layout:
/// - Horizontal split between file manager (left) and editor+terminal (right)
/// - The right side has a vertical split between editor (top) and terminal (bottom)
/// - A status bar is placed at the bottom of the entire application
pub fn create_paned(
    file_manager_panel: &GtkBox,     // File browser sidebar
    editor_notebook: &Notebook,      // Editor tabs container
    terminal_box: &impl IsA<gtk4::Widget>,  // Terminal container (either ScrolledWindow or GtkBox)
    status_bar: &GtkBox              // Status bar with path label
) -> GtkBox {
    // Create the main horizontal split pane
    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);  // Use a wider drag handle for easier resizing
    paned.set_vexpand(true);      // Allow the paned area to expand vertically
    
    // Create the vertical split pane for the right side
    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true);
    
    // Place editor notebook at the top of the vertical split
    editor_paned.set_start_child(Some(editor_notebook));
    
    // Place terminal at the bottom of the vertical split
    editor_paned.set_end_child(Some(terminal_box));
    
    // Make the editor paned expand vertically
    editor_paned.set_vexpand(true);
    
    // Place file manager on the left side of the horizontal split
    paned.set_start_child(Some(file_manager_panel));
    
    // Place the editor+terminal vertical split on the right side
    paned.set_end_child(Some(&editor_paned));
    
    // Set initial split positions
    paned.set_position(200);        // Width of file manager sidebar
    editor_paned.set_position(400); // Height of editor area
    
    // Create a vertical box to hold the paned layout and status bar
    let main_container = GtkBox::new(Orientation::Vertical, 0);
    
    // Add the paned container as the main content
    main_container.append(&paned);
    
    // Add a separator before the status bar
    let separator = gtk4::Separator::new(Orientation::Horizontal);
    main_container.append(&separator);
    
    // Add the status bar at the bottom
    main_container.append(status_bar);
    
    main_container
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

/// Creates a tabbed terminal interface with Add and Close buttons
/// 
/// This function creates a notebook container with an initial terminal tab,
/// plus an "Add" button to create new terminal tabs.
/// Each terminal tab has its own close button.
pub fn create_terminal_notebook() -> (Notebook, Button) {
    // Create a notebook for terminal tabs
    let terminal_notebook = Notebook::new();
    terminal_notebook.set_scrollable(true);
    terminal_notebook.set_show_border(true);
    
    // Add some CSS classes for better tab styling
    terminal_notebook.add_css_class("basado-notebook");
    
    // Create an "Add Terminal" button
    let add_terminal_button = Button::from_icon_name("list-add-symbolic");
    add_terminal_button.set_tooltip_text(Some("Add a new terminal tab"));
    add_terminal_button.set_margin_end(8); // Add right padding
    
    // Create the first terminal tab
    add_terminal_tab(&terminal_notebook, None);
    
    // Connect the Add Terminal button click handler
    let terminal_notebook_clone = terminal_notebook.clone();
    add_terminal_button.connect_clicked(move |_| {
        add_terminal_tab(&terminal_notebook_clone, None);
    });
    
    (terminal_notebook, add_terminal_button)
}

/// Adds a new terminal tab to the terminal notebook
/// 
/// Creates a new terminal instance, places it in a tab, and adds it to the notebook
/// 
/// Parameters:
/// - terminal_notebook: The notebook to add the terminal tab to
/// - working_dir: Optional working directory to start the terminal in
///
/// Returns the page number of the new tab
pub fn add_terminal_tab(terminal_notebook: &Notebook, working_dir: Option<PathBuf>) -> u32 {
    // Use the last folder name from the path for the tab title, or "Home" for default tabs
    let tab_title = if let Some(dir_path) = &working_dir {
        // Get the last component of the path (the folder name)
        dir_path.file_name()
                .and_then(|name| name.to_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| "Home".to_string())
    } else {
        "home".to_string()
    };
    
    // Create a new terminal with a clone of the working directory
    let terminal = create_terminal(working_dir.clone());
    let terminal_box = create_terminal_box(&terminal);
    
    // Create a tab widget with the folder name or default title
    let (tab_widget, _tab_label, tab_close_button) = create_tab_widget(&tab_title);
    
    // Append the terminal to the notebook
    let page_num = terminal_notebook.append_page(&terminal_box, Some(&tab_widget));
    terminal_notebook.set_current_page(Some(page_num));
    
    // Connect the close button
    let notebook_clone = terminal_notebook.clone();
    let terminal_box_clone = terminal_box.clone();
    tab_close_button.connect_clicked(move |_| {
        // Find the current page number for this tab's content - it may have changed since creation
        if let Some(current_page_num) = notebook_clone.page_num(&terminal_box_clone) {
            // Remove the terminal tab regardless of whether it's the last one
            notebook_clone.remove_page(Some(current_page_num));
        }
    });
    
    page_num
}

/// Creates a container box for the terminal notebook with the add button
/// 
/// The terminal notebook is placed in a box and the add button is placed as an action button
/// in the notebook's tab bar area using the notebook's action widget feature
pub fn create_terminal_notebook_box(terminal_notebook: &Notebook, add_terminal_button: &Button) -> GtkBox {
    let terminal_box = GtkBox::new(Orientation::Vertical, 0);
    
    // Add the add button to the tab bar via the action widget feature
    // This places the button in the same row as the tabs
    terminal_notebook.set_action_widget(add_terminal_button, gtk4::PackType::End);
    
    // Set the terminal notebook to expand vertically
    terminal_notebook.set_vexpand(true);
    
    // Pack just the notebook into the container box
    terminal_box.append(terminal_notebook);
    
    // Make the entire container expand vertically
    terminal_box.set_vexpand(true);
    
    terminal_box
}

/// Creates a status bar for the bottom of the application
///
/// This function creates a status bar with a horizontal box to display the current directory path
/// as a series of clickable buttons, one for each directory level
/// 
/// Returns a tuple of:
/// - GtkBox: The status bar container
/// - GtkBox: The path box that will contain individual path segment buttons
pub fn create_status_bar() -> (GtkBox, GtkBox) {
    // Create a horizontal box for the status bar
    let status_bar = GtkBox::new(Orientation::Horizontal, 5);
    status_bar.set_margin_start(10);
    status_bar.set_margin_end(10);
    status_bar.set_margin_top(5);
    status_bar.set_margin_bottom(5);
    
    // Create a horizontal box to hold the path segment buttons
    let path_box = GtkBox::new(Orientation::Horizontal, 2);
    path_box.set_halign(gtk4::Align::Start); // Align to the left
    path_box.set_hexpand(true); // Use all available horizontal space
    
    // Add some styling to make the path box visually distinct
    path_box.add_css_class("path-box");
    
    // Add the path box to the status bar
    status_bar.append(&path_box);
    
    // Add a CSS class for custom styling
    status_bar.add_css_class("basado-status-bar");
    
    (status_bar, path_box)
}

/// Apply custom CSS to enhance the appearance of tabs
/// 
/// This function creates and applies CSS styles to improve the tab appearance,
/// making them look less flat and more visually distinct.
pub fn apply_custom_css() {
    let provider = gtk4::CssProvider::new();
    
    let css = build_complete_css();
    
    // Load and apply the CSS
    provider.load_from_data(&css);
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
    );
}

/// Builds the complete CSS string by combining all component styles
fn build_complete_css() -> String {
    format!(
        "{}{}{}{}",
        get_notebook_tab_styles(),
        get_button_styles(),
        get_status_bar_styles(),
        get_path_navigation_styles()
    )
}

/// Returns CSS styles for notebook tabs and related components
fn get_notebook_tab_styles() -> &'static str {
    "
    /* === NOTEBOOK AND TAB STYLES === */
    
    /* Notebook header container */
    notebook > header {
        padding: 1px;
        margin: 0;
    }
    
    notebook > header > tabs {
        margin: 0;
        padding: 1px;
    }
    
    /* Base tab styling */
    tab {
        padding: 3px 6px;
        min-width: 120px;
        min-height: 26px;
        border-radius: 5px 5px 0 0;
        border-bottom: 3px solid transparent;
        background-color: shade(@theme_bg_color, 0.85);
        box-shadow: 0 -1px 2px -1px shade(@theme_bg_color, 1.1) inset;
        transition: all 0.2s ease;
        margin: 1px 2px 0 2px;
        margin-bottom: -1px;
    }
    
    /* Active/selected tab styling */
    tab:checked {
        background-color: shade(@theme_bg_color, 1.5);
        border-bottom: 3px solid @theme_selected_bg_color;
        box-shadow: 0 -2px 3px -1px shade(@theme_bg_color, 1.2) inset;
    }
    
    /* Tab label styling */
    .tab-label {
        min-width: 80px;
        padding: 1px 3px;
        margin: 0;
        font-size: 95%;
        opacity: 0.85;
    }
    
    tab:checked .tab-label {
        opacity: 1.0;
        font-weight: 500;
    }
    "
}

/// Returns CSS styles for buttons, including circular close buttons
fn get_button_styles() -> String {
    let is_dark_mode = crate::syntax::is_dark_mode_enabled();
    let active_tab_shade = if is_dark_mode { "2" } else { "0.85" };
    
    format!(
        "
    /* === BUTTON STYLES === */
    
    /* Circular button base styling */
    button.circular {{
        background-color: shade(@theme_bg_color, 0.85);
        min-height: 20px;
        min-width: 20px;
        padding: 1px;
        margin: 0;
        border: none;
        border-radius: 50%;
    }}
    
    /* Circular button icon styling */
    button.circular image {{
        background-color: shade(@theme_bg_color, 0.85);
        -gtk-icon-transform: scale(0.8);
        border-radius: 50%;
        min-height: 20px;
        min-width: 20px;
    }}
    
    /* Circular button styling in active tabs */
    tab:checked button.circular,
    tab:checked button.circular image {{
        background-color: shade(@theme_bg_color, {});
        border-radius: 50%;
        min-height: 20px;
        min-width: 20px;
    }}
    ",
        active_tab_shade
    )
}

/// Returns CSS styles for the status bar
fn get_status_bar_styles() -> &'static str {
    "
    /* === STATUS BAR STYLES === */
    
    .basado-status-bar {
        border-top: 1px solid alpha(#999, 0.3);
    }
    "
}

/// Returns CSS styles for path navigation components
fn get_path_navigation_styles() -> &'static str {
    "
    /* === PATH NAVIGATION STYLES === */
    
    .path-box {
        padding: 2px;
    }
    
    .path-segment-button {
        padding: 2px 4px;
        margin: 0 1px;
        border-radius: 4px;
        min-height: 24px;
        min-width: 24px;
        border: 1px solid transparent;
        transition: all 0.15s ease;
    }
    
    .path-segment-button:hover {
        background-color: alpha(#888, 0.1);
        border-color: alpha(#888, 0.3);
    }
    
    .path-separator {
        opacity: 0.7;
        margin: 0 1px;
        font-family: monospace;
    }
    "
}

/// Sets up the terminal color theme to match the editor's syntax highlighting theme
///
/// This function configures the VTE terminal colors to match the editor's color scheme
/// based on whether the application is in dark mode or light mode. It sets:
/// - Foreground (text) color
/// - Background color
/// - Cursor color
/// - Selection colors
/// - A 16-color palette (standard ANSI colors and bright variants)
/// 
/// The color scheme is designed to be readable and consistent with the editor's appearance.
fn setup_terminal_theme(terminal: &VteTerminal) {
    // Check if we're in dark mode to choose appropriate colors
    let is_dark_mode = crate::syntax::is_dark_mode_enabled();
    
    if is_dark_mode {
        // Dark mode color scheme
        // Set foreground (text) color to light gray/white
        terminal.set_color_foreground(&gdk::RGBA::new(0.85, 0.85, 0.85, 1.0));
        
        // Set background color to dark gray (not pure black for better readability)
        terminal.set_color_background(&gdk::RGBA::new(0.15, 0.15, 0.15, 1.0));
        
        // Set cursor color for visibility
        terminal.set_color_cursor(Some(&gdk::RGBA::new(0.8, 0.8, 0.8, 1.0)));
        
        // Set selection colors
        terminal.set_color_highlight(Some(&gdk::RGBA::new(0.3, 0.3, 0.5, 1.0)));
        terminal.set_color_highlight_foreground(Some(&gdk::RGBA::new(1.0, 1.0, 1.0, 1.0)));
        
        // Set the palette for ANSI colors
        let palette = [
            // Standard colors (0-7)
            gdk::RGBA::new(0.15, 0.15, 0.15, 1.0), // Black
            gdk::RGBA::new(0.8, 0.2, 0.2, 1.0),    // Red
            gdk::RGBA::new(0.2, 0.7, 0.2, 1.0),    // Green
            gdk::RGBA::new(0.8, 0.8, 0.0, 1.0),    // Yellow
            gdk::RGBA::new(0.2, 0.5, 0.8, 1.0),    // Blue
            gdk::RGBA::new(0.8, 0.2, 0.8, 1.0),    // Magenta
            gdk::RGBA::new(0.0, 0.7, 0.7, 1.0),    // Cyan
            gdk::RGBA::new(0.85, 0.85, 0.85, 1.0), // White
            
            // Bright colors (8-15)
            gdk::RGBA::new(0.3, 0.3, 0.3, 1.0),    // Bright Black
            gdk::RGBA::new(1.0, 0.3, 0.3, 1.0),    // Bright Red
            gdk::RGBA::new(0.3, 0.9, 0.3, 1.0),    // Bright Green
            gdk::RGBA::new(1.0, 1.0, 0.3, 1.0),    // Bright Yellow
            gdk::RGBA::new(0.3, 0.6, 0.9, 1.0),    // Bright Blue
            gdk::RGBA::new(0.9, 0.3, 0.9, 1.0),    // Bright Magenta
            gdk::RGBA::new(0.3, 0.9, 0.9, 1.0),    // Bright Cyan
            gdk::RGBA::new(1.0, 1.0, 1.0, 1.0),    // Bright White
        ];
        
        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();
        
        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs      // Palette references
        );
        
    } else {
        // Light mode color scheme
        // Set foreground (text) color to dark gray/black
        terminal.set_color_foreground(&gdk::RGBA::new(0.1, 0.1, 0.1, 1.0));
        
        // Set background color to white/very light gray
        terminal.set_color_background(&gdk::RGBA::new(0.98, 0.98, 0.98, 1.0));
        
        // Set cursor color for visibility
        terminal.set_color_cursor(Some(&gdk::RGBA::new(0.2, 0.2, 0.2, 1.0)));
        
        // Set selection colors
        terminal.set_color_highlight(Some(&gdk::RGBA::new(0.7, 0.7, 0.9, 1.0)));
        terminal.set_color_highlight_foreground(Some(&gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)));
        
        // Set the palette for ANSI colors
        let palette = [
            // Standard colors (0-7)
            gdk::RGBA::new(0.98, 0.98, 0.98, 1.0), // Black (actually white for background)
            gdk::RGBA::new(0.7, 0.0, 0.0, 1.0),    // Red
            gdk::RGBA::new(0.0, 0.6, 0.0, 1.0),    // Green
            gdk::RGBA::new(0.6, 0.6, 0.0, 1.0),    // Yellow
            gdk::RGBA::new(0.0, 0.3, 0.7, 1.0),    // Blue
            gdk::RGBA::new(0.7, 0.0, 0.7, 1.0),    // Magenta
            gdk::RGBA::new(0.0, 0.6, 0.6, 1.0),    // Cyan
            gdk::RGBA::new(0.1, 0.1, 0.1, 1.0),    // White (actually black/dark gray for text)
            
            // Bright colors (8-15)
            gdk::RGBA::new(0.8, 0.8, 0.8, 1.0),    // Bright Black (light gray)
            gdk::RGBA::new(0.9, 0.2, 0.2, 1.0),    // Bright Red
            gdk::RGBA::new(0.2, 0.8, 0.2, 1.0),    // Bright Green
            gdk::RGBA::new(0.8, 0.8, 0.2, 1.0),    // Bright Yellow
            gdk::RGBA::new(0.2, 0.4, 0.8, 1.0),    // Bright Blue
            gdk::RGBA::new(0.8, 0.2, 0.8, 1.0),    // Bright Magenta
            gdk::RGBA::new(0.2, 0.8, 0.8, 1.0),    // Bright Cyan
            gdk::RGBA::new(0.0, 0.0, 0.0, 1.0),    // Bright White (actually black)
        ];
        
        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();
        
        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs      // Palette references
        );
    }
}

/// Updates the theme for all terminals in the terminal notebook
/// 
/// This should be called whenever the system theme changes to ensure
/// the terminal colors match the new theme
pub fn update_all_terminal_themes(terminal_notebook: &Notebook) {
    println!("Updating themes for all terminal tabs...");
    // Go through all tabs in the terminal notebook
    for page_num in 0..terminal_notebook.n_pages() {
        if let Some(page) = terminal_notebook.nth_page(Some(page_num)) {
            // Try to find ScrolledWindow which contains our terminal
            if let Some(scrolled_window) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                if let Some(child) = scrolled_window.child() {
                    // Check if the child is a VteTerminal
                    if let Some(terminal) = child.downcast_ref::<VteTerminal>() {
                        println!("Updating theme for terminal tab {}", page_num);
                        setup_terminal_theme(terminal);
                        
                        // Force redraw
                        terminal.queue_draw();
                    }
                }
            }
        }
    }
    
    // Force the notebook to redraw
    terminal_notebook.queue_draw();
    
    // Print the current theme setting for debugging
    if let Some(settings) = gtk4::Settings::default() {
        let is_dark = settings.is_gtk_application_prefer_dark_theme();
        println!("Terminal colors updated. Dark mode is now: {}", 
            if is_dark { "enabled" } else { "disabled" });
    }
}

/// Creates a settings dialog for configuring editor preferences
///
/// This function creates a dialog where the user can:
/// - Choose preferred syntax highlighting color schemes
/// - Set other editor preferences
///
/// Returns the dialog for display
pub fn create_settings_dialog(parent: &ApplicationWindow) -> Dialog {
    // Create a dialog with standard buttons
    let dialog = Dialog::builder()
        .title("Editor Settings")
        .transient_for(parent)
        .modal(true)
        .destroy_with_parent(true)
        .use_header_bar(1) // Use header bar
        .build();
    
    // Get the content area to add our widgets
    let content_area = dialog.content_area();
    content_area.set_margin_top(10);
    content_area.set_margin_bottom(10);
    content_area.set_margin_start(10);
    content_area.set_margin_end(10);
    content_area.set_spacing(10);
    
    // Create a container for the settings
    let settings_box = GtkBox::new(Orientation::Vertical, 10);
    
    // Create a section for syntax highlighting themes
    let themes_label = Label::new(Some("Syntax Highlighting Themes"));
    themes_label.set_halign(gtk4::Align::Start);
    themes_label.set_margin_bottom(5);
    themes_label.add_css_class("heading");
    settings_box.append(&themes_label);
    
    // Add info about current system theme mode
    let system_mode = if syntax::is_dark_mode_enabled() {
        "dark mode"
    } else {
        "light mode" 
    };
    let current_system_theme_name = syntax::get_preferred_style_scheme();
    let theme_info = Label::new(Some(&format!("Current system theme: {} (using {})", current_system_theme_name, system_mode)));
    theme_info.set_halign(gtk4::Align::Start);
    theme_info.set_margin_bottom(10);
    theme_info.add_css_class("caption");
    settings_box.append(&theme_info);
    
    // Get available color schemes
    let scheme_manager = StyleSchemeManager::new();
    let available_schemes: Vec<String> = scheme_manager.scheme_ids()
        .iter()
        .map(|s| s.to_string())
        .collect();
    
    // Debug: print available schemes
    println!("Available style schemes: {:?}", available_schemes);
    
    // Make sure we have the latest settings
    settings::refresh_settings();
    
    // Get current settings
    let settings_instance = settings::get_settings();
    let current_light_theme = settings_instance.get_light_theme();
    let current_dark_theme = settings_instance.get_dark_theme();
    
    println!("Settings dialog using - Light theme: {}, Dark theme: {}", 
             current_light_theme, current_dark_theme);
    
    // Get current theme based on system theme
    let current_system_theme = syntax::get_preferred_style_scheme();
    
    // Create dropdowns for light and dark themes
    // Select the appropriate theme based on the current system state
    let light_theme_box = if !syntax::is_dark_mode_enabled() {
        // If we're in light mode, prioritize the current system theme for the light theme dropdown
        create_theme_selection_box("Light Mode Theme:", &available_schemes, current_system_theme.clone())
    } else {
        create_theme_selection_box("Light Mode Theme:", &available_schemes, current_light_theme)
    };
    
    let dark_theme_box = if syntax::is_dark_mode_enabled() {
        // If we're in dark mode, prioritize the current system theme for the dark theme dropdown
        create_theme_selection_box("Dark Mode Theme:", &available_schemes, current_system_theme.clone())
    } else {
        create_theme_selection_box("Dark Mode Theme:", &available_schemes, current_dark_theme)
    };
    
    settings_box.append(&light_theme_box.0);
    settings_box.append(&dark_theme_box.0);
    
    // Add the settings box to the content area
    content_area.append(&settings_box);
    
    // Add save and cancel buttons
    dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
    dialog.add_button("Save", gtk4::ResponseType::Accept);
    dialog.set_default_response(gtk4::ResponseType::Accept);
    
    // Handle the dialog response
    // We need to capture the dropdowns and available_schemes to get their values when the user clicks Save
    let light_dropdown = light_theme_box.1;
    let dark_dropdown = dark_theme_box.1;
    let available_schemes_clone = available_schemes.clone();
    
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            // Get the selected theme values from the position in the dropdown
            let light_position = light_dropdown.selected() as usize;
            if light_position < available_schemes_clone.len() {
                let light_theme = available_schemes_clone[light_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_light_theme(&light_theme);
            }
            
            let dark_position = dark_dropdown.selected() as usize;
            if dark_position < available_schemes_clone.len() {
                let dark_theme = available_schemes_clone[dark_position].clone();
                let mut settings = settings::get_settings_mut();
                settings.set_dark_theme(&dark_theme);
            }
            
            // Save settings to disk
            if let Err(e) = settings::get_settings_mut().save() {
                eprintln!("Failed to save settings: {}", e);
            }
            
            // Release the mutex before refreshing settings
            drop(settings::get_settings_mut());
            
            // Refresh settings across the application
            settings::refresh_settings();
            
            // Get a reference to the parent window to update themes
            if let Some(parent) = dialog.transient_for() {
                if let Ok(parent_window) = parent.downcast::<ApplicationWindow>() {
                    // Apply theme changes throughout the application
                    apply_theme_changes_globally(&parent_window);
                }
            }
        }
        
        dialog.close();
    });
    
    dialog
}

/// Creates a theme selection dropdown with label
///
/// Returns a tuple containing:
/// - A container with the label and dropdown
/// - The dropdown widget for connecting signals
fn create_theme_selection_box(label_text: &str, available_themes: &[String], current_theme: String) 
    -> (GtkBox, gtk4::DropDown) 
{
    let box_container = GtkBox::new(Orientation::Horizontal, 10);
    
    // Add label
    let label = Label::new(Some(label_text));
    label.set_halign(gtk4::Align::Start);
    label.set_width_chars(20);
    label.set_xalign(0.0);
    box_container.append(&label);
    
    // Create a string list model for the dropdown
    let model = gtk4::StringList::new(&[]);
    for theme in available_themes {
        model.append(theme);
    }
    
    // Create dropdown
    let dropdown = gtk4::DropDown::new(Some(model), None::<gtk4::Expression>);
    dropdown.set_hexpand(true);
    
    // Set current selection
    for (idx, theme) in available_themes.iter().enumerate() {
        if theme == &current_theme {
            dropdown.set_selected(idx as u32);
            break;
        }
    }
    
    box_container.append(&dropdown);
    
    (box_container, dropdown)
}

/// Finds all notebooks within a window
/// 
/// This function finds all notebook widgets in the window.
fn find_notebooks(window: &ApplicationWindow) -> Vec<Notebook> {
    // For simplicity, we'll look for notebooks by their names or in specific locations
    // This is a simplified approach - in a real application you might want to traverse
    // the widget tree properly
    
    let mut result = Vec::new();
    
    // In this editor, we know there's a main notebook for editors
    // We can assume it's the main editor notebook based on your application structure
    if let Some(notebook) = window
        .child()
        .and_then(|child| child.first_child())
        .and_then(|box_container| box_container.first_child()) 
    {
        if let Ok(notebook) = notebook.downcast::<Notebook>() {
            result.push(notebook);
        }
    }
    
    result
}

/// Updates the themes in all sourceview buffers in a notebook
/// 
/// This function updates all sourceview buffers in a notebook with the current theme
fn update_notebook_themes(notebook: &Notebook) {
    for i in 0..notebook.n_pages() {
        if let Some(page) = notebook.nth_page(Some(i)) {
            // Try to find a ScrolledWindow inside the page
            if let Some(scrolled) = page.first_child() {
                if let Ok(scrolled_window) = scrolled.downcast::<ScrolledWindow>() {
                    // Try to get the child of the scrolled window, which should be our SourceView
                    if let Some(child) = scrolled_window.child() {
                        // Check if this is a SourceView
                        if let Ok(source_view) = child.downcast::<sourceview5::View>() {
                            // Get the buffer and update its theme
                            let buffer = source_view.buffer();
                            // Here we can safely downcast to SourceBuffer
                            if let Ok(source_buffer) = buffer.downcast::<sourceview5::Buffer>() {
                                // Update the buffer's theme
                                syntax::update_buffer_style_scheme(&source_buffer);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Updates themes throughout the application
/// 
/// This function updates all theme-related components to reflect the current settings:
/// - Updates all editor buffers with the current syntax highlighting theme
/// - Updates all terminal tabs with matching theme colors
/// - Updates any other UI elements that depend on theme settings
/// 
/// Should be called after changing theme settings.
pub fn apply_theme_changes_globally(parent_window: &ApplicationWindow) {
    println!("Applying global theme changes...");
    
    // Get fresh settings to ensure we have the latest values
    let settings = crate::settings::get_settings();
    println!("Current settings - Light theme: {}, Dark theme: {}", 
             settings.get_light_theme(), settings.get_dark_theme());
    
    // Use the robust buffer update function from main.rs instead of the simpler one
    // This ensures all editor buffers are updated regardless of their widget structure
    crate::update_all_buffer_themes(parent_window);
    
    // Find the terminal notebook if it exists
    if let Some(terminal_notebook) = find_terminal_notebook(parent_window) {
        // Update terminal themes
        update_all_terminal_themes(&terminal_notebook);
    }
    
    // Force a redraw of the window to ensure theme changes are visible
    parent_window.queue_draw();
    
    println!("Theme changes applied successfully");
}

/// Finds the terminal notebook within a window
fn find_terminal_notebook(window: &ApplicationWindow) -> Option<Notebook> {
    // Look through the window structure to find the terminal notebook
    // This is specific to the structure of our application window
    
    window.child()
        .and_then(|main_box| main_box.first_child())  // Main content box
        .and_then(|paned| paned.last_child())         // The horizontal paned container
        .and_then(|editor_paned| editor_paned.last_child()) // The vertical paned container
        .and_then(|terminal_box| terminal_box.first_child())
        .and_then(|child| {
            // Check if this is our terminal notebook
            if let Ok(notebook) = child.downcast::<Notebook>() {
                Some(notebook)
            } else {
                None
            }
        })
}

/// Manually refreshes all themes in the application
/// This can be called when you suspect the automatic theme detection isn't working
pub fn manually_refresh_themes(window: &ApplicationWindow) {
    println!("=== Manual Theme Refresh ===");
    
    // Debug current theme state
    crate::syntax::debug_theme_detection();
    
    // Find all notebooks in the window and update their themes
    let notebooks = find_notebooks(window);
    for notebook in &notebooks {
        update_notebook_themes(notebook);
    }
    
    // Update terminal themes if they exist
    if let Some(terminal_notebook) = find_terminal_notebook(window) {
        update_all_terminal_themes(&terminal_notebook);
    }
    
    // Force a complete redraw
    window.queue_draw();
    
    println!("Manual theme refresh completed");
}
