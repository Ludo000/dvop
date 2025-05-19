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
    Orientation
};

// Import our syntax highlighting module
use crate::syntax;

// Standard library imports
use std::cell::RefCell;  // For interior mutability pattern
use std::rc::Rc;         // For shared ownership
use std::env;            // For environment variables
use std::path::PathBuf;  // For file paths

// Terminal emulator support
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;

// Home directory detection
use home;

/// Creates the main application window with default settings
pub fn create_window(app: &Application) -> ApplicationWindow {
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
pub fn create_header() -> (HeaderBar, Button, Button, Button, MenuButton, Button) {
    // Create the main header bar
    let header = HeaderBar::new();

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
    (header, new_button, open_button, save_main_button, save_menu_button, save_as_button)
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
pub fn create_terminal() -> VteTerminal {
    let terminal = VteTerminal::new();
    
    // Get the user's default shell from environment variables
    if let Some(shell) = env::var("SHELL").ok() {
        // Use the user's home directory as the starting directory for the terminal
        let home_dir = home::home_dir().expect("Could not find home directory");
        
        if let Some(home_dir_str) = home_dir.to_str() {
            // Spawn the shell asynchronously in the terminal
            terminal.spawn_async(
                vte4::PtyFlags::DEFAULT,          // Default pseudo-terminal flags
                Some(home_dir_str),               // Working directory
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
            eprintln!("Failed to convert home directory path to string");
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
        .vexpand(false)            // Don't expand vertically to fill all available space
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
pub fn create_file_manager_panel() -> (ListBox, ScrolledWindow, GtkBox, Button, Button) {
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
    
    // Create the "Refresh" button with a standard icon
    let refresh_button_icon = Image::from_icon_name("view-refresh-symbolic");
    let refresh_button = Button::new();
    refresh_button.set_child(Some(&refresh_button_icon));
    refresh_button.set_margin_end(5); // Add right margin
    
    // Create an expanding spacer to push buttons to opposite sides
    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true); // Make the spacer expand to push buttons apart
    
    // Assemble the navigation toolbar
    nav_box.append(&up_button);      // Up button on left
    nav_box.append(&spacer);         // Expanding space in middle
    nav_box.append(&refresh_button); // Refresh button on right
    
    // Return the components for further assembly and event handling
    (file_list_box, scrolled_window, nav_box, up_button, refresh_button)
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
pub fn create_paned(
    file_manager_panel: &GtkBox,     // File browser sidebar
    editor_notebook: &Notebook,      // Editor tabs container
    terminal_box: &impl IsA<gtk4::Widget>  // Terminal container (either ScrolledWindow or GtkBox)
) -> gtk4::Paned {
    // Create the main horizontal split pane
    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);  // Use a wider drag handle for easier resizing
    
    // Create the vertical split pane for the right side
    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true);
    
    // Place editor notebook at the top of the vertical split
    editor_paned.set_start_child(Some(editor_notebook));
    
    // Place terminal at the bottom of the vertical split
    editor_paned.set_end_child(Some(terminal_box));
    
    // Place file manager on the left side of the horizontal split
    paned.set_start_child(Some(file_manager_panel));
    
    // Place the editor+terminal vertical split on the right side
    paned.set_end_child(Some(&editor_paned));
    
    // Set initial split positions
    paned.set_position(200);        // Width of file manager sidebar
    editor_paned.set_position(400); // Height of editor area
    
    paned
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
    // Create horizontal container for tab contents
    let tab_box = GtkBox::new(Orientation::Horizontal, 5);
    tab_box.set_margin_bottom(2); // Add a small bottom margin for visual spacing
    
    // Create label with the provided title
    let label = Label::new(Some(tab_title));
    
    // Create close button with a standard X icon
    let close_button = Button::from_icon_name("window-close-symbolic");
    
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
    
    // Create an "Add Terminal" button
    let add_terminal_button = Button::from_icon_name("list-add-symbolic");
    add_terminal_button.set_tooltip_text(Some("Add a new terminal tab"));
    
    // Create the first terminal tab
    add_terminal_tab(&terminal_notebook);
    
    // Connect the Add Terminal button click handler
    let terminal_notebook_clone = terminal_notebook.clone();
    add_terminal_button.connect_clicked(move |_| {
        add_terminal_tab(&terminal_notebook_clone);
    });
    
    (terminal_notebook, add_terminal_button)
}

/// Adds a new terminal tab to the terminal notebook
/// 
/// Creates a new terminal instance, places it in a tab, and adds it to the notebook
fn add_terminal_tab(terminal_notebook: &Notebook) -> u32 {
    // Create a new terminal
    let terminal = create_terminal();
    let terminal_box = create_terminal_box(&terminal);
    
    // Create a tab widget with a close button
    let tab_index = terminal_notebook.n_pages();
    let tab_title = format!("Terminal {}", tab_index + 1);
    let (tab_widget, _tab_label, tab_close_button) = create_tab_widget(&tab_title);
    
    // Append the terminal to the notebook
    let page_num = terminal_notebook.append_page(&terminal_box, Some(&tab_widget));
    terminal_notebook.set_current_page(Some(page_num));
    
    // Connect the close button
    let notebook_clone = terminal_notebook.clone();
    tab_close_button.connect_clicked(move |_| {
        // Don't close the last terminal tab
        if notebook_clone.n_pages() > 1 {
            notebook_clone.remove_page(Some(page_num));
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
    
    // Pack just the notebook into the container box
    terminal_box.append(terminal_notebook);
    
    terminal_box
}
