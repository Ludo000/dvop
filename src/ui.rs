use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, Box as GtkBox,
    HeaderBar, Image, Orientation, ScrolledWindow, TextView, ListBox, Label, Align, Picture,
    Notebook, // Added Notebook
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
    let header = HeaderBar::builder()
        .show_title_buttons(true)
        .build();

    let new_button = Button::new();
    let open_button = Button::new();
    let save_as_button = Button::new();
    let save_button = Button::new();

    let new_button_box = create_button_box("document-new-symbolic", "New", "Create a new file");
    new_button.set_child(Some(&new_button_box));

    let open_button_box = create_button_box("document-open-symbolic", "Open", "Open an existing file");
    open_button.set_child(Some(&open_button_box));

    let save_button_box = create_button_box("document-save-symbolic", "Save", "Save the current file");
    save_button.set_child(Some(&save_button_box));

    let save_as_button_box = create_button_box("document-save-as-symbolic", "Save As", "Save the current file as a new file");
    save_as_button.set_child(Some(&save_as_button_box));

    header.pack_start(&new_button);
    header.pack_start(&open_button);
    header.pack_end(&save_as_button);
    header.pack_end(&save_button);

    (header, new_button, open_button, save_button, save_as_button)
}

pub fn create_button_box(icon_name: &str, label_text: &str, tooltip_text: &str) -> GtkBox {
    let button_box = GtkBox::new(Orientation::Horizontal, 5);
    let icon = Image::from_icon_name(icon_name);
    let label = Label::new(Some(label_text));
    button_box.append(&icon);
    button_box.append(&label);
    button_box.set_tooltip_text(Some(tooltip_text));
    button_box
}

pub fn create_text_view() -> (
    Notebook, // Changed from TextView
    gtk4::TextBuffer, // This will be for the initial/active tab or managed per tab
    Rc<RefCell<Option<PathBuf>>>,
    Label,
    Picture,
    Rc<RefCell<PathBuf>>,
    // ScrolledWindow, // This will be created per tab now
) {
    let notebook = Notebook::new();
    notebook.set_scrollable(true);

    // Create an initial empty tab or leave it empty until a file is opened
    let initial_text_view = TextView::new();
    let initial_text_buffer = initial_text_view.buffer().clone();
    let initial_scrolled_window = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&initial_text_view)
        .build();
    let initial_tab_label = Label::new(Some("Untitled"));
    notebook.append_page(&initial_scrolled_window, Some(&initial_tab_label));


    let file_path = Rc::new(RefCell::new(None)); // This might need to be a collection for multiple tabs
    let error_label = Label::new(Some("Cannot open this file type."));
    error_label.set_halign(Align::Center);
    error_label.set_valign(Align::Center);
    let picture = Picture::new(); // This might also need to be per-tab if images are opened in tabs
    let home_dir = home::home_dir().expect("Could not find home directory");
    let current_dir = Rc::new(RefCell::new(home_dir));

    (
        notebook,
        initial_text_buffer, // Return buffer of the first tab for now
        file_path,
        error_label,
        picture,
        current_dir,
        // scrolled_window is no longer returned directly, it's part of the notebook
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
    let file_list_scrolled_window = ScrolledWindow::builder()
        .child(&file_list_box)
        .vexpand(true)
        .hexpand(false)
        .min_content_width(200)
        .build();

    let nav_box = GtkBox::new(Orientation::Horizontal, 5);

    let up_button_box = GtkBox::new(Orientation::Horizontal, 0);
    up_button_box.set_margin_top(5);
    up_button_box.set_margin_bottom(5);
    up_button_box.set_margin_start(5);
    up_button_box.set_margin_end(5);

    let up_button = Button::new();
    let up_button_content = GtkBox::new(Orientation::Horizontal, 5);
    let up_label = Label::new(Some("../"));
    up_button_content.append(&up_label);
    up_button.set_child(Some(&up_button_content));
    up_button.set_tooltip_text(Some("Go to the parent directory"));
    up_button_box.append(&up_button);
    nav_box.append(&up_button_box);

    let spacer = GtkBox::new(Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    nav_box.append(&spacer);

    let refresh_button = Button::new();
    let refresh_button_content = GtkBox::new(Orientation::Horizontal, 5);
    let refresh_icon = Image::from_icon_name("view-refresh-symbolic");
    refresh_button_content.append(&refresh_icon);
    refresh_button.set_child(Some(&refresh_button_content));
    refresh_button.set_tooltip_text(Some("Refresh the current folder view"));
    let refresh_button_box = GtkBox::new(Orientation::Horizontal, 0);
    refresh_button_box.set_margin_top(5);
    refresh_button_box.set_margin_bottom(5);
    refresh_button_box.set_margin_start(5);
    refresh_button_box.set_margin_end(5);
    refresh_button_box.append(&refresh_button);
    nav_box.append(&refresh_button_box);

    (file_list_box, file_list_scrolled_window, nav_box, up_button, refresh_button)
}

pub fn create_file_manager_panel_container(nav_box: GtkBox, file_list_scrolled_window: ScrolledWindow) -> GtkBox {
    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    file_manager_panel.append(&nav_box);
    file_manager_panel.append(&file_list_scrolled_window);
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
