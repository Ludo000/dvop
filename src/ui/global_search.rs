// Global Search UI and logic: search across current folder (like VS Code)

use gtk4::prelude::*;
use gtk4::{self as gtk, Box as GtkBox, Button, CheckButton, Entry, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow};
use glib::{self};
use std::cell::RefCell;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::rc::Rc;

// Thread-local storage for the global search dialog to maintain state
thread_local! {
    static GLOBAL_SEARCH_DIALOG: RefCell<Option<gtk::Dialog>> = RefCell::new(None);
    static SEARCH_FOLDER: RefCell<Option<PathBuf>> = RefCell::new(None);
}

#[derive(Clone, Debug)]
struct SearchResult {
    path: PathBuf,
    line: usize,   // 1-based
    col: usize,    // 1-based
    preview: String,
}

fn is_text_file(path: &Path) -> bool {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    crate::utils::is_allowed_mime_type(&mime)
}

fn search_file(path: &Path, needle: &str, case_sensitive: bool, max_file_size_bytes: u64) -> Vec<SearchResult> {
    // Skip very large files to avoid UI stalls
    if let Ok(meta) = fs::metadata(path) {
        if meta.len() > max_file_size_bytes { return Vec::new(); }
        if !meta.is_file() { return Vec::new(); }
    }
    if !is_text_file(path) { return Vec::new(); }

    let mut results = Vec::new();
    if let Ok(file) = fs::File::open(path) {
        let reader = BufReader::new(file);
        let mut line_no = 0usize;
        for line in reader.lines().flatten() {
            line_no += 1;
            if needle.is_empty() { continue; }
            if case_sensitive {
                if let Some(idx) = line.find(needle) {
                    results.push(SearchResult { path: path.to_path_buf(), line: line_no, col: idx + 1, preview: line });
                }
            } else {
                let l = line.to_lowercase();
                let n = needle.to_lowercase();
                if let Some(idx) = l.find(&n) {
                    results.push(SearchResult { path: path.to_path_buf(), line: line_no, col: idx + 1, preview: line });
                }
            }
        }
    }
    results
}

fn walk_dir_recursive(root: &Path, files_out: &mut Vec<PathBuf>, max_files: usize) {
    if files_out.len() >= max_files { return; }
    if let Ok(read) = fs::read_dir(root) {
        for entry in read.flatten() {
            let p = entry.path();
            // Skip hidden files/dirs
            if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                if name.starts_with('.') { continue; }
                // Skip target directory by convention
                if name == "target" { continue; }
                // Skip node_modules if present
                if name == "node_modules" { continue; }
            }

            if p.is_dir() {
                walk_dir_recursive(&p, files_out, max_files);
                if files_out.len() >= max_files { return; }
            } else if p.is_file() {
                files_out.push(p);
                if files_out.len() >= max_files { return; }
            }
        }
    }
}

