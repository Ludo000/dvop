// Global Search UI and logic: search across current folder (like VS Code)

use gtk4::prelude::*;
use gtk4::{self as gtk, Box as GtkBox, Button, CheckButton, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, pango, TextView, TextBuffer, EventControllerKey, gdk};
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
    needle: String,
    case_sensitive: bool,
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
    if needle.is_empty() { return Vec::new(); }

    let mut results = Vec::new();
    
    // Check if needle contains newlines (multi-line search)
    if needle.contains('\n') {
        // Multi-line search: read entire file and search
        if let Ok(content) = fs::read_to_string(path) {
            let search_content = if case_sensitive { content.clone() } else { content.to_lowercase() };
            let search_needle = if case_sensitive { needle.to_string() } else { needle.to_lowercase() };
            
            let mut search_start = 0;
            while let Some(match_pos) = search_content[search_start..].find(&search_needle) {
                let abs_pos = search_start + match_pos;
                
                // Calculate line and column
                let before_match = &content[..abs_pos];
                let line_no = before_match.matches('\n').count() + 1;
                let last_newline = before_match.rfind('\n').map(|p| p + 1).unwrap_or(0);
                let col = abs_pos - last_newline + 1;
                
                // Get preview for multi-line match - show multiple lines from the file
                let match_end = abs_pos + needle.len();
                
                // Get the full line where the match starts
                let line_start = last_newline;
                
                // Show up to 5 lines of context
                let max_lines_to_show = 5;
                let mut preview_lines = Vec::new();
                let mut current_pos = line_start;
                
                for _ in 0..max_lines_to_show {
                    if current_pos >= content.len() {
                        break;
                    }
                    let next_newline = content[current_pos..].find('\n').map(|p| current_pos + p).unwrap_or(content.len());
                    let line_text = &content[current_pos..next_newline];
                    preview_lines.push(line_text.to_string());
                    
                    // Stop if we've passed the end of the match
                    if next_newline >= match_end {
                        break;
                    }
                    
                    current_pos = next_newline + 1;
                }
                
                // Count total lines in the match
                let match_text = &content[abs_pos..match_end.min(content.len())];
                let num_lines = match_text.matches('\n').count();
                
                let preview = if preview_lines.len() > 1 {
                    let shown = preview_lines.join("\n");
                    if num_lines + 1 > preview_lines.len() {
                        let remaining = num_lines + 1 - preview_lines.len();
                        format!("{}... (+{} more line{})", shown.trim_end(), remaining, if remaining == 1 { "" } else { "s" })
                    } else {
                        shown
                    }
                } else if !preview_lines.is_empty() {
                    preview_lines[0].to_string()
                } else {
                    String::new()
                };
                
                results.push(SearchResult {
                    path: path.to_path_buf(),
                    line: line_no,
                    col,
                    preview,
                    needle: needle.to_string(),
                    case_sensitive,
                });
                
                search_start = abs_pos + 1;
            }
        }
    } else {
        // Single-line search: line-by-line for better performance
        if let Ok(file) = fs::File::open(path) {
            let reader = BufReader::new(file);
            let mut line_no = 0usize;
            for line in reader.lines().flatten() {
                line_no += 1;
                if case_sensitive {
                    if let Some(idx) = line.find(needle) {
                        results.push(SearchResult { 
                            path: path.to_path_buf(), 
                            line: line_no, 
                            col: idx + 1, 
                            preview: line,
                            needle: needle.to_string(),
                            case_sensitive,
                        });
                    }
                } else {
                    let l = line.to_lowercase();
                    let n = needle.to_lowercase();
                    if let Some(idx) = l.find(&n) {
                        results.push(SearchResult { 
                            path: path.to_path_buf(), 
                            line: line_no, 
                            col: idx + 1, 
                            preview: line,
                            needle: needle.to_string(),
                            case_sensitive,
                        });
                    }
                }
            }
        }
    }
    results
}

