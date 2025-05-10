use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, Box as GtkBox, FileChooserAction, FileChooserDialog,
    HeaderBar, Orientation, ResponseType, ScrolledWindow, TextView, ListBox, ListBoxRow, Label, Align,
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

fn main() {
    let app = Application::builder()
        .application_id("com.example.BasadoTextEditor")
        .build();

    app.connect_activate(build_ui);
    app.run();
}

fn build_ui(app: &Application) {
    let window = ApplicationWindow::builder()
        .application(app)
        .default_width(800)
        .default_height(600)
        .title("Basado Text Editor")
        .build();

    let header = HeaderBar::builder()
        .show_title_buttons(true)
        .build();

    let new_button = Button::with_label("New");
    let open_button = Button::with_label("Open");
    let save_as_button = Button::with_label("Save As");
    let save_button = Button::with_label("Save");

    header.pack_start(&new_button);
    header.pack_start(&open_button);
    header.pack_end(&save_as_button);
    header.pack_end(&save_button);

    window.set_titlebar(Some(&header));

    let text_view = TextView::new();
    let text_buffer = text_view.buffer().clone();
    let file_path = Rc::new(RefCell::new(None));

    // Set the initial current_dir to the home directory
    let home_dir = home::home_dir().expect("Could not find home directory");
    let current_dir = Rc::new(RefCell::new(home_dir));

    let scrolled_window = ScrolledWindow::builder()
        .child(&text_view)
        .vexpand(true)
        .hexpand(true)
        .build();

    // Terminal
    let terminal = VteTerminal::new();
    if let Some(shell) = env::var("SHELL").ok() {
        terminal.spawn_async(
            vte4::PtyFlags::DEFAULT,
            None,
            &[&shell],
            &[],                 // empty env
            glib::SpawnFlags::DEFAULT,
            || {},               // empty child_setup closure
            -1,
            None::<&Cancellable>,
            move |res| {
                if let Err(err) = res {
                    eprintln!("Failed to spawn shell: {}", err);
                }
            },
        );
    }

    let terminal_box = ScrolledWindow::builder()
        .child(&terminal)
        .vexpand(false)
        .hexpand(true)
        .min_content_height(150)
        .build();

    // File Manager Panel
    let file_list_box = ListBox::new();
    let file_list_scrolled_window = ScrolledWindow::builder()
        .child(&file_list_box)
        .vexpand(true)
        .hexpand(false)
        .min_content_width(200)
        .build();

    // Navigation buttons
    let nav_box = GtkBox::new(Orientation::Horizontal, 5);

    // Create a box to add margin around the button
    let up_button_box = GtkBox::new(Orientation::Horizontal, 0);
    up_button_box.set_margin_top(5);
    up_button_box.set_margin_bottom(5);
    up_button_box.set_margin_start(5);
    up_button_box.set_margin_end(5);

    let up_button = Button::with_label("../");
    up_button_box.append(&up_button);
    nav_box.append(&up_button_box);

    let file_manager_panel = GtkBox::new(Orientation::Vertical, 5);
    file_manager_panel.append(&nav_box);
    file_manager_panel.append(&file_list_scrolled_window);

    update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());

    let paned = gtk4::Paned::new(Orientation::Horizontal);
    paned.set_wide_handle(true); // Optional: easier to grab

    // Left: File Manager Panel
    paned.set_start_child(Some(&file_manager_panel));
    // Right: Text Editor and Terminal
    let editor_paned = gtk4::Paned::new(Orientation::Vertical);
    editor_paned.set_wide_handle(true); // Optional: easier to grab
    editor_paned.set_start_child(Some(&scrolled_window));
    editor_paned.set_end_child(Some(&terminal_box));
    paned.set_end_child(Some(&editor_paned));

    // Set initial position (optional)
    paned.set_position(200);
    editor_paned.set_position(400);

    window.set_child(Some(&paned));

    // New
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let file_list_box = file_list_box.clone();
        let current_dir = current_dir.clone();
        new_button.connect_clicked(move |_| {
            text_buffer.set_text("");
            *file_path.borrow_mut() = None;
            update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
        });
    }

    // Open
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let window = window.clone();
        let current_dir = current_dir.clone();
        let file_list_box = file_list_box.clone();
        open_button.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Open File"),
                Some(&window),
                FileChooserAction::Open,
                &[("Open", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            // Convert PathBuf to gio::File and set the current folder of the dialog
            let current_folder = gio::File::for_path(&*current_dir.borrow());
            let _ = dialog.set_current_folder(Some(&current_folder)); // Ignore the Result

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            let current_dir = current_dir.clone();
            let file_list_box = file_list_box.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(content) = std::fs::read_to_string(&file) {
                            text_buffer.set_text(&content);
                            *file_path.borrow_mut() = Some(file.clone());

                            // Update the current directory to the directory of the opened file
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
        });
    }

    // Save
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let window = window.clone();
        let file_list_box = file_list_box.clone();
        let current_dir = current_dir.clone();
        save_button.connect_clicked(move |_| {
            if let Some(ref path) = *file_path.borrow() {
                if let Ok(mut file) = File::create(path) {
                    let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                    let _ = file.write_all(text.as_bytes());
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
                dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Accept {
                        if let Some(file) = dialog.file().and_then(|f| f.path()) {
                            if let Ok(mut f) = File::create(&file) {
                                let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                                let _ = f.write_all(text.as_bytes());
                                *file_path.borrow_mut() = Some(file.clone());
                                update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                        }
                    }
                    dialog.close();
                });

                dialog.show();
            }
        });
    }

    // Save As
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let window = window.clone();
        let current_dir = current_dir.clone();
        let file_list_box = file_list_box.clone();
        save_as_button.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Save File As"),
                Some(&window),
                FileChooserAction::Save,
                &[("Save As", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            // Convert PathBuf to gio::File and set the current folder of the dialog
            let current_folder = gio::File::for_path(&*current_dir.borrow());
            let _ = dialog.set_current_folder(Some(&current_folder)); // Ignore the Result

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            let current_dir = current_dir.clone();
            let file_list_box = file_list_box.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file.clone());

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
        });
    }

    // File selection in the file manager panel
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let current_dir = current_dir.clone();
        let file_list_box_clone = file_list_box.clone();
        file_list_box_clone.clone().connect_row_activated(move |_, row| {
            if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
                let file_name = label.text();
                let mut path = current_dir.borrow().clone();
                path.push(&file_name);

                if path.is_dir() {
                    *current_dir.borrow_mut() = path;
                    update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                } else if path.is_file() {
                    if let Ok(content) = fs::read_to_string(&path) {
                        text_buffer.set_text(&content);
                        *file_path.borrow_mut() = Some(path);
                        update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                    }
                }
            }
        });
    }

    // Navigation to parent directory
    {
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

    window.show();
}

fn update_file_list(file_list_box: &ListBox, current_dir: &PathBuf, file_path: &Option<PathBuf>) {
    // Clear the current list
    while let Some(child) = file_list_box.first_child() {
        file_list_box.remove(&child);
    }

    let mut folders = Vec::new();
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(current_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name();
                let file_name_str = file_name.to_string_lossy();

                // Skip dot files
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

    // Sort folders and files
    folders.sort_by(|a, b| a.0.cmp(&b.0));
    files.sort_by(|a, b| a.0.cmp(&b.0));

    // Add folders to the list
    for (file_name_str, _entry) in folders {
        let row = ListBoxRow::new();
        let label = Label::new(Some(&file_name_str));
        label.set_halign(Align::Start);
        label.set_margin_start(5);
        label.set_markup(&format!("<span weight=\"bold\">{}</span>", file_name_str));
        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    // Add files to the list
    let mut selected_row = None;
    for (file_name_str, _entry) in files {
        let row = ListBoxRow::new();
        let label = Label::new(Some(&file_name_str));
        label.set_halign(Align::Start);
        label.set_margin_start(5);

        // Check if this file is the currently opened file
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

    // Set the selected row
    if let Some(row) = selected_row {
        file_list_box.select_row(Some(&row));
    }
}
