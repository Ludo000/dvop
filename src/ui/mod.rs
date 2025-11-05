// UI module for Dvop
// Contains all UI component creation and layout functions

pub mod terminal;
pub mod file_manager;
pub mod css;
pub mod settings;
pub mod global_search;
pub mod git_diff;
pub mod search_panel_template;
pub mod git_diff_panel_template;
pub mod settings_dialog_template;

use gtk4::prelude::*;
use gtk4::{
    // Main application and window components
    Application, ApplicationWindow, 
    
    // Layout containers
    Box as GtkBox, Notebook,
    
    // Common UI elements
    Button, HeaderBar, Label, Picture, TextView, Image, TextBuffer, Scale,
    
    // Menu components for split button functionality
    MenuButton, gio, glib,
    
    // Layout orientation for containers
    Orientation
};

// Import our modules
use crate::syntax;
use std::cell::RefCell;  // For interior mutability pattern
use std::rc::Rc;         // For shared ownership
use std::path::PathBuf;  // For file paths
use std::collections::HashMap;  // For file path manager

// Template support
use gtk4::subclass::prelude::*;

// Home directory detection

mod imp {
    use super::*;
    use gtk4::CompositeTemplate;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/window.ui")]
    pub struct DvopWindow {
        // Header bar widgets
        #[template_child]
        pub header_bar: TemplateChild<HeaderBar>,
        #[template_child]
        pub menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub menu_search_entry: TemplateChild<gtk4::SearchEntry>,
        #[template_child]
        pub open_button: TemplateChild<Button>,
        #[template_child]
        pub save_main_button: TemplateChild<Button>,
        #[template_child]
        pub save_menu_button: TemplateChild<MenuButton>,
        
        // Path bar widgets
        #[template_child]
        pub path_bar: TemplateChild<GtkBox>,
        #[template_child]
        pub up_button: TemplateChild<Button>,
        #[template_child]
        pub path_box: TemplateChild<GtkBox>,
        #[template_child]
        pub volume_control_box: TemplateChild<GtkBox>,
        #[template_child]
        pub volume_icon: TemplateChild<Image>,
        #[template_child]
        pub global_volume_scale: TemplateChild<Scale>,
        #[template_child]
        pub volume_label: TemplateChild<Label>,
        #[template_child]
        pub refresh_button: TemplateChild<Button>,
        #[template_child]
        pub terminal_button: TemplateChild<Button>,
        
        // Activity bar
        #[template_child]
        pub activity_bar: TemplateChild<GtkBox>,
        #[template_child]
        pub explorer_button: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub search_button: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub git_diff_button: TemplateChild<gtk4::ToggleButton>,
        
        // Main layout
        #[template_child]
        pub paned: TemplateChild<gtk4::Paned>,
        #[template_child]
        pub editor_paned: TemplateChild<gtk4::Paned>,
        #[template_child]
        pub sidebar_stack: TemplateChild<gtk4::Stack>,
        
        // File manager
        #[template_child]
        pub file_manager_panel: TemplateChild<GtkBox>,
        #[template_child]
        pub file_list_box: TemplateChild<gtk4::ListBox>,
        
        // Search panel
        #[template_child]
        pub search_panel: TemplateChild<GtkBox>,
        
        // Git diff panel
        #[template_child]
        pub git_diff_panel: TemplateChild<GtkBox>,
        
        // Editor
        #[template_child]
        pub editor_notebook: TemplateChild<Notebook>,
        #[template_child]
        pub search_bar: TemplateChild<gtk4::SearchBar>,
        
        // Terminal
        #[template_child]
        pub terminal_notebook: TemplateChild<Notebook>,
        #[template_child]
        pub add_terminal_button: TemplateChild<Button>,
        
