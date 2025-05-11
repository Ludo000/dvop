use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, Box as GtkBox, FileChooserAction, FileChooserDialog,
    HeaderBar, Image, Orientation, ResponseType, ScrolledWindow, TextView, ListBox, ListBoxRow, Label, Align, Picture,
};
use std::cell::RefCell;
use std::fs::{self, File};
use std::io::Write;
use std::rc::Rc;
use std::env;
use std::path::PathBuf;
use gtk4::gio::{self, Cancellable};
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;
use home;
use mime_guess;
use mime_guess::Mime;

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn is_allowed_mime_type(mime_type: &Mime) -> bool {
    mime_type.type_() == "text" ||
    mime_type == &mime_guess::mime::APPLICATION_OCTET_STREAM ||
    mime_type == &mime_guess::mime::APPLICATION_JSON ||
    mime_type == &mime_guess::mime::APPLICATION_JAVASCRIPT ||
    mime_type.type_().as_str().starts_with("text/") ||
    mime_type.essence_str() == "application/xml" ||
    mime_type.essence_str() == "application/x-httpd-php" ||
    mime_type.essence_str() == "application/x-mspublisher"
}

fn build_ui(app: &Application) {
    let window = create_window(app);
    let (header, new_button, open_button, save_button, save_as_button) = create_header();
    let (text_view, text_buffer, file_path, error_label, picture, current_dir, scrolled_window) = create_text_view();
    let terminal = create_terminal();
    let terminal_box = create_terminal_box(&terminal);
    let (file_list_box, file_list_scrolled_window, nav_box, up_button, refresh_button) = create_file_manager_panel();
    let file_manager_panel = create_file_manager_panel_container(nav_box, file_list_scrolled_window);
    let paned = create_paned(&file_manager_panel, &scrolled_window, &terminal_box);

    // Set the header bar as the title bar of the window
    window.set_titlebar(Some(&header));

    update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());

    window.set_child(Some(&paned));

    setup_button_handlers(
        &new_button, &open_button, &save_button, &save_as_button,
        &text_buffer, &file_path, &window, &current_dir, &file_list_box,
        &scrolled_window, &text_view, &error_label, &picture,
        &up_button, &refresh_button, &file_list_box
    );

    window.show();
}

fn create_window(app: &Application) -> ApplicationWindow {
    ApplicationWindow::builder()
        .application(app)
        .default_width(800)
        .default_height(600)
        .title("Basado Text Editor")
        .build()
}

fn create_header() -> (HeaderBar, Button, Button, Button, Button) {
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

fn create_button_box(icon_name: &str, label_text: &str, tooltip_text: &str) -> GtkBox {
    let button_box = GtkBox::new(Orientation::Horizontal, 5);
    let icon = Image::from_icon_name(icon_name);
    let label = Label::new(Some(label_text));
    button_box.append(&icon);
    button_box.append(&label);
    button_box.set_tooltip_text(Some(tooltip_text));
    button_box
}

fn create_text_view() -> (TextView, gtk4::TextBuffer, Rc<RefCell<Option<PathBuf>>>, Label, Picture, Rc<RefCell<PathBuf>>, ScrolledWindow) {
    let text_view = TextView::new();
    let text_buffer = text_view.buffer().clone();
    let file_path = Rc::new(RefCell::new(None));
    let error_label = Label::new(Some("Cannot open this file type."));
    error_label.set_halign(Align::Center);
    error_label.set_valign(Align::Center);
    let picture = Picture::new();
    let home_dir = home::home_dir().expect("Could not find home directory");
    let current_dir = Rc::new(RefCell::new(home_dir));
    let scrolled_window = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&text_view)
        .build();

    (text_view, text_buffer, file_path, error_label, picture, current_dir, scrolled_window)
}

fn create_terminal() -> VteTerminal {
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

fn create_terminal_box(terminal: &VteTerminal) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(terminal)
        .vexpand(false)
        .hexpand(true)
        .min_content_height(150)
        .build()
}

fn create_file_manager_panel() -> (ListBox, ScrolledWindow, GtkBox, Button, Button) {
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

fn create_file_manager_panel_container(nav_box: GtkBox, file_list_scrolled_window: ScrolledWindow) -> GtkBox {
    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    file_manager_panel.append(&nav_box);
    file_manager_panel.append(&file_list_scrolled_window);
    file_manager_panel
}

fn create_paned(
    file_manager_panel: &GtkBox,
    scrolled_window: &ScrolledWindow,
    terminal_box: &ScrolledWindow
) -> gtk4::Paned {
    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true);

    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true);
    editor_paned.set_start_child(Some(scrolled_window));
    editor_paned.set_end_child(Some(terminal_box));

    paned.set_start_child(Some(file_manager_panel));
    paned.set_end_child(Some(&editor_paned));

    paned.set_position(200);
    editor_paned.set_position(400);

    paned
}

