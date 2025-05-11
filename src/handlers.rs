use gtk4::prelude::*;
use gtk4::{Button, TextBuffer, ApplicationWindow, ListBox, ScrolledWindow, TextView, Label, Picture, Notebook};
use std::collections::HashMap;
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
    _initial_text_buffer: &TextBuffer, // Prefixed with underscore as it's not directly used here
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    editor_notebook: &Notebook, // Changed from scrolled_window
    // text_view is no longer passed directly, it\'s fetched from notebook
    error_label: &Label,
    picture: &Picture, // Picture handling might need to be per-tab or removed from main editor
    up_button: &Button,
    refresh_button: &Button,
    file_list_box_clone: &ListBox,
) {
    setup_new_button_handler(
        new_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        file_list_box,
        current_dir,
        save_button,
        save_as_button,
    );

    setup_open_button_handler(
        open_button,
        editor_notebook,
        window,
        current_dir,
        file_list_box,
        error_label,
        picture, // Picture handling needs review for tabs
        save_button,
        save_as_button,
        active_tab_path,
        file_path_manager,
    );

    setup_save_button_handler(
        save_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        window,
        file_list_box,
        current_dir,
    );

    setup_save_as_button_handler(
        save_as_button,
        editor_notebook,
        active_tab_path,
        file_path_manager,
        window,
        current_dir,
        file_list_box,
    );

    setup_file_selection_handler(
        file_list_box_clone, // Assuming this is the correct ListBox instance
        editor_notebook,
        active_tab_path,
        file_path_manager,
        current_dir,
        error_label,
        picture, // Picture handling needs review
        save_button,
        save_as_button,
    );

    // These handlers likely don\'t need direct access to the editor_notebook content itself
    // but might influence which file is considered "active" if that logic is centralized.
    setup_up_button_handler(up_button, current_dir, file_list_box, active_tab_path); // Pass active_tab_path
    setup_refresh_button_handler(refresh_button, file_list_box, current_dir, active_tab_path); // Pass active_tab_path
}

// Helper to get current TextView and TextBuffer from active Notebook tab
fn get_active_text_view_and_buffer(notebook: &Notebook) -> Option<(TextView, TextBuffer)> {
    notebook.current_page().and_then(|page_num| {
        notebook.nth_page(Some(page_num)).and_then(|page_widget| {
            if let Some(scrolled_window) = page_widget.downcast_ref::<ScrolledWindow>() {
                scrolled_window.child().and_then(|child| {
                    if let Some(text_view) = child.downcast_ref::<TextView>() {
                        Some((text_view.clone(), text_view.buffer()))
                    } else {
                        None
                    }
                })
            } else {
                None
            }
        })
    })
}

// Helper function to open a file in a new tab or focus if already open
fn open_or_focus_tab(
    notebook: &Notebook,
    file_to_open: &PathBuf,
    content: &str,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    save_button: &Button, // To update visibility
    save_as_button: &Button, // To update visibility
    mime_type: &mime_guess::Mime, // To update save buttons
) {
    // Check if file is already open
    let mut page_to_focus = None;
    let num_pages = notebook.n_pages();
    for i in 0..num_pages {
        if let Some(path) = file_path_manager.borrow().get(&i) {
            if path == file_to_open {
                page_to_focus = Some(i);
                break;
            }
        }
    }

    if let Some(page_num) = page_to_focus {
        notebook.set_current_page(Some(page_num));
        *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());
    } else {
        // Create new tab
        let new_text_view = TextView::new();
        let new_text_buffer = new_text_view.buffer();
        new_text_buffer.set_text(content);

        let new_scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&new_text_view)
            .build();

        let file_name = file_to_open.file_name().unwrap_or_default().to_string_lossy().to_string();
        let tab_label = Label::new(Some(&file_name));

        let new_page_num = notebook.append_page(&new_scrolled_window, Some(&tab_label));
        notebook.set_current_page(Some(new_page_num));

        file_path_manager.borrow_mut().insert(new_page_num, file_to_open.clone());
        *active_tab_path_ref.borrow_mut() = Some(file_to_open.clone());

        // Mark the TextView with its path for future reference (optional, if file_path_manager is robust)
        // new_text_view.set_data("file-path", file_to_open.clone());

        // TODO: Connect buffer changed signal to update tab_label with "*"
        let tab_label_clone = tab_label.clone();
        let file_name_clone = file_name.clone();
        new_text_buffer.connect_changed(move |_buffer| { // Prefixed buffer with underscore
            // Basic dirty tracking:
            // We need a more robust way to track if it's truly "dirty" vs just changed from initial load
            // For now, just add/remove asterisk
            if !tab_label_clone.text().ends_with("*") {
                 tab_label_clone.set_text(&format!("{}*", file_name_clone));
            }
        });
    }
    utils::update_save_buttons_visibility(save_button, save_as_button, Some(mime_type.clone()));
}