        // Status bar
        #[template_child]
        pub status_bar: TemplateChild<GtkBox>,
        #[template_child]
        pub status_button: TemplateChild<Button>,
        #[template_child]
        pub status_label: TemplateChild<Label>,
        #[template_child]
        pub secondary_status_label: TemplateChild<Label>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DvopWindow {
        const NAME: &'static str = "DvopWindow";
        type Type = super::DvopWindow;
        type ParentType = ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for DvopWindow {}
    impl WidgetImpl for DvopWindow {}
    impl WindowImpl for DvopWindow {}
    impl ApplicationWindowImpl for DvopWindow {}
}

glib::wrapper! {
    pub struct DvopWindow(ObjectSubclass<imp::DvopWindow>)
        @extends gtk4::Widget, gtk4::Window, ApplicationWindow;
}

impl DvopWindow {
    pub fn new(app: &Application) -> Self {
        // Apply CSS before creating the window
        css::apply_custom_css();
        
        // Load resources
        let resources = gio::Resource::load(
            std::path::Path::new(env!("OUT_DIR")).join("resources.gresource")
        ).expect("Could not load resources");
        gio::resources_register(&resources);
        
        // Get saved window dimensions from settings
        let settings = crate::settings::get_settings();
        let window_width = settings.get_window_width();
        let window_height = settings.get_window_height();
        let file_panel_width = settings.get_file_panel_width();
        let terminal_height = settings.get_terminal_height();
        
        let window: Self = glib::Object::builder()
            .property("application", app)
            .property("default-width", window_width)
            .property("default-height", window_height)
            .build();
        
        // Load icon
        window.setup_icon();
        
        // Set paned positions
        let imp = window.imp();
        imp.paned.set_position(file_panel_width);
        imp.editor_paned.set_position(terminal_height);
        
        // Set initial volume
        let initial_volume = settings.get_audio_volume();
        imp.global_volume_scale.set_value(initial_volume);
        let volume_percent = (initial_volume * 100.0) as i32;
        imp.volume_label.set_text(&format!("{}%", volume_percent));
        
        window
    }
    
    fn setup_icon(&self) {
        // Load the custom icon into the icon theme
        if let Some(display) = gtk4::gdk::Display::default() {
            let icon_theme = gtk4::IconTheme::for_display(&display);
            
            // Embedded icon data (fallback)
            const ICON_DATA: &[u8] = include_bytes!("../../dvop.svg");
            
            // Try to create icon file in config directory if it doesn't exist in search paths
            let config_dir = crate::settings::get_settings().config_dir();
            let icon_path = config_dir.join("dvop.svg");
            
            // Write embedded icon to config directory if it doesn't exist
            if !icon_path.exists() {
                if let Ok(()) = std::fs::write(&icon_path, ICON_DATA) {
                    icon_theme.add_search_path(&config_dir);
                }
            } else {
                icon_theme.add_search_path(&config_dir);
            }
            
            // Try to add icon search path from executable directory (for installed version)
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(parent) = exe_path.parent() {
                    let icon_file = parent.join("dvop.svg");
                    if icon_file.exists() {
                        icon_theme.add_search_path(parent);
                    } else {
                        // Write embedded icon next to executable if possible
                        let _ = std::fs::write(&icon_file, ICON_DATA);
                        if icon_file.exists() {
                            icon_theme.add_search_path(parent);
                        }
                    }
                }
            }
            
            // Also try project root (for development)
            if let Ok(current_dir) = std::env::current_dir() {
                let dev_icon = current_dir.join("dvop.svg");
                if dev_icon.exists() {
                    icon_theme.add_search_path(&current_dir);
                }
            }
        }
    }
    