/// Highlight all occurrences of needle in text with green background and white text
/// For multi-line needles, only highlight the portion that appears in the text
fn highlight_matches(text: &str, needle: &str, case_sensitive: bool) -> String {
    if needle.is_empty() {
        return glib::markup_escape_text(text).to_string();
    }
    
    // For multi-line needles, use only the first line for highlighting the preview
    let needle_to_highlight = if needle.contains('\n') {
        needle.lines().next().unwrap_or(needle)
    } else {
        needle
    };
    
    let mut result = String::new();
    let text_for_search = if case_sensitive { text.to_string() } else { text.to_lowercase() };
    let needle_for_search = if case_sensitive { 
        needle_to_highlight.to_string() 
    } else { 
        needle_to_highlight.to_lowercase() 
    };
    
    let mut last_end = 0;
    let mut search_start = 0;
    
    while search_start < text.len() {
        if let Some(match_pos) = text_for_search[search_start..].find(&needle_for_search) {
            let match_start = search_start + match_pos;
            let match_end = match_start + needle_to_highlight.len();
            
            // Add text before match (escaped)
            result.push_str(&glib::markup_escape_text(&text[last_end..match_start]));
            
            // Add highlighted match with green background and white text
            result.push_str("<span background='#26a269' foreground='#ffffff' weight='bold'>");
            result.push_str(&glib::markup_escape_text(&text[match_start..match_end]));
            result.push_str("</span>");
            
            last_end = match_end;
            search_start = match_end;
        } else {
            break;
        }
    }
    
    // Add remaining text (escaped)
    result.push_str(&glib::markup_escape_text(&text[last_end..]));
    result
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

    // Search text area
    let search_buffer = TextBuffer::new(None);
    let search_text_view = TextView::with_buffer(&search_buffer);
    search_text_view.set_wrap_mode(gtk::WrapMode::Word);
    search_text_view.set_accepts_tab(false);
    search_text_view.add_css_class("monospace");
    
    let search_scroller = ScrolledWindow::builder()
        .child(&search_text_view)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .height_request(80)
        .hexpand(true)
        .build();
    
    // Controls row
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.set_margin_bottom(8);
    controls.append(&search_scroller);
    
    let controls_right = GtkBox::new(Orientation::Vertical, 4);
    let case_cb = CheckButton::with_label("Case sensitive");
    let search_btn = Button::with_label("Search");
    search_btn.add_css_class("suggested-action");
    controls_right.append(&case_cb);
    controls_right.append(&search_btn);
    controls.append(&controls_right);

    // Status label
    let status = Label::new(Some(""));
    status.set_xalign(0.0);
    status.add_css_class("dim-label");
    status.set_margin_bottom(4);

    // Results list
    let results_list = ListBox::new();
    results_list.set_selection_mode(gtk::SelectionMode::Single);
    results_list.add_css_class("boxed-list");
    results_list.add_css_class("zebra-list");
    let scroller = ScrolledWindow::builder()
        .vexpand(true)
        .hexpand(true)
        .child(&results_list)
        .build();

    vbox.append(&controls);
    vbox.append(&status);
    vbox.append(&scroller);
    content.append(&vbox);

    // Channel for results - will be recreated for each search
    let sender_rc: Rc<RefCell<Option<std::sync::mpsc::Sender<Option<SearchResult>>>>> = Rc::new(RefCell::new(None));
    let receiver_rc: Rc<RefCell<Option<std::sync::mpsc::Receiver<Option<SearchResult>>>>> = Rc::new(RefCell::new(None));

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
            if let Some(vbox) = child.downcast_ref::<GtkBox>() {
                // Get the data from the box's first child (invisible label with metadata)
                if let Some(first_child) = vbox.first_child() {
                    if let Some(data_label) = first_child.downcast_ref::<Label>() {
                        if let Some(tt) = data_label.tooltip_text() {
                            // Format: path|line|col|needle|case_sensitive
                            let parts: Vec<&str> = tt.splitn(5, '|').collect();
                            if parts.len() >= 5 {
                                let path = PathBuf::from(parts[0]);
                                let line: usize = parts[1].parse().unwrap_or(1);
                                let col: usize = parts[2].parse().unwrap_or(1);
                                let needle = parts[3].to_string();
                                let _case_sensitive: bool = parts[4].parse().unwrap_or(false);

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

                                        // After opening, jump to position and select matching text (delay to ensure buffer ready)
                                        let editor_notebook_for_jump = editor_notebook_c.clone();
                                        let needle_clone = needle.clone();
                                        glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                                            if let Some((text_view, buffer)) = crate::handlers::get_active_text_view_and_buffer(&editor_notebook_for_jump) {
                                                // Get start position
                                                let mut start_iter = buffer.start_iter();
                                                let target_line = line.saturating_sub(1) as i32;
                                                start_iter.set_line(target_line);
                                                let target_col = col.saturating_sub(1) as i32;
                                                start_iter.set_line_offset(target_col);
                                                
                                                // Calculate end position based on needle length
                                                let mut end_iter = start_iter;
                                                if needle_clone.contains('\n') {
                                                    // Multi-line match: select from start to end of match
                                                    let lines_to_add = needle_clone.matches('\n').count() as i32;
                                                    end_iter.forward_lines(lines_to_add);
                                                    // Find the position after last line
                                                    if let Some(last_line) = needle_clone.lines().last() {
                                                        end_iter.set_line_offset(last_line.len() as i32);
                                                    }
                                                } else {
                                                    // Single-line match: select needle length
                                                    end_iter.forward_chars(needle_clone.chars().count() as i32);
                                                }
                                                
                                                // Select the text
                                                buffer.select_range(&start_iter, &end_iter);
                                                
                                                // Ensure visible
                                                if let Ok(view) = text_view.downcast::<sourceview5::View>() {
                                                    let mut it2 = start_iter;
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
            }
        }
    });

    // Track result count
    let result_count = Rc::new(RefCell::new(0usize));
    
    // Trigger search on button or Enter
    let start_search: Rc<dyn Fn(String, bool)> = {
        let status = status.clone();
        let results_list = results_list.clone();
        let result_count_clone = result_count.clone();
        let receiver_rc_clone = receiver_rc.clone();
        let sender_rc_clone = sender_rc.clone();
        Rc::new(move |query: String, case_sensitive: bool| {
            // Clear previous results
            while let Some(child) = results_list.first_child() { results_list.remove(&child); }
            *result_count_clone.borrow_mut() = 0;
            status.set_text("Searching...");
            
            // Create new channel for this search (this will automatically drop the old receiver,
            // causing the old timer to finish naturally)
            let (sender, receiver) = std::sync::mpsc::channel::<Option<SearchResult>>();
            *sender_rc_clone.borrow_mut() = Some(sender.clone());
            *receiver_rc_clone.borrow_mut() = Some(receiver);

            let root = SEARCH_FOLDER.with(|sf| {
                sf.borrow().clone().expect("Search folder should be initialized")
            });
            
            // Start search in background thread
            std::thread::spawn(move || {
                let mut files = Vec::with_capacity(2048);
                walk_dir_recursive(&root, &mut files, 20000);
                let mut found = 0usize;
                for p in files {
                    for r in search_file(&p, &query, case_sensitive, 5 * 1024 * 1024) {
                        found += 1;
                        let _ = sender.send(Some(r));
                        if found >= 10000 { break; }
                    }
                    if found >= 10000 { break; }
                }
                let _ = sender.send(None);
            });
            
            // Start polling timer to receive results
            let results_list_c = results_list.clone();
            let status_c = status.clone();
            let receiver_rc_c = receiver_rc_clone.clone();
            let result_count_c = result_count_clone.clone();
            glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let mut finished = false;
                let mut processed = 0usize;
                if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                    while processed < 200 {
                        match rx.try_recv() {
                            Ok(Some(sr)) => {
                                let row = ListBoxRow::new();
                                
                                // Create a vertical box for the result layout
                                let result_vbox = GtkBox::new(Orientation::Vertical, 2);
                                result_vbox.set_margin_top(4);
                                result_vbox.set_margin_bottom(4);
                                result_vbox.set_margin_start(8);
                                result_vbox.set_margin_end(8);
                                
                                // Hidden label with metadata (for click handler)
                                let data_label = Label::new(None);
                                data_label.set_visible(false);
                                data_label.set_tooltip_text(Some(&format!("{}|{}|{}|{}|{}",
                                    sr.path.display(), sr.line, sr.col, sr.needle, sr.case_sensitive)));
                                result_vbox.append(&data_label);
                                
                                // File path and location (bold, colored)
                                let file_name = sr.path.file_name()
                                    .and_then(|n| n.to_str())
                                    .unwrap_or("unknown");
                                
                                let location_box = GtkBox::new(Orientation::Horizontal, 4);
                                let file_label = Label::new(None);
                                file_label.set_markup(&format!("<b>{}</b>", glib::markup_escape_text(file_name)));
                                file_label.set_xalign(0.0);
                                file_label.add_css_class("accent");
                                
                                let line_col_label = Label::new(Some(&format!("  {}:{}", sr.line, sr.col)));
                                line_col_label.set_xalign(0.0);
                                line_col_label.add_css_class("dim-label");
                                
                                location_box.append(&file_label);
                                location_box.append(&line_col_label);
                                
                                // Add multi-line indicator badge if needle contains newlines
                                if sr.needle.contains('\n') {
                                    let multiline_badge = Label::new(Some("  ⏎ multi-line"));
                                    multiline_badge.set_xalign(0.0);
                                    multiline_badge.add_css_class("dim-label");
                                    location_box.append(&multiline_badge);
                                }
                                
                                result_vbox.append(&location_box);
                                
                                // Preview text (with proper trimming and highlighting)
                                let sanitized_preview = sr.preview
                                    .replace('\0', " ")
                                    .replace('\r', "")
                                    .trim()
                                    .to_string();
                                
                                if !sanitized_preview.is_empty() {
                                    let preview_label = Label::new(Some(&sanitized_preview));
                                    preview_label.set_xalign(0.0);
                                    // Don't ellipsize if it's a multi-line match preview (already truncated)
                                    if !sanitized_preview.contains("more line") {
                                        preview_label.set_ellipsize(pango::EllipsizeMode::End);
                                        preview_label.set_max_width_chars(80);
                                    }
                                    preview_label.add_css_class("monospace");
                                    preview_label.set_margin_top(2);
                                    result_vbox.append(&preview_label);
                                }
                                
                                // Add zebra striping
                                let count = *result_count_c.borrow();
                                if count % 2 == 0 {
                                    row.add_css_class("zebra-even");
                                } else {
                                    row.add_css_class("zebra-odd");
                                }
                                
                                row.set_child(Some(&result_vbox));
                                results_list_c.append(&row);
                                
                                *result_count_c.borrow_mut() += 1;
                                let count = *result_count_c.borrow();
                                status_c.set_text(&format!("Found {} result{}", count, if count == 1 { "" } else { "s" }));
                                
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
                    let count = *result_count_c.borrow();
                    if count == 0 {
                        status_c.set_text("No results found");
                    } else {
                        status_c.set_text(&format!("Search complete - {} result{} found", count, if count == 1 { "" } else { "s" }));
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        })
    };

    let cb_clone_btn = start_search.clone();
    let buffer_weak = search_buffer.downgrade();
    let case_cb_weak = case_cb.downgrade();
    search_btn.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_cb)) = (buffer_weak.upgrade(), case_cb_weak.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            (cb_clone_btn)(text, case_cb.is_active());
        }
    });
    
    // Add keyboard handling to TextView: Enter to search, Shift+Enter for line break
    let key_controller = EventControllerKey::new();
    let cb_clone_enter = start_search.clone();
    let buffer_weak_key = search_buffer.downgrade();
    let case_cb_weak_key = case_cb.downgrade();
    key_controller.connect_key_pressed(move |_controller, key, _code, modifier| {
        if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
            if modifier.contains(gdk::ModifierType::SHIFT_MASK) {
                // Shift+Enter: allow default behavior (insert line break)
                glib::Propagation::Proceed
            } else {
                // Enter: trigger search
                if let (Some(buffer), Some(case_cb)) = (buffer_weak_key.upgrade(), case_cb_weak_key.upgrade()) {
                    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
                    (cb_clone_enter)(text, case_cb.is_active());
                }
                glib::Propagation::Stop
            }
        } else {
            glib::Propagation::Proceed
        }
    });
    search_text_view.add_controller(key_controller);

    dialog.add_button("Close", gtk::ResponseType::Close);
    dialog.connect_response(|d, _| {
        d.set_visible(false);
    });
    dialog.present();
}