fn setup_new_button_handler(
    new_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    file_list_box: &ListBox, // To update file list selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list
    save_button: &Button,
    save_as_button: &Button,
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone(); // Clone Rc for the closure
    let _file_path_manager = file_path_manager.clone(); // Clone Rc, mark unused for now
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let save_button = save_button.clone();
    let save_as_button = save_as_button.clone();

    new_button.connect_clicked(move |_| {
        let new_text_view = TextView::new();
        let new_text_buffer = new_text_view.buffer();
        new_text_buffer.set_text(""); // Empty content for new file

        let new_scrolled_window = ScrolledWindow::builder()
            .vexpand(true)
            .hexpand(true)
            .child(&new_text_view)
            .build();

        let tab_label_text = "Untitled";
        let tab_label = Label::new(Some(tab_label_text));

        let new_page_num = editor_notebook.append_page(&new_scrolled_window, Some(&tab_label));
        editor_notebook.set_current_page(Some(new_page_num));

        *active_tab_path_ref.borrow_mut() = None; // No path for new, unsaved file
        // file_path_manager.borrow_mut().insert(new_page_num, PathBuf::new()); // Or some placeholder for untitled

        // Update file list (optional, as no file is selected)
        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path_ref.borrow());
        utils::update_save_buttons_visibility(&save_button, &save_as_button, None); // New file is not text/image initially

        // Connect changed signal for dirty tracking
        let tab_label_clone = tab_label.clone();
        new_text_buffer.connect_changed(move |_buffer| {
            if !tab_label_clone.text().ends_with("*") && tab_label_clone.text() == tab_label_text {
                 tab_label_clone.set_text(&format!("{}*", tab_label_text));
            }
        });
    });
}