    pub fn imp(&self) -> &imp::DvopWindow {
        imp::DvopWindow::from_obj(self)
    }
}

/// Creates the main application window with default settings
pub fn create_window(app: &Application) -> DvopWindow {
    DvopWindow::new(app)
}

/// Creates the application header bar with action buttons
///
/// This function returns references to the action buttons for connecting event handlers.
/// The header bar is now part of the template and doesn't need to be returned.
pub fn create_header(window: &DvopWindow) -> (Button, Button, Button, MenuButton, Button, Button, Button) {
    let imp = window.imp();
    
    // Create hidden buttons for backward compatibility
    let new_button = Button::new();
    new_button.set_visible(false);
    
    let save_as_button = Button::new();
    let save_as_button_icon = Image::from_icon_name("document-save-as-symbolic");
    let save_as_button_label = Label::new(Some("Save As"));
    let save_as_button_box = GtkBox::new(Orientation::Horizontal, 5);
    save_as_button_box.append(&save_as_button_icon);
    save_as_button_box.append(&save_as_button_label);
    save_as_button.set_child(Some(&save_as_button_box));
    save_as_button.set_tooltip_text(Some("Save the current file with a new name"));
    save_as_button.set_visible(false);
    
    let save_button = Button::new();
    save_button.set_visible(false);
    
    let settings_button = Button::new();
    settings_button.set_visible(false);
    
    (
        new_button,
        imp.open_button.get(),
        imp.save_main_button.get(),
        imp.save_menu_button.get(),
        save_as_button,
        save_button,
        settings_button,
    )
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
pub fn create_text_view(window: &DvopWindow) -> (
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
    let imp = window.imp();
    let editor_notebook = imp.editor_notebook.get();

    // Create an "Add File" button similar to the terminal's add button
    let add_file_button = Button::from_icon_name("list-add-symbolic");
    add_file_button.set_tooltip_text(Some("Create a new file"));
    add_file_button.set_margin_end(8); // Add right padding

    // Create the first "Untitled" tab
    let (tab_widget, tab_label, tab_close_button) = create_tab_widget("Untitled");
    
    // Add middle mouse click support for the tab
    setup_tab_middle_click(&tab_widget, &tab_close_button);
    
    // Note: Right-click menu setup is deferred until after dependencies are created in main.rs
    
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
    
    // Set action widget for add button
    editor_notebook.set_action_widget(&add_file_button, gtk4::PackType::End);

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
        editor_notebook.clone(),   // Main tabbed container for multiple documents
        tab_widget,        // Container for tab components
        tab_label,         // Label showing filename in tab
        tab_close_button,  // Button to close the tab
        add_file_button    // Button to add new file tabs
    )
}

/// Returns references to the paned components from the template
///
/// Returns a tuple of:
/// - GtkBox: The main container with activity bar + paned content (dummy for compatibility)
/// - gtk4::Paned: The main horizontal paned container
/// - gtk4::Paned: The vertical paned container for editor+terminal
/// - gtk4::ToggleButton: The explorer button from activity bar
/// - gtk4::ToggleButton: The search button from activity bar
/// - gtk4::ToggleButton: The git diff button from activity bar
/// - gtk4::Stack: The sidebar stack for switching panels
pub fn create_paned(
    window: &DvopWindow,
) -> (GtkBox, gtk4::Paned, gtk4::Paned, gtk4::ToggleButton, gtk4::ToggleButton, gtk4::ToggleButton, gtk4::Stack) {
    let imp = window.imp();
    
    // Create dummy box for backward compatibility (not used in template approach)
    let dummy_box = GtkBox::new(Orientation::Horizontal, 0);
    
    (
        dummy_box,
        imp.paned.get(),
        imp.editor_paned.get(),
        imp.explorer_button.get(),
        imp.search_button.get(),
        imp.git_diff_button.get(),
        imp.sidebar_stack.get(),
    )
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
    
    // Set responsive sizing - min and max width for better adaptability
    tab_box.set_size_request(100, -1);  // Minimum width
    tab_box.set_hexpand(false); // Prevent excessive expansion
    
    // Create label with the provided title
    let label = Label::new(Some(tab_title));
    label.set_ellipsize(gtk4::pango::EllipsizeMode::End); // Add ellipsis if text overflows
    label.set_xalign(0.0); // Align text to the left
    label.set_hexpand(true); // Expand to fill available space
    label.set_max_width_chars(20); // Limit maximum width for better responsiveness
    label.set_width_chars(12); // Preferred width
    label.add_css_class("tab-label"); // Add custom CSS class for styling
    
    // Create close button with a standard X icon
    let close_button = Button::from_icon_name("window-close-symbolic");
    
    // Use a comfortably sized button
    close_button.add_css_class("circular"); // Make button more rounded
    close_button.set_valign(gtk4::Align::Center);
    close_button.set_halign(gtk4::Align::End); // Align button to the right
    close_button.set_hexpand(false); // Don't expand
    
    // Set comfortable button margins
    close_button.set_margin_start(4);
    
    // Make the button a comfortable size
    close_button.set_size_request(20, 20);
    
    // Assemble tab components: label and close button (no spacer)
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

/// Adds right-click context menu support to a tab widget with "Close All" option
///
/// This function sets up a gesture click controller that listens for
/// right mouse button clicks on the tab and displays a context menu.
pub fn setup_tab_right_click(
    tab_box: &GtkBox,
    notebook: &Notebook,
    window: &ApplicationWindow,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &gtk4::ListBox,
    new_tab_deps: Option<crate::handlers::NewTabDependencies>,
) {
    use gtk4::prelude::*;
    
    // Create a gesture click controller that responds to right mouse button clicks
    let right_click_gesture = gtk4::GestureClick::new();
    right_click_gesture.set_button(3); // Right mouse button
    
    // Clone the notebook and tab_box for the closure
    let notebook_clone = notebook.clone();
    let tab_box_clone = tab_box.clone();
    let window_clone = window.clone();
    let file_path_manager_clone = file_path_manager.clone();
    let _active_tab_path_clone = active_tab_path.clone();
    let _current_dir_clone = current_dir.clone();
    let _file_list_box_clone = file_list_box.clone();
    let _new_tab_deps_clone = new_tab_deps.clone();
    
    // Connect the pressed signal to show context menu
    right_click_gesture.connect_pressed(move |_, _n_press, x, y| {
        crate::status_log::log_info("Right-click detected on tab - showing menu");
        
        // Find which page was right-clicked by finding the page containing this tab_box
        let mut clicked_page_num = None;
        for page_num in 0..notebook_clone.n_pages() {
            if let Some(page) = notebook_clone.nth_page(Some(page_num)) {
                if let Some(tab_label) = notebook_clone.tab_label(&page) {
                    if tab_label == tab_box_clone {
                        clicked_page_num = Some(page_num);
                        break;
                    }
                }
            }
        }
        
        // Create a popover for the context menu
        let popover = gtk4::Popover::new();
        popover.set_autohide(true);
        popover.set_has_arrow(true);
        popover.set_can_focus(false);
        
        // Create a box to hold the menu items
        let menu_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
        menu_box.add_css_class("menu");
        
        // Create "Close Others" button
        let close_others_button = Button::with_label("Close Others");
        close_others_button.add_css_class("flat");
        close_others_button.set_hexpand(true);
        close_others_button.set_halign(gtk4::Align::Start);
        
        // Disable "Close Others" if there's only one tab
        if notebook_clone.n_pages() <= 1 {
            close_others_button.set_sensitive(false);
        }
        
        // Clone for the button closure
        let notebook_for_close_others = notebook_clone.clone();
        let popover_weak_others = popover.downgrade();
        let window_for_close_others = window_clone.clone();
        let file_path_manager_for_close_others = file_path_manager_clone.clone();
        
        close_others_button.connect_clicked(move |_| {
            crate::status_log::log_info("Closing other tabs...");
            
            // Hide the context menu first
            if let Some(popover) = popover_weak_others.upgrade() {
                popover.popdown();
            }
            
            // Close all tabs except the clicked one
            if let Some(keep_page) = clicked_page_num {
                // Check if any tabs (except the kept one) have unsaved changes
                let mut unsaved_files = Vec::new();
                for page_num in 0..notebook_for_close_others.n_pages() {
                    if page_num != keep_page {
                        if let Some(page_widget) = notebook_for_close_others.nth_page(Some(page_num)) {
                            if let Some(tab_label_widget) = notebook_for_close_others.tab_label(&page_widget) {
                                if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                                    if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<gtk4::Label>().ok()) {
                                        if label.text().starts_with('*') {
                                            // Found an unsaved file
                                            let filename = file_path_manager_for_close_others.borrow()
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
                }
                
                // If there are unsaved files, show confirmation dialog
                if !unsaved_files.is_empty() {
                    let message = if unsaved_files.len() == 1 {
                        format!("You have unsaved changes in {}.\n\nAre you sure you want to close other tabs without saving?", unsaved_files[0])
                    } else {
                        format!("You have unsaved changes in {} files:\n• {}\n\nAre you sure you want to close other tabs without saving?", 
                                unsaved_files.len(), 
                                unsaved_files.join("\n• "))
                    };
                    
                    let dialog = gtk4::MessageDialog::new(
                        Some(&window_for_close_others),
                        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                        gtk4::MessageType::Warning,
                        gtk4::ButtonsType::None,
                        &message
                    );
                    
                    dialog.add_buttons(&[
                        ("Cancel", gtk4::ResponseType::Cancel),
                        ("Close Others Anyway", gtk4::ResponseType::Yes),
                    ]);
                    
                    dialog.set_default_response(gtk4::ResponseType::Cancel);
                    
                    let notebook_clone = notebook_for_close_others.clone();
                    dialog.connect_response(move |d, response| {
                        if response == gtk4::ResponseType::Yes {
                            // User confirmed - close other tabs without saving
                            // Close tabs after the kept page first (from end to beginning)
                            while notebook_clone.n_pages() > keep_page + 1 {
                                let last_page = notebook_clone.n_pages() - 1;
                                notebook_clone.remove_page(Some(last_page));
                            }
                            
                            // Close tabs before the kept page (from beginning, but now it's always index 0)
                            while keep_page > 0 && notebook_clone.n_pages() > 1 {
                                notebook_clone.remove_page(Some(0));
                            }
                            crate::status_log::log_success("Other tabs closed");
                        }
                        d.close();
                    });
                    
                    dialog.show();
                } else {
                    // No unsaved files, close other tabs directly
                    // Close tabs after the kept page first (from end to beginning)
                    while notebook_for_close_others.n_pages() > keep_page + 1 {
                        let last_page = notebook_for_close_others.n_pages() - 1;
                        notebook_for_close_others.remove_page(Some(last_page));
                    }
                    
                    // Close tabs before the kept page (from beginning, but now it's always index 0)
                    while keep_page > 0 && notebook_for_close_others.n_pages() > 1 {
                        notebook_for_close_others.remove_page(Some(0));
                    }
                    
                    crate::status_log::log_success("Other tabs closed");
                }
            }
        });
        
        // Create "Close All" button
        let close_all_button = Button::with_label("Close All");
        close_all_button.add_css_class("flat");
        close_all_button.set_hexpand(true);
        close_all_button.set_halign(gtk4::Align::Start);
        
        // Clone notebook for the button closure
        let notebook_for_close = notebook_clone.clone();
        let popover_weak = popover.downgrade();
        let window_for_close = window_clone.clone();
        let file_path_manager_for_close = file_path_manager_clone.clone();
        
        close_all_button.connect_clicked(move |_| {
            crate::status_log::log_info("Closing all tabs...");
            
            // Hide the context menu first
            if let Some(popover) = popover_weak.upgrade() {
                popover.popdown();
            }
            
            // Check if any tabs have unsaved changes
            let mut unsaved_files = Vec::new();
            for page_num in 0..notebook_for_close.n_pages() {
                if let Some(page_widget) = notebook_for_close.nth_page(Some(page_num)) {
                    if let Some(tab_label_widget) = notebook_for_close.tab_label(&page_widget) {
                        if let Some(tab_box) = tab_label_widget.downcast_ref::<gtk4::Box>() {
                            if let Some(label) = tab_box.first_child().and_then(|w| w.downcast::<gtk4::Label>().ok()) {
                                if label.text().starts_with('*') {
                                    // Found an unsaved file
                                    let filename = file_path_manager_for_close.borrow()
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
                    format!("You have unsaved changes in {}.\n\nAre you sure you want to close all tabs without saving?", unsaved_files[0])
                } else {
                    format!("You have unsaved changes in {} files:\n• {}\n\nAre you sure you want to close all tabs without saving?", 
                            unsaved_files.len(), 
                            unsaved_files.join("\n• "))
                };
                
                let dialog = gtk4::MessageDialog::new(
                    Some(&window_for_close),
                    gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
                    gtk4::MessageType::Warning,
                    gtk4::ButtonsType::None,
                    &message
                );
                
                dialog.add_buttons(&[
                    ("Cancel", gtk4::ResponseType::Cancel),
                    ("Close All Anyway", gtk4::ResponseType::Yes),
                ]);
                
                dialog.set_default_response(gtk4::ResponseType::Cancel);
                
                let notebook_clone = notebook_for_close.clone();
                dialog.connect_response(move |d, response| {
                    if response == gtk4::ResponseType::Yes {
                        // User confirmed - close all tabs without saving
                        while notebook_clone.n_pages() > 0 {
                            let last_page = notebook_clone.n_pages() - 1;
                            notebook_clone.remove_page(Some(last_page));
                        }
                        crate::status_log::log_success("All tabs closed");
                    }
                    d.close();
                });
                
                dialog.show();
            } else {
                // No unsaved files, close all tabs directly
                while notebook_for_close.n_pages() > 0 {
                    let last_page = notebook_for_close.n_pages() - 1;
                    notebook_for_close.remove_page(Some(last_page));
                }
                crate::status_log::log_success("All tabs closed");
            }
        });
        
        // Add buttons to menu
        menu_box.append(&close_others_button);
        menu_box.append(&close_all_button);
        
        // Set the menu box as the popover's child
        popover.set_child(Some(&menu_box));
        
        // Set the popover's parent to the notebook (not the tab_box)
        // This prevents the tab from expanding when the popover is shown
        popover.set_parent(&notebook_clone);
        
        // Convert coordinates from tab_box to notebook coordinate space
        if let Some((notebook_x, notebook_y)) = tab_box_clone.translate_coordinates(&notebook_clone, x, y) {
            let rect = gtk4::gdk::Rectangle::new(notebook_x as i32, notebook_y as i32, 1, 1);
            popover.set_pointing_to(Some(&rect));
        }
        
        // Properly handle cleanup when the popover is closed
        let popover_weak_cleanup = popover.downgrade();
        popover.connect_closed(move |_| {
            if let Some(popover) = popover_weak_cleanup.upgrade() {
                popover.unparent();
            }
        });
        
        // Show the popover
        popover.popup();
    });
    
    // Add the gesture controller to the tab box
    tab_box.add_controller(right_click_gesture);
}

/// Returns status bar components from the template
///
/// Returns a tuple of:
/// - GtkBox: Container for the status bar
/// - Label: Main status text label
/// - Label: Secondary status information (current file, line/column, etc.)
pub fn create_status_bar(window: &DvopWindow) -> (GtkBox, Label, Label) {
    let imp = window.imp();
    (
        imp.status_bar.get(),
        imp.status_label.get(),
        imp.secondary_status_label.get(),
    )
}

/// Updates the visibility of the volume control based on active tab content
pub fn update_volume_control_visibility_for_tab(volume_control_box: &GtkBox, active_tab_path: &Option<std::path::PathBuf>) {
    let is_media_tab = active_tab_path
        .as_ref()
        .map(|path| crate::audio::is_music_file(path) || crate::video::is_video_file(path))
        .unwrap_or(false);
    volume_control_box.set_visible(is_media_tab);
}

/// Creates and shows a log history popup window
/// 
/// # Arguments
/// * `parent_window` - The parent window to center the popup on
pub fn show_log_history_popup(parent_window: &impl IsA<gtk4::ApplicationWindow>) {
    use crate::status_log;
    
    // Create the dialog window as an ApplicationWindow for full window controls
    let app_window: &gtk4::ApplicationWindow = parent_window.upcast_ref();
    let dialog = gtk4::ApplicationWindow::new(app_window.application().as_ref().unwrap());
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


