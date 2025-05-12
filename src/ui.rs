use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Box as GtkBox, Button, HeaderBar, Label, ListBox, Notebook,
    Orientation, Picture, ScrolledWindow, TextView, Image, PolicyType, // Added PolicyType
};
use std::cell::RefCell;
use std::rc::Rc;
use std::env;
use std::path::PathBuf;
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;
use home;

pub fn create_window(app: &Application) -> ApplicationWindow {
    ApplicationWindow::builder()
        .application(app)
        .default_width(800)
        .default_height(600)
        .title("Basado Text Editor")
        .build()
}

pub fn create_header() -> (HeaderBar, Button, Button, Button, Button) {
    let header = HeaderBar::new();

    // New Button
    let new_button = Button::new();
    let new_button_icon = Image::from_icon_name("document-new-symbolic");
    let new_button_label = Label::new(Some("New"));
    let new_button_box = GtkBox::new(Orientation::Horizontal, 5);
    new_button_box.append(&new_button_icon);
    new_button_box.append(&new_button_label);
    new_button.set_child(Some(&new_button_box));
    new_button.set_tooltip_text(Some("Create a new file"));
    header.pack_start(&new_button);

    // Open Button
    let open_button = Button::new();
    let open_button_icon = Image::from_icon_name("document-open-symbolic");
    let open_button_label = Label::new(Some("Open"));
    let open_button_box = GtkBox::new(Orientation::Horizontal, 5);
    open_button_box.append(&open_button_icon);
    open_button_box.append(&open_button_label);
    open_button.set_child(Some(&open_button_box));
    open_button.set_tooltip_text(Some("Open a file"));
    header.pack_start(&open_button);

    // Save As Button
    let save_as_button = Button::new();
    let save_as_button_icon = Image::from_icon_name("document-save-as-symbolic");
    let save_as_button_label = Label::new(Some("Save As"));
    let save_as_button_box = GtkBox::new(Orientation::Horizontal, 5);
    save_as_button_box.append(&save_as_button_icon);
    save_as_button_box.append(&save_as_button_label);
    save_as_button.set_child(Some(&save_as_button_box));
    save_as_button.set_tooltip_text(Some("Save the current file with a new name"));
    header.pack_end(&save_as_button);

    // Save Button
    let save_button = Button::new();
    let save_button_icon = Image::from_icon_name("document-save-symbolic");
    let save_button_label = Label::new(Some("Save"));
    let save_button_box = GtkBox::new(Orientation::Horizontal, 5);
    save_button_box.append(&save_button_icon);
    save_button_box.append(&save_button_label);
    save_button.set_child(Some(&save_button_box));
    save_button.set_tooltip_text(Some("Save the current file"));
    header.pack_end(&save_button);


    (header, new_button, open_button, save_button, save_as_button)
}

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
    let editor_notebook = Notebook::new();
    editor_notebook.set_scrollable(true);

    let (tab_widget, tab_label, tab_close_button) = create_tab_widget("Untitled");
    
    let text_view = TextView::builder()
        .monospace(true)
        .editable(true)
        .cursor_visible(true)
        .build();

    let buffer = text_view.buffer();

    let scrolled_window = gtk4::ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&text_view)
        .build();

    editor_notebook.append_page(&scrolled_window, Some(&tab_widget));
    editor_notebook.set_tab_label(&scrolled_window, Some(&tab_widget));


    let file_path = Rc::new(RefCell::new(None));
    let error_label = Label::new(None);
    let picture = Picture::new();
    let current_dir = Rc::new(RefCell::new(home::home_dir().unwrap_or_else(|| PathBuf::from("/"))));

    (
        scrolled_window, // This is now the content of the first tab, not the main scrolled_window
        text_view,
        buffer,
        file_path,
        error_label,
        picture,
        current_dir,
        editor_notebook, // Return the notebook itself
        tab_widget,
        tab_label,
        tab_close_button
    )
}

pub fn create_terminal() -> VteTerminal {
    let terminal = VteTerminal::new();
    if let Some(shell) = env::var("SHELL").ok() {
        let home_dir = home::home_dir().expect("Could not find home directory");
        if let Some(home_dir_str) = home_dir.to_str() {
            terminal.spawn_async(
                vte4::PtyFlags::DEFAULT,
                Some(home_dir_str),
                &[&shell],
                &[],
                glib::SpawnFlags::DEFAULT,
                || {},
                -1,
                None::<&Cancellable>,
                move |res| {
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

pub fn create_terminal_box(terminal: &VteTerminal) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(terminal)
        .vexpand(false)
        .hexpand(true)
        .min_content_height(150)
        .build()
}

pub fn create_file_manager_panel() -> (ListBox, ScrolledWindow, GtkBox, Button, Button) {
    let file_list_box = ListBox::new();
    file_list_box.set_selection_mode(gtk4::SelectionMode::Single);

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(gtk4::PolicyType::Never) 
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .child(&file_list_box)
        .vexpand(true) 
        .build();

    let nav_box = GtkBox::new(Orientation::Horizontal, 0); 
    nav_box.set_margin_top(5); // Apply top margin to the nav_box

    let up_button_icon = Image::from_icon_name("go-up-symbolic");
    let up_button = Button::new();
    up_button.set_child(Some(&up_button_icon));
    up_button.set_margin_start(5); 

    let refresh_button_icon = Image::from_icon_name("view-refresh-symbolic");
    let refresh_button = Button::new();
    refresh_button.set_child(Some(&refresh_button_icon));
    refresh_button.set_margin_end(5);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    nav_box.append(&up_button);
    nav_box.append(&spacer); // Add spacer to push refresh_button to the right
    nav_box.append(&refresh_button);

    (file_list_box, scrolled_window, nav_box, up_button, refresh_button)
}

pub fn create_file_manager_panel_container(nav_box: GtkBox, file_list_scrolled_window: ScrolledWindow) -> GtkBox {
    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    file_manager_panel.append(&nav_box);
    file_manager_panel.append(&file_list_scrolled_window);
    file_manager_panel.set_vexpand(true); // Make the whole panel expand
    file_manager_panel
}

pub fn create_paned(
    file_manager_panel: &GtkBox,
    editor_notebook: &Notebook, // Changed from scrolled_window
    terminal_box: &ScrolledWindow
) -> gtk4::Paned {
    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);

    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true);
    editor_paned.set_start_child(Some(editor_notebook)); // Use Notebook here
    editor_paned.set_end_child(Some(terminal_box));

    paned.set_start_child(Some(file_manager_panel));
    paned.set_end_child(Some(&editor_paned));

    paned.set_position(200);
    editor_paned.set_position(400);

    paned
}

// Add a new public function to create a tab widget (Box with Label and Close Button)
pub fn create_tab_widget(tab_title: &str) -> (GtkBox, Label, Button) {
    let tab_box = GtkBox::new(Orientation::Horizontal, 5);
    let label = Label::new(Some(tab_title));
    let close_button = Button::from_icon_name("window-close-symbolic");

    tab_box.append(&label);
    tab_box.append(&close_button);

    (tab_box, label, close_button)
}
