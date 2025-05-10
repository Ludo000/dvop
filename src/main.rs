use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Button, FileChooserAction, FileChooserDialog,
    HeaderBar, ResponseType, ScrolledWindow, TextView, Orientation,
};
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;
use std::env;
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;

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

    let paned = gtk4::Paned::new(Orientation::Vertical);
    paned.set_wide_handle(true); // Optional: easier to grab

    // Top: Text Editor
    paned.set_start_child(Some(&scrolled_window));
    // Bottom: Terminal
    paned.set_end_child(Some(&terminal_box));

    // Set initial position (optional)
    paned.set_position(400);

    window.set_child(Some(&paned));


    // New
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        new_button.connect_clicked(move |_| {
            text_buffer.set_text("");
            *file_path.borrow_mut() = None;
        });
    }

    // Open
    {
        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let window = window.clone();
        open_button.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Open File"),
                Some(&window),
                FileChooserAction::Open,
                &[("Open", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(content) = std::fs::read_to_string(&file) {
                            text_buffer.set_text(&content);
                            *file_path.borrow_mut() = Some(file);
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
                dialog.connect_response(move |dialog, response| {
                    if response == ResponseType::Accept {
                        if let Some(file) = dialog.file().and_then(|f| f.path()) {
                            if let Ok(mut f) = File::create(&file) {
                                let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                                let _ = f.write_all(text.as_bytes());
                                *file_path.borrow_mut() = Some(file);
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
        save_as_button.connect_clicked(move |_| {
            let dialog = FileChooserDialog::new(
                Some("Save File As"),
                Some(&window),
                FileChooserAction::Save,
                &[("Save As", ResponseType::Accept), ("Cancel", ResponseType::Cancel)],
            );

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            dialog.connect_response(move |dialog, response| {
                if response == ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file);
                        }
                    }
                }
                dialog.close();
            });

            dialog.show();
        });
    }

    window.show();
}