pub fn show_global_search_dialog(
    parent_window: &gtk::ApplicationWindow,
    current_dir: &Rc<RefCell<PathBuf>>,
    editor_notebook: &gtk::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    save_button: &gtk::Button,
    save_as_button: &gtk::Button,
    file_list_box: &gtk::ListBox,
) {
    // Check if dialog already exists
    let existing_dialog = GLOBAL_SEARCH_DIALOG.with(|d| d.borrow().clone());
    
    if let Some(dialog) = existing_dialog {
        // Dialog exists, just show it
        dialog.present();
        return;
    }
    
    // Initialize search folder with current directory if not set
    let search_folder = SEARCH_FOLDER.with(|sf| {
        let mut sf_mut = sf.borrow_mut();
        if sf_mut.is_none() {
            *sf_mut = Some(current_dir.borrow().clone());
        }
        sf_mut.clone().unwrap()
    });
    
    // Create new dialog
    let dialog = gtk::Dialog::builder()
        .transient_for(parent_window)
        .modal(true)
        .title("Global Search")
        .resizable(true)
        .default_width(800)
        .default_height(500)
        .build();

    // Store dialog for reuse
    GLOBAL_SEARCH_DIALOG.with(|d| {
        *d.borrow_mut() = Some(dialog.clone());
    });
    
    // Handle dialog close to hide instead of destroy
    dialog.connect_close_request(|dialog| {
        dialog.set_visible(false);
        glib::Propagation::Stop
    });

    // Content
    let content = dialog.content_area();
    let vbox = GtkBox::new(Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Show current folder path with change button
    let folder_box = GtkBox::new(Orientation::Horizontal, 8);
    let folder_label = Label::new(None);
    folder_label.set_xalign(0.0);
    folder_label.set_markup(&format!("<b>Search in:</b> {}", search_folder.display()));
    folder_label.set_hexpand(true);
    folder_label.add_css_class("dim-label");
    
    let change_folder_btn = Button::with_label("Change Folder...");
    change_folder_btn.set_tooltip_text(Some("Select a different folder to search"));
    
    folder_box.append(&folder_label);
    folder_box.append(&change_folder_btn);
    folder_box.set_margin_bottom(8);
    vbox.append(&folder_box);
    
    // Connect change folder button
    {
        let parent_window_c = parent_window.clone();
        let folder_label_c = folder_label.clone();
        let search_dialog_c = dialog.clone();
        
        change_folder_btn.connect_clicked(move |_| {
            // Temporarily hide the search dialog to avoid modal conflicts
            search_dialog_c.set_visible(false);
            
            let chooser_dialog = gtk::FileChooserDialog::new(
                Some("Select Search Folder"),
                Some(&parent_window_c),
                gtk::FileChooserAction::SelectFolder,
                &[("Cancel", gtk::ResponseType::Cancel), ("Select", gtk::ResponseType::Accept)],
            );
            
            chooser_dialog.set_modal(true); // Set back to modal now that search dialog is hidden
            
            // Set the current search folder as default
            let current_search_folder = SEARCH_FOLDER.with(|sf| {
                sf.borrow().clone().expect("Search folder should be initialized")
            });
            let current_file = gtk::gio::File::for_path(&current_search_folder);
            let _ = chooser_dialog.set_current_folder(Some(&current_file));
            
            let folder_label_clone = folder_label_c.clone();
            let search_dialog_clone = search_dialog_c.clone();
            
            chooser_dialog.connect_response(move |chooser_dialog, response| {
                if response == gtk::ResponseType::Accept {
                    if let Some(folder) = chooser_dialog.file() {
                        if let Some(path) = folder.path() {
                            // Update only the search folder (not the app's current directory)
                            SEARCH_FOLDER.with(|sf| {
                                *sf.borrow_mut() = Some(path.clone());
                            });
                            
                            // Update the folder label in the search dialog
                            folder_label_clone.set_markup(&format!("<b>Search in:</b> {}", path.display()));
                        }
                    }
                }
                chooser_dialog.close();
                
                // Show the search dialog again
                search_dialog_clone.present();
            });
            
            chooser_dialog.present();
        });
    }

    // Controls row
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    let entry = Entry::new();
    entry.set_placeholder_text(Some("Search in folder..."));
    entry.set_hexpand(true);
    let case_cb = CheckButton::with_label("Case sensitive");
    let search_btn = Button::with_label("Search");
    controls.append(&entry);
    controls.append(&case_cb);
    controls.append(&search_btn);

    // Status label
    let status = Label::new(Some(""));
    status.add_css_class("dim-label");

    // Results list
    let results_list = ListBox::new();
    results_list.set_selection_mode(gtk::SelectionMode::Single);
    let scroller = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&results_list)
        .build();

    vbox.append(&controls);
    vbox.append(&status);
    vbox.append(&scroller);
    content.append(&vbox);

    // Channel for results (std mpsc) and a glib timer to poll it on main thread
    let (sender, receiver) = std::sync::mpsc::channel::<Option<SearchResult>>();
    let receiver_rc: Rc<RefCell<Option<std::sync::mpsc::Receiver<Option<SearchResult>>>>> = Rc::new(RefCell::new(Some(receiver)));

    // Open result handler (row activation)
    let parent_window_c = parent_window.clone();
    let editor_notebook_c = editor_notebook.clone();
    let file_path_manager_c = file_path_manager.clone();
    let active_tab_path_c = active_tab_path.clone();
    let save_button_c = save_button.clone();
    let save_as_button_c = save_as_button.clone();
    let file_list_box_c = file_list_box.clone();
    let current_dir_c = current_dir.clone();
    let dialog_close = dialog.clone();

    results_list.connect_row_activated(move |_list, row| {
        if let Some(child) = row.child() {
            if let Some(lbl) = child.downcast_ref::<Label>() {
                // We stored full info in widget tooltip as JSON-ish text to avoid hidden state
                if let Some(tt) = lbl.tooltip_text() {
                    // Format: path|line|col|needle
                    let parts: Vec<&str> = tt.splitn(4, '|').collect();
                    if parts.len() >= 3 {
                        let path = PathBuf::from(parts[0]);
                        let line: usize = parts[1].parse().unwrap_or(1);
                        let col: usize = parts[2].parse().unwrap_or(1);

                        // Open (or focus) the file
                        let mime = mime_guess::from_path(&path).first_or_octet_stream();
                        let content_opt = if crate::utils::is_allowed_mime_type(&mime) {
                            fs::read_to_string(&path).ok()
                        } else { None };

                        if crate::utils::is_allowed_mime_type(&mime) {
                            if let Some(content) = content_opt {
                                crate::handlers::open_or_focus_tab(
                                    &editor_notebook_c,
                                    &path,
                                    &content,
                                    &active_tab_path_c,
                                    &file_path_manager_c,
                                    &save_button_c,
                                    &save_as_button_c,
                                    &mime,
                                    &parent_window_c,
                                    &file_list_box_c,
                                    &current_dir_c,
                                    None,
                                );

                                // Update global folder to the file's parent directory (after opening)
                                if let Some(parent) = path.parent() {
                                    let parent_buf = parent.to_path_buf();
                                    *current_dir_c.borrow_mut() = parent_buf.clone();
                                    
                                    // Update file manager to show the file's directory and select it
                                    crate::utils::update_file_list(
                                        &file_list_box_c,
                                        &parent_buf,
                                        &active_tab_path_c.borrow(),
                                        crate::utils::FileSelectionSource::TabSwitch
                                    );
                                    
                                    // Save to settings
                                    let mut settings = crate::settings::get_settings_mut();
                                    settings.set_last_folder(&parent_buf);
                                    let _ = settings.save();
                                    drop(settings);
                                }

                                // After opening, jump to position (delay to ensure buffer ready)
                                let editor_notebook_for_jump = editor_notebook_c.clone();
                                glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                                    if let Some((text_view, buffer)) = crate::handlers::get_active_text_view_and_buffer(&editor_notebook_for_jump) {
                                        // Place cursor to line/col
                                        let mut iter = buffer.start_iter();
                                        // Move to line (clamp)
                                        let target_line = line.saturating_sub(1) as i32;
                                        iter.set_line(target_line);
                                        let target_col = col.saturating_sub(1) as i32;
                                        iter.set_line_offset(target_col);
                                        buffer.place_cursor(&iter);
                                        // Ensure visible
                                        if let Ok(view) = text_view.downcast::<sourceview5::View>() {
                                            let mut it2 = iter;
                                            view.scroll_to_iter(&mut it2, 0.25, false, 0.0, 0.0);
                                        }
                                    }
                                });
                                dialog_close.set_visible(false);
                            }
                        } else {
                            // Non-text file: still open via handler with empty content
                            crate::handlers::open_or_focus_tab(
                                &editor_notebook_c,
                                &path,
                                "",
                                &active_tab_path_c,
                                &file_path_manager_c,
                                &save_button_c,
                                &save_as_button_c,
                                &mime,
                                &parent_window_c,
                                &file_list_box_c,
                                &current_dir_c,
                                None,
                            );
                            
                            // Update global folder to the file's parent directory (after opening)
                            if let Some(parent) = path.parent() {
                                let parent_buf = parent.to_path_buf();
                                *current_dir_c.borrow_mut() = parent_buf.clone();
                                
                                // Update file manager to show the file's directory and select it
                                crate::utils::update_file_list(
                                    &file_list_box_c,
                                    &parent_buf,
                                    &active_tab_path_c.borrow(),
                                    crate::utils::FileSelectionSource::TabSwitch
                                );
                                
                                // Save to settings
                                let mut settings = crate::settings::get_settings_mut();
                                settings.set_last_folder(&parent_buf);
                                let _ = settings.save();
                                drop(settings);
                            }
                            
                            dialog_close.set_visible(false);
                        }
                    }
                }
            }
        }
    });

    // Polling timer to receive results without blocking main thread
    {
        let results_list_c = results_list.clone();
        let status_c = status.clone();
        let receiver_rc_c = receiver_rc.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
            let mut finished = false;
            let mut processed = 0usize;
            if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                while processed < 200 {
                    match rx.try_recv() {
                        Ok(Some(sr)) => {
                            let row = ListBoxRow::new();
                            // Sanitize preview to remove null bytes and other problematic chars
                            let sanitized_preview = sr.preview.replace('\0', " ").replace('\r', "");
                            let display = format!("{}:{}:{}  {}",
                                sr.path.display(), sr.line, sr.col, sanitized_preview);
                            let lbl = Label::new(Some(&display));
                            lbl.set_xalign(0.0);
                            lbl.set_tooltip_text(Some(&format!("{}|{}|{}|{}",
                                sr.path.display(), sr.line, sr.col, sanitized_preview)));
                            row.set_child(Some(&lbl));
                            results_list_c.append(&row);
                            processed += 1;
                        }
                        Ok(None) => { finished = true; break; }
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => { finished = true; break; }
                    }
                }
            } else {
                finished = true;
            }
            if finished {
                status_c.set_text("Search complete");
                glib::ControlFlow::Break
            } else {
                glib::ControlFlow::Continue
            }
        });
    }

    // Trigger search on button or Enter
    let start_search: Rc<dyn Fn(String, bool)> = {
        let sender = sender.clone();
        let status = status.clone();
        let results_list = results_list.clone();
        Rc::new(move |query: String, case_sensitive: bool| {
            // Clear previous
            while let Some(child) = results_list.first_child() { results_list.remove(&child); }
            status.set_text("Searching...");

            let root = SEARCH_FOLDER.with(|sf| {
                sf.borrow().clone().expect("Search folder should be initialized")
            });
            let tx = sender.clone();
            std::thread::spawn(move || {
                let mut files = Vec::with_capacity(2048);
                walk_dir_recursive(&root, &mut files, 20000);
                let mut found = 0usize;
                for p in files {
                    for r in search_file(&p, &query, case_sensitive, 5 * 1024 * 1024) {
                        found += 1;
                        let _ = tx.send(Some(r));
                        if found >= 10000 { break; }
                    }
                    if found >= 10000 { break; }
                }
                let _ = tx.send(None);
            });
        })
    };

    let cb_clone_btn = start_search.clone();
    let entry_weak = entry.downgrade();
    let case_cb_weak = case_cb.downgrade();
    search_btn.connect_clicked(move |_| {
        if let (Some(entry), Some(case_cb)) = (entry_weak.upgrade(), case_cb_weak.upgrade()) {
            (cb_clone_btn)(entry.text().to_string(), case_cb.is_active());
        }
    });

    let cb_clone_enter = start_search.clone();
    entry.connect_activate(move |e| {
        let q = e.text().to_string();
        (cb_clone_enter)(q, false);
    });

    dialog.add_button("Close", gtk::ResponseType::Close);
    dialog.connect_response(|d, _| {
        d.set_visible(false);
    });
    dialog.present();
}
