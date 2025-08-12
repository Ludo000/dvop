// UI module for the Basado Text Editor
// Contains all UI component creation and layout functions

pub mod terminal;
pub mod file_manager;
pub mod css;
pub mod settings;

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

    // Create a hidden new button for backward compatibility with existing handler code
    let new_button = Button::new();
    new_button.set_visible(false);

    // Return the header and all action buttons (new_button is now hidden for compatibility)
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
        tab_close_button,  // Button to close the tab
        add_file_button    // Button to add new file tabs
    )
}

/// Creates the main application layout using paned containers
///
/// This function arranges the major UI components into a nested paned layout:
/// - Horizontal split between file manager (left) and editor+terminal (right)
/// - The right side has a vertical split between editor (top) and terminal (bottom)
pub fn create_paned(
    file_manager_panel: &GtkBox,     // File browser sidebar
    editor_notebook_box: &GtkBox,    // Editor notebook container with add button
    terminal_box: &impl IsA<gtk4::Widget>,  // Terminal container (either ScrolledWindow or GtkBox)
) -> gtk4::Paned {
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

/// Creates a container box for the editor notebook with the add button
/// 
/// The editor notebook is placed in a box and the add button is placed as an action button
/// in the notebook's tab bar area using the notebook's action widget feature
pub fn create_editor_notebook_box(editor_notebook: &Notebook, add_file_button: &Button) -> GtkBox {
    let editor_box = GtkBox::new(Orientation::Vertical, 0);
    
    // Add the add button to the tab bar via the action widget feature
    // This places the button in the same row as the tabs
    editor_notebook.set_action_widget(add_file_button, gtk4::PackType::End);
    
    // Set the editor notebook to expand vertically
    editor_notebook.set_vexpand(true);
    
    // Pack just the notebook into the container box
    editor_box.append(editor_notebook);
    
    // Make the entire container expand vertically
    editor_box.set_vexpand(true);
    
    editor_box
}

/// Creates a status bar for the bottom of the application
///
/// This function creates a status bar with a horizontal box to display the current directory path
/// as a series of clickable buttons, one for each directory level
/// 
/// Returns a tuple of:
/// - GtkBox: The status bar container
/// - GtkBox: The path box that will contain individual path segment buttons
#[allow(dead_code)]
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