fn setup_open_button_handler(
    open_button: &Button,
    editor_notebook: &Notebook,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    error_label: &Label, // For showing errors if a tab can't display content
    picture: &Picture,   // For image files - this needs to be rethought for tabs
    save_button: &Button,
    save_as_button: &Button,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
) {
    let editor_notebook = editor_notebook.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();
    let error_label = error_label.clone();
    let picture = picture.clone();
    let save_button = save_button.clone();
    let save_as_button = save_as_button.clone();
    // Clone the Rc itself, not the reference, to move ownership into the closure
    let active_tab_path_ref_owned = active_tab_path_ref.clone();
    let file_path_manager_owned = file_path_manager.clone();

    open_button.connect_clicked(move |_| {
        let dialog = gtk4::FileChooserDialog::new(
            Some("Open File"),
            Some(&window),
            gtk4::FileChooserAction::Open,
            &[("Open", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
        );

        let current_dialog_dir = current_dir.borrow().clone();
        if let Some(gio_file) = gtk4::gio::File::for_path(&current_dialog_dir).parent() {
             let _ = dialog.set_current_folder(Some(&gio_file)); // Handle Result
        }

        let editor_notebook_clone = editor_notebook.clone();
        let current_dir_clone = current_dir.clone();
        let file_list_box_clone = file_list_box.clone();
        let error_label_clone = error_label.clone();
        let picture_clone = picture.clone();
        let save_button_clone = save_button.clone();
        let save_as_button_clone = save_as_button.clone();
        // Use the owned Rcs for the nested closure
        let active_tab_path_ref_for_response = active_tab_path_ref_owned.clone();
        let file_path_manager_for_response = file_path_manager_owned.clone();

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                if let Some(file_to_open) = dialog.file().and_then(|f| f.path()) {
                    let mime_type = mime_guess::from_path(&file_to_open).first_or_octet_stream();
                    if utils::is_allowed_mime_type(&mime_type) {
                        if let Ok(content) = std::fs::read_to_string(&file_to_open) {
                            open_or_focus_tab(
                                &editor_notebook_clone,
                                &file_to_open,
                                &content,
                                &active_tab_path_ref_for_response, // Use owned Rc
                                &file_path_manager_for_response,   // Use owned Rc
                                &save_button_clone,
                                &save_as_button_clone,
                                &mime_type,
                            );

                            if let Some(parent) = file_to_open.parent() {
                                *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_for_response.borrow());
                            }
                        }
                    } else if mime_type.type_() == "image" {
                        // Image handling in tabs:
                        // Option 1: Open in a new tab with a Picture widget
                        // Option 2: Open with external viewer
                        // For now, let\'s use the existing picture widget if no text tabs are open,
                        // or show an error. This part needs a dedicated design.
                        if editor_notebook_clone.n_pages() == 1 { // Assuming 1 initial empty tab
                             if let Some(page_widget) = editor_notebook_clone.nth_page(Some(0)){
                                if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                                    if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&file_to_open) {
                                        picture_clone.set_pixbuf(Some(&pixbuf));
                                        sw.set_child(Some(&picture_clone)); // This replaces the textview in the first tab
                                        *active_tab_path_ref_for_response.borrow_mut() = Some(file_to_open.clone());
                                         file_path_manager_for_response.borrow_mut().insert(0, file_to_open.clone());


                                        if let Some(parent) = file_to_open.parent() {
                                            *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                        }
                                        utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_for_response.borrow());
                                        utils::update_save_buttons_visibility(&save_button_clone, &save_as_button_clone, Some(mime_type));
                                    }
                                }
                             }
                        } else {
                            // Can\'t open image in a new tab easily with current setup
                            if let Some(page_widget) = editor_notebook_clone.current_page().and_then(|p| editor_notebook_clone.nth_page(Some(p))) {
                                if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                                    sw.set_child(Some(&error_label_clone));
                                }
                            }
                        }
                    } else {
                         if let Some(page_widget) = editor_notebook_clone.current_page().and_then(|p| editor_notebook_clone.nth_page(Some(p))) {
                            if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                                sw.set_child(Some(&error_label_clone));
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

fn setup_save_button_handler(
    save_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    window: &ApplicationWindow,
    file_list_box: &ListBox, // To update selection
    current_dir: &Rc<RefCell<PathBuf>>, // To update file list path
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();

    save_button.connect_clicked(move |_| {
        if let Some((_active_text_view, active_buffer)) = get_active_text_view_and_buffer(&editor_notebook) { // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            let path_to_save_opt = file_path_manager.borrow().get(&current_page_num).cloned();

            if let Some(path_to_save) = path_to_save_opt {
                let mime_type = mime_guess::from_path(&path_to_save).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(mut file) = File::create(&path_to_save) {
                        let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                        if file.write_all(text.as_bytes()).is_ok() {
                            // Update tab label (remove *)
                            if let Some(page) = editor_notebook.nth_page(Some(current_page_num)) {
                                if let Some(tab_label_widget) = editor_notebook.tab_label(&page) {
                                    if let Some(label) = tab_label_widget.downcast_ref::<Label>() {
                                        let current_text = label.text();
                                        if current_text.ends_with("*") {
                                            label.set_text(&current_text[..current_text.len()-1]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            } else { // No path associated, treat as "Save As"
                // This logic should ideally call a shared "save_as" function
                let dialog = gtk4::FileChooserDialog::new(
                    Some("Save File"),
                    Some(&window),
                    gtk4::FileChooserAction::Save,
                    &[("Save", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
                );
                // ... (rest of save_as logic, simplified here) ...
                let editor_notebook_clone = editor_notebook.clone();
                let active_tab_path_ref_clone = active_tab_path_ref.clone();
                let file_path_manager_clone = file_path_manager.clone();
                let file_list_box_clone = file_list_box.clone();
                let current_dir_clone = current_dir.clone();

                dialog.connect_response(move |d, resp| {
                    if resp == gtk4::ResponseType::Accept {
                        if let Some(file) = d.file().and_then(|f| f.path()) {
                             if let Ok(mut f_obj) = File::create(&file) {
                                let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                                if f_obj.write_all(text.as_bytes()).is_ok() {
                                    file_path_manager_clone.borrow_mut().insert(current_page_num, file.clone());
                                    *active_tab_path_ref_clone.borrow_mut() = Some(file.clone());
                                     // Update tab label
                                    if let Some(page) = editor_notebook_clone.nth_page(Some(current_page_num)) {
                                        if let Some(tab_label_widget) = editor_notebook_clone.tab_label(&page) {
                                            if let Some(label) = tab_label_widget.downcast_ref::<Label>() {
                                                label.set_text(&file.file_name().unwrap_or_default().to_string_lossy());
                                            }
                                        }
                                        // Update main window title potentially
                                    }
                                    if let Some(parent) = file.parent() {
                                        *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                    }
                                    utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_clone.borrow());
                                }
                            }
                        }
                    }
                    d.close();
                });
                dialog.show();
            }
        }
    });
}

fn setup_save_as_button_handler(
    save_as_button: &Button,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    window: &ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>, // To set initial dialog directory and update after save
    file_list_box: &ListBox, // To update file list
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let window = window.clone();
    let current_dir = current_dir.clone();
    let file_list_box = file_list_box.clone();

    save_as_button.connect_clicked(move |_| {
        if let Some((_active_text_view, active_buffer)) = get_active_text_view_and_buffer(&editor_notebook) { // Prefixed active_text_view
            let current_page_num_opt = editor_notebook.current_page();
            if current_page_num_opt.is_none() { return; }
            let current_page_num = current_page_num_opt.unwrap();

            let dialog = gtk4::FileChooserDialog::new(
                Some("Save File As"),
                Some(&window),
                gtk4::FileChooserAction::Save,
                &[("Save As", gtk4::ResponseType::Accept), ("Cancel", gtk4::ResponseType::Cancel)],
            );

            let current_dialog_dir = current_dir.borrow().clone();
             if let Some(gio_file) = gtk4::gio::File::for_path(&current_dialog_dir).parent() {
                let _ = dialog.set_current_folder(Some(&gio_file)); // Handle Result
            }
            // Suggest current file name if available
            if let Some(p) = file_path_manager.borrow().get(&current_page_num) {
                if let Some(name) = p.file_name() {
                    dialog.set_current_name(&name.to_string_lossy());
                }
            }


            let editor_notebook_clone = editor_notebook.clone();
            let active_tab_path_ref_clone = active_tab_path_ref.clone();
            let file_path_manager_clone = file_path_manager.clone();
            let current_dir_clone = current_dir.clone();
            let file_list_box_clone = file_list_box.clone();

            dialog.connect_response(move |d, resp| {
                if resp == gtk4::ResponseType::Accept {
                    if let Some(file_to_save) = d.file().and_then(|f| f.path()) {
                        let mime_type = mime_guess::from_path(&file_to_save).first_or_octet_stream();
                        if utils::is_allowed_mime_type(&mime_type) {
                            if let Ok(mut f_obj) = File::create(&file_to_save) {
                                let text = active_buffer.text(&active_buffer.start_iter(), &active_buffer.end_iter(), false);
                                if f_obj.write_all(text.as_bytes()).is_ok() {
                                    file_path_manager_clone.borrow_mut().insert(current_page_num, file_to_save.clone());
                                    *active_tab_path_ref_clone.borrow_mut() = Some(file_to_save.clone());

                                    // Update tab label
                                    if let Some(page) = editor_notebook_clone.nth_page(Some(current_page_num)) {
                                        if let Some(tab_label_widget) = editor_notebook_clone.tab_label(&page) {
                                            if let Some(label) = tab_label_widget.downcast_ref::<Label>() {
                                                label.set_text(&file_to_save.file_name().unwrap_or_default().to_string_lossy());
                                            }
                                        }
                                    }
                                    if let Some(parent) = file_to_save.parent() {
                                        *current_dir_clone.borrow_mut() = parent.to_path_buf();
                                    }
                                     utils::update_file_list(&file_list_box_clone, &current_dir_clone.borrow(), &active_tab_path_ref_clone.borrow());
                                }
                            }
                        }
                    }
                }
                d.close();
            });
            dialog.show();
        }
    });
}


fn setup_file_selection_handler(
    file_list_box: &ListBox,
    editor_notebook: &Notebook,
    active_tab_path_ref: &Rc<RefCell<Option<PathBuf>>>,
    file_path_manager: &Rc<RefCell<HashMap<u32, PathBuf>>>,
    current_dir: &Rc<RefCell<PathBuf>>,
    error_label: &Label,
    picture: &Picture, // Needs tab-specific handling
    save_button: &Button,
    save_as_button: &Button,
) {
    let editor_notebook = editor_notebook.clone();
    let active_tab_path_ref = active_tab_path_ref.clone();
    let file_path_manager = file_path_manager.clone();
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone(); // Used for updating list after dir change
    let error_label = error_label.clone();
    let picture = picture.clone();
    let save_button = save_button.clone();
    let save_as_button = save_as_button.clone();


    file_list_box.connect_row_activated(move |_, row| {
        if let Some(label) = row.child().and_then(|c| c.downcast::<Label>().ok()) {
            let file_name = label.text();
            let mut path_from_list = current_dir.borrow().clone();
            path_from_list.push(&file_name.as_str());

            if path_from_list.is_dir() {
                *current_dir.borrow_mut() = path_from_list;
                // Pass the active_tab_path_ref correctly for selection update
                utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &active_tab_path_ref.borrow());
            } else if path_from_list.is_file() {
                let mime_type = mime_guess::from_path(&path_from_list).first_or_octet_stream();
                if utils::is_allowed_mime_type(&mime_type) {
                    if let Ok(content) = std::fs::read_to_string(&path_from_list) {
                        open_or_focus_tab(
                            &editor_notebook,
                            &path_from_list,
                            &content,
                            &active_tab_path_ref,
                            &file_path_manager,
                            &save_button,
                            &save_as_button,
                            &mime_type,
                        );
                        // update_file_list might not be needed here unless current_dir changed
                        // utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &Some(path_from_list));
                    }
                } else if mime_type.type_() == "image" {
                    // Simplified image handling (same as open_button)
                     if editor_notebook.n_pages() == 1 {
                         if let Some(page_widget) = editor_notebook.nth_page(Some(0)){
                            if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                                if let Ok(pixbuf) = gtk4::gdk_pixbuf::Pixbuf::from_file(&path_from_list) {
                                    picture.set_pixbuf(Some(&pixbuf));
                                    sw.set_child(Some(&picture));
                                    *active_tab_path_ref.borrow_mut() = Some(path_from_list.clone());
                                    file_path_manager.borrow_mut().insert(0, path_from_list.clone());
                                    utils::update_save_buttons_visibility(&save_button, &save_as_button, Some(mime_type));
                                }
                            }
                         }
                    } else {
                        if let Some(page_widget) = editor_notebook.current_page().and_then(|p| editor_notebook.nth_page(Some(p))) {
                            if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                                sw.set_child(Some(&error_label));
                            }
                        }
                    }
                } else {
                     if let Some(page_widget) = editor_notebook.current_page().and_then(|p| editor_notebook.nth_page(Some(p))) {
                        if let Some(sw) = page_widget.downcast_ref::<ScrolledWindow>() {
                            sw.set_child(Some(&error_label));
                        }
                    }
                }
            }
        }
    });
}

fn setup_up_button_handler(
    up_button: &Button,
    current_dir: &Rc<RefCell<PathBuf>>,
    file_list_box: &ListBox,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>> // Changed from file_path
) {
    let current_dir = current_dir.clone();
    let file_list_box_clone = file_list_box.clone();
    let active_tab_path = active_tab_path.clone(); // Clone Rc for closure
    up_button.connect_clicked(move |_| {
        let mut path = current_dir.borrow().clone();
        if path.pop() {
            *current_dir.borrow_mut() = path;
            // Pass the active tab\'s path for selection highlighting
            utils::update_file_list(&file_list_box_clone, &current_dir.borrow(), &active_tab_path.borrow());
        }
    });
}

fn setup_refresh_button_handler(
    refresh_button: &Button,
    file_list_box: &ListBox,
    current_dir: &Rc<RefCell<PathBuf>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>> // Changed from file_path
) {
    let file_list_box = file_list_box.clone();
    let current_dir = current_dir.clone();
    let active_tab_path = active_tab_path.clone(); // Clone Rc for closure
    refresh_button.connect_clicked(move |_| {
        // Pass the active tab\'s path for selection highlighting
        utils::update_file_list(&file_list_box, &current_dir.borrow(), &active_tab_path.borrow());
    });
}