fn setup_button_handlers(
    new_button: &Button,
    open_button: &Button,
    save_button: &Button,
    save_as_button: &Button,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView,
    error_label: &Label,
    picture: &Picture,
    up_button: &Button,
    refresh_button: &Button,
    file_list_box_clone: &ListBox
) {
    setup_new_button_handler(
        new_button, text_buffer, file_path, file_list_box, current_dir,
        scrolled_window, text_view, save_button, save_as_button
    );

    setup_open_button_handler(
        open_button, text_buffer, file_path, window, current_dir, file_list_box,
        scrolled_window, text_view, error_label, picture, save_button, save_as_button
    );

    setup_save_button_handler(
        save_button, text_buffer, file_path, window, file_list_box, current_dir,
        scrolled_window, text_view
    );

    setup_save_as_button_handler(
        save_as_button, text_buffer, file_path, window, current_dir, file_list_box,
        scrolled_window, text_view
    );

    setup_file_selection_handler(
        file_list_box_clone, text_buffer, file_path, current_dir, scrolled_window,
        text_view, error_label, picture, save_button, save_as_button
    );

    setup_up_button_handler(up_button, current_dir, file_list_box, file_path);

    setup_refresh_button_handler(refresh_button, file_list_box, current_dir, file_path);
}

fn setup_new_button_handler(
    new_button: &Button,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView,
    save_button: &Button,
    save_as_button: &Button
) {
    let text_buffer = text_buffer.clone();
    let file_path = file_path.clone();
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let scrolled_window = scrolled_window.clone();
    let text_view = text_view.clone();
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    new_button.connect_clicked(move |_| {
        text_buffer.set_text("");
        *file_path.borrow_mut() = None;
        scrolled_window.set_child(Some(&text_view));
        update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
        update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, None);
    });
}

fn setup_open_button_handler(
    open_button: &Button,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView,
    error_label: &Label,
    picture: &Picture,
    save_button: &Button,
    save_as_button: &Button
) {
    let text_buffer = text_buffer.clone();
    let file_path = file_path.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();
    let scrolled_window = scrolled_window.clone();
    let text_view = text_view.clone();
    let error_label = error_label.clone();
    let picture = picture.clone();
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    open_button.connect_clicked(move |_| {
        let dialog = FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            FileChooserAction::Open,
            &[("Open", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        let current_folder = gio::File::for_path(&*current_dir.borrow());
        let _ = dialog.set_current_folder(Some(&current_folder));

        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let current_dir = current_dir.clone();
        let file_list_box = file_list_box.clone();
        let scrolled_window = scrolled_window.clone();
        let text_view = text_view.clone();
        let error_label = error_label.clone();
        let picture = picture.clone();
        let save_button_clone = save_button_clone.clone();
        let save_as_button_clone = save_as_button_clone.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file).first_or_octet_stream();
                    if is_allowed_mime_type(&mime_type) {
                        if let Ok(content) = std::fs::read_to_string(&file) {
                            text_buffer.set_text(&content);
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));

                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                            update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                        }
                    } else if mime_type.type_() == "image" {
                        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&file) {
                            picture.set_pixbuf(Some(&pixbuf));
                            scrolled_window.set_child(Some(&picture));

                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                            update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                        }
                    } else {
                        scrolled_window.set_child(Some(&error_label));
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });
}

fn setup_save_button_handler(
    save_button: &Button,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView
) {
    let text_buffer = text_buffer.clone();
    let file_path = file_path.clone();
    let window = window.clone();
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let scrolled_window = scrolled_window.clone();
    let text_view = text_view.clone();
    save_button.connect_clicked(move |_| {
        if let Some(ref path) = *file_path.borrow() {
            let mime_type = mime_guess::from_path(path).first_or_octet_stream();
            if is_allowed_mime_type(&mime_type) {
                if let Ok(mut file) = File::create(path) {
                    let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                    let _ = file.write_all(text.as_bytes());
                }
            }
        } else {
            let dialog = FileChooserDialog::new(
                Some("Save File"),
                Some(&window),
                FileChooserAction::Save,
                &[("Save", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            let file_list_box = file_list_box.clone();
            let current_dir = current_dir.clone();
            let scrolled_window = scrolled_window.clone();
            let text_view = text_view.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));

                            // Update the current directory to the directory of the saved file
                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                        }
                    }
                }
                dialog.close();
            });

            dialog.show();
        }
    });
}

