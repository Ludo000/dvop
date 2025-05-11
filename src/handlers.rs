use gtk4::prelude::*;
use gtk4::{Button, TextBuffer, ApplicationWindow, ListBox, ScrolledWindow, TextView, Label, Picture};
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use crate::utils;
use std::fs::File;
use std::io::Write;

pub fn setup_button_handlers(
    new_button: &Button,
    open_button: &Button,
    save_button: &Button,
    save_as_button: &Button,
    text_buffer: &TextBuffer,
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
    text_buffer: &TextBuffer,
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
        utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
        utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, None);
    });
}

fn setup_open_button_handler(
    open_button: &Button,
    text_buffer: &TextBuffer,
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
        let dialog = gtk4::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk4::FileChooserAction::Open,
            &[("Open", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
        );

        let current_folder = gtk4::gio::File::for_path(&*current_dir.borrow());
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
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file).first_or_octet_stream();
                    if utils::is_allowed_mime_type(&mime_type) {
                        if let Ok(content) = std::fs::read_to_string(&file) {
                            text_buffer.set_text(&content);
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));

                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                            utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                        }
                    } else if mime_type.type_() == "image" {
                        if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&file) {
                            picture.set_pixbuf(Some(&pixbuf));
                            scrolled_window.set_child(Some(&picture));

                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
                            }
                            utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
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
    text_buffer: &TextBuffer,
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
            if utils::is_allowed_mime_type(&mime_type) {
                if let Ok(mut file) = File::create(path) {
                    let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                    let _ = file.write_all(text.as_bytes());
                }
            }
        } else {
            let dialog = gtk4::FileChooserDialog::new(
                Some("Save File"),
                Some(&window),
                gtk4::FileChooserAction::Save,
                &[("Save", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
            );

            let text_buffer = text_buffer.clone();
            let file_path = file_path.clone();
            let file_list_box = file_list_box.clone();
            let current_dir = current_dir.clone();
            let scrolled_window = scrolled_window.clone();
            let text_view = text_view.clone();
            dialog.connect_response(move |dialog, response| {
                if response == gtk4::ResponseType::Accept {
                    if let Some(file) = dialog.file().and_then(|f| f.path()) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));
                            utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
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
    text_buffer: &TextBuffer,
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
        let dialog = gtk4::FileChooserDialog::new(
            Some("Save File As"),
            Some(&window),
            gtk4::FileChooserAction::Save,
            &[("Save As", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
        );

        let current_folder = gtk4::gio::File::for_path(&*current_dir.borrow());
        let _ = dialog.set_current_folder(Some(&current_folder));

        let text_buffer = text_buffer.clone();
        let file_path = file_path.clone();
        let current_dir = current_dir.clone();
        let file_list_box = file_list_box.clone();
        let scrolled_window = scrolled_window.clone();
        let text_view = text_view.clone();
        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file).first_or_octet_stream();
                    if utils::is_allowed_mime_type(&mime_type) {
                        if let Ok(mut f) = File::create(&file) {
                            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
                            let _ = f.write_all(text.as_bytes());
                            *file_path.borrow_mut() = Some(file.clone());
                            scrolled_window.set_child(Some(&text_view));

                            if let Some(parent) = file.parent() {
                                *current_dir.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
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
    text_buffer: &TextBuffer,
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
                utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
            } else if path.is_file() {
                let mime_type = mime_guess::from_path(&path).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        text_buffer.set_text(&content);
                        *file_path.borrow_mut() = Some(path);
                        scrolled_window.set_child(Some(&text_view));
                        utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                        utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                    }
                } else if mime_type.type_() == "image" {
                    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&path) {
                        picture.set_pixbuf(Some(&pixbuf));
                        scrolled_window.set_child(Some(&picture));
                        *file_path.borrow_mut() = Some(path);
                        utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
                        utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
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
            utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &file_path.borrow());
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
        utils::update_file_list(&file_list_box, &current_dir.borrow(), &file_path.borrow());
    });
}
