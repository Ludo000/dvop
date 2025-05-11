use gtk4::prelude::*;
use gtk4::{Button, ListBox, ScrolledWindow};
use std::rc::Rc;
use std::cell::RefCell;
use std::path::PathBuf;
use mime_guess;
use mime_guess::Mime;

pub fn is_allowed_mime_type(mime_type: &Mime) -> bool {
    mime_type.type_() == "text" ||
    mime_type == &mime_guess::mime::APPLICATION_OCTET_STREAM ||
    mime_type == &mime_guess::mime::APPLICATION_JSON ||
    mime_type == &mime_guess::mime::APPLICATION_JAVASCRIPT ||
    mime_type.type_().as_str().starts_with("text/") ||
    mime_type.essence_str() == "application/xml" ||
    mime_type.essence_str() == "application/x-httpd-php" ||
    mime_type.essence_str() == "application/x-mspublisher"
}

pub fn update_file_list(file_list_box: &ListBox, current_dir: &PathBuf, file_path: &Option<PathBuf>) {
    while let Some(child) = file_list_box.first_child() {
        file_list_box.remove(&child);
    }

    file_list_box.unselect_all();

    let mut folders = Vec::new();
    let mut files = Vec::new();

    if let Ok(entries) = std::fs::read_dir(current_dir) {
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
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&file_name_str));
        label.set_halign(gtk4::Align::Start);
        label.set_margin_start(5);
        label.set_markup(&format!("<span weight=\"bold\">{}</span>", file_name_str));
        row.set_child(Some(&label));
        file_list_box.append(&row);
    }

    let mut selected_row = None;
    for (file_name_str, _entry) in files {
        let row = gtk4::ListBoxRow::new();
        let label = gtk4::Label::new(Some(&file_name_str));
        label.set_halign(gtk4::Align::Start);
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

pub fn update_save_buttons_visibility(save_button: &Button, save_as_button: &Button, mime_type: Option<mime_guess::Mime>) {
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