fn setup_save_as_button_handler(
    save_as_button: &Button,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView
) {
    let text_buffer = text_buffer.clone();
    let file_path = file_path.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();
    let scrolled_window = scrolled_window.clone();
    let text_view = text_view.clone();
    save_as_button.connect_clicked(move |_| {
        let dialog = FileChooserDialog::new(
            Some("Save File As"),
            Some(&window),
            FileChooserAction::Save,
            &[("Save As", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
        );

        let current_folder = gio::File::for_path(&*current_dir.borrow());
        let _ = dialog.set_current_folder(Some(&current_folder));

        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let current_dir = current_dir.clone();
        let file_list_box = file_list_box.clone();
        let scrolled_window = scrolled_window.clone();
        let text_view = text_view.clone();
        dialog.connect_response(move |dialog, response| {
            if response == ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file).first_or_octet_stream();
                    if is_allowed_mime_type(&mime_type) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));

                            // Update the current directory to the directory of the saved file
                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                        }
                    }
                }
            }
            dialog.close();
        });

        dialog.show();
    });
}

fn setup_file_selection_handler(
    file_list_box: &ListBox,
    text_buffer: &gtk4::TextBuffer,
    file_path: &Rc<RefCell<Option<PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>,
    scrolled_window: &ScrolledWindow,
    text_view: &TextView,
    error_label: &Label,
    picture: &Picture,
    save_button: &Button,
    save_as_button: &Button
) {
    let text_buffer = text_buffer.clone();
    let file_path = file_path.clone();
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let scrolled_window = scrolled_window.clone();
    let text_view = text_view.clone();
    let error_label = error_label.clone();
    let picture = picture.clone();
    let save_button_clone = save_button.clone();
    let save_as_button_clone = save_as_button.clone();
    file_list_box.connect_row_activated(move |_, row| {
        if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
            let file_name = label.text();
            let mut path = current_dir.borrow().clone();
            path.push(&file_name);

            if path.is_dir() {
                *current_dir.borrow_mut() = path;
                update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
            } else if path.is_file() {
                let mime_type = mime_guess::from_path(&path).first_or_octet_stream();
                if is_allowed_mime_type(&mime_type) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        text_buffer.set_text(&content);
                        *file_path.borrow_mut() = Some(path);
                        scrolled_window.set_child(Some(&text_view));
                        update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                        update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                    }
                } else if mime_type.type_() == "image" {
                    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&path) {
                        picture.set_pixbuf(Some(&pixbuf));
                        scrolled_window.set_child(Some(&picture));
                        *file_path.borrow_mut() = Some(path);
                        update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                        update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                    }
                } else {
                    scrolled_window.set_child(Some(&error_label));
                }
            }
        }
    });
}

fn setup_up_button_handler(
    up_button: &Button,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    file_path: &Rc<RefCell<Option<PathBuf>>>
) {
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let file_path = file_path.clone();
    up_button.connect_clicked(move |_| {
        let mut path = current_dir.borrow().clone();
        if path.pop() {
            *current_dir.borrow_mut() = path;
            update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
        }
    });
}

fn setup_refresh_button_handler(
    refresh_button: &Button,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_path: &Rc<RefCell<Option<PathBuf>>>
) {
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let file_path = file_path.clone();
    refresh_button.connect_clicked(move |_| {
        update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
    });
}

fn update_file_list(file_list_box: &ListBox, current_dir: &PathBuf, file_path: &Option<PathBuf>) {
    while let Some(child) = file_list_box.first_child() {
        file_list_box.remove(&child);
    }

    file_list_box.unselect_all();

    let mut folders = Vec::new();
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                if file_name_str.starts_with('.') {
                    continue;
                }

                if entry.file_type().map_or(false, |ft| ft.is_dir()) {
                    folders.push((file_name_str.to_string(), entry));
                } else {
                    files.push((file_name_str.to_string(), entry));
                }
            }
        }
    }

    folders.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    for (file_name_str, _entry) in folders {
        let row = ListBoxRow::new();
        let label = Label::new(Some(&file_name_str));
        label.set_halign(Align::Start);
        label.set_margin_start(5);
        label.set_markup(&format!("<span weight=\"bold\">{}</span>", file_name_str));
        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    let mut selected_row = None;
    for (file_name_str, _entry) in files {
        let row = ListBoxRow::new();
        let label = Label::new(Some(&file_name_str));
        label.set_halign(Align::Start);
        label.set_margin_start(5);

        if let Some(ref path) = file_path {
            let path = path.clone();
            if let Some(file_name) = path.file_name() {
                if file_name.to_string_lossy() == file_name_str {
                    label.set_markup(&format!("<u>{}</u>", file_name_str));
                    selected_row = Some(row.clone());
                }
            }
        }

        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    if let Some(row) = selected_row {
        file_list_box.select_row(Some(&row));
    }
}

fn update_save_buttons_visibility(save_button: &Button, save_as_button: &Button, mime_type: Option<mime_guess::Mime>) {
    match mime_type {
        Some(mime) if mime.type_() == "image" => {
            save_button.set_visible(false);
            save_as_button.set_visible(false);
        },
        _ => {
            save_button.set_visible(true);
            save_as_button.set_visible(true);
        }
    }
}
