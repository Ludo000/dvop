// Global Search UI and logic: search across current folder (like VS Code)

use gtk4::prelude::*;
use gtk4::{self as gtk, Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow, pango, TextView, TextBuffer, EventControllerKey, gdk};
use gtk4::subclass::prelude::ObjectSubclassIsExt;
use glib::{self};
use std::cell::RefCell;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use super::search_panel_template::SearchPanel;

// Thread-local storage for the global search dialog to maintain state
thread_local! {
    static GLOBAL_SEARCH_DIALOG: RefCell<Option<gtk::Dialog>> = RefCell::new(None);
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

/// Search in string content (for open buffer content)
fn search_in_content(
    path: &Path,
    content: &str,
    query: &str,
    case_sensitive: bool,
    whole_word: bool,
) -> Vec<SearchResult> {
    let mut results = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let query_lower = query.to_lowercase();
    
    for (line_num, line) in lines.iter().enumerate() {
        let search_line = if case_sensitive { line.to_string() } else { line.to_lowercase() };
        let search_query = if case_sensitive { query } else { &query_lower };
        
        let mut start = 0;
        while let Some(pos) = search_line[start..].find(search_query) {
            let actual_pos = start + pos;
            
            // Check whole word if needed
            if whole_word {
                let before_ok = actual_pos == 0 || !search_line.as_bytes()[actual_pos - 1].is_ascii_alphanumeric();
                let after_idx = actual_pos + search_query.len();
                let after_ok = after_idx >= search_line.len() || !search_line.as_bytes()[after_idx].is_ascii_alphanumeric();
                
                if !(before_ok && after_ok) {
                    start = actual_pos + 1;
                    continue;
                }
            }
            
            results.push(SearchResult {
                path: path.to_path_buf(),
                line: line_num + 1,
                col: actual_pos + 1,
                preview: line.to_string(),
                needle: query.to_string(),
                case_sensitive,
            });
            
            start = actual_pos + 1;
        }
    }
    
    results
}

/// Search in an open buffer instead of reading from disk
#[allow(dead_code)]
fn search_in_buffer(
    editor_notebook: &gtk::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
    path: &Path,
    needle: &str,
    case_sensitive: bool,
    whole_word: bool,
) -> Option<Vec<SearchResult>> {
    // Find the tab with this file
    let file_path_map = file_path_manager.borrow();
    let mut target_page_num: Option<u32> = None;
    
    for (page_num, tab_path) in file_path_map.iter() {
        if tab_path == path {
            target_page_num = Some(*page_num);
            break;
        }
    }
    
    drop(file_path_map);
    
    let page_num = target_page_num?;
    
    // Get the page widget
    let page = editor_notebook.nth_page(Some(page_num))?;
    let scrolled = page.downcast_ref::<gtk4::ScrolledWindow>()?;
    let child = scrolled.child()?;
    
    // Get buffer content
    let content = if let Some(source_view) = child.downcast_ref::<sourceview5::View>() {
        let buffer = source_view.buffer();
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
    } else if let Some(text_view) = child.downcast_ref::<gtk4::TextView>() {
        let buffer = text_view.buffer();
        buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
    } else {
        return None;
    };
    
    // Now search in the buffer content
    let mut results = Vec::new();
    
    if needle.is_empty() {
        return Some(results);
    }
    
    // Check if needle contains newlines (multi-line search)
    if needle.contains('\n') {
        let search_content = if case_sensitive { content.clone() } else { content.to_lowercase() };
        let search_needle = if case_sensitive { needle.to_string() } else { needle.to_lowercase() };
        
        let mut search_start = 0;
        while let Some(match_pos) = search_content[search_start..].find(&search_needle) {
            let abs_pos = search_start + match_pos;
            
            // Check whole word match
            if whole_word {
                let match_end = abs_pos + search_needle.len();
                let before_ok = abs_pos == 0 || !content.chars().nth(abs_pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                let after_ok = match_end >= content.len() || !content.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                if !before_ok || !after_ok {
                    search_start = abs_pos + 1;
                    continue;
                }
            }
            
            // Calculate line and column
            let before_match = &content[..abs_pos];
            let line_no = before_match.matches('\n').count() + 1;
            let last_newline = before_match.rfind('\n').map(|p| p + 1).unwrap_or(0);
            let col = abs_pos - last_newline + 1;
            
            // Get preview
            let match_end = abs_pos + needle.len();
            let line_start = last_newline;
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
                
                if next_newline >= match_end {
                    break;
                }
                
                current_pos = next_newline + 1;
            }
            
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
    } else {
        // Single-line search
        let mut line_no = 0usize;
        for line in content.lines() {
            line_no += 1;
            if case_sensitive {
                let mut search_pos = 0;
                while let Some(idx) = line[search_pos..].find(needle) {
                    let abs_idx = search_pos + idx;
                    if whole_word {
                        let match_end = abs_idx + needle.len();
                        let before_ok = abs_idx == 0 || !line.chars().nth(abs_idx - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        let after_ok = match_end >= line.len() || !line.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        if before_ok && after_ok {
                            results.push(SearchResult { 
                                path: path.to_path_buf(), 
                                line: line_no, 
                                col: abs_idx + 1, 
                                preview: line.to_string(),
                                needle: needle.to_string(),
                                case_sensitive,
                            });
                        }
                    } else {
                        results.push(SearchResult { 
                            path: path.to_path_buf(), 
                            line: line_no, 
                            col: abs_idx + 1, 
                            preview: line.to_string(),
                            needle: needle.to_string(),
                            case_sensitive,
                        });
                    }
                    search_pos = abs_idx + 1;
                }
            } else {
                let l = line.to_lowercase();
                let n = needle.to_lowercase();
                let mut search_pos = 0;
                while let Some(idx) = l[search_pos..].find(&n) {
                    let abs_idx = search_pos + idx;
                    if whole_word {
                        let match_end = abs_idx + n.len();
                        let before_ok = abs_idx == 0 || !l.chars().nth(abs_idx - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        let after_ok = match_end >= l.len() || !l.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                        if before_ok && after_ok {
                            results.push(SearchResult { 
                                path: path.to_path_buf(), 
                                line: line_no, 
                                col: abs_idx + 1, 
                                preview: line.to_string(),
                                needle: needle.to_string(),
                                case_sensitive,
                            });
                        }
                    } else {
                        results.push(SearchResult { 
                            path: path.to_path_buf(), 
                            line: line_no, 
                            col: abs_idx + 1, 
                            preview: line.to_string(),
                            needle: needle.to_string(),
                            case_sensitive,
                        });
                    }
                    search_pos = abs_idx + 1;
                }
            }
        }
    }
    
    Some(results)
}

fn search_file(path: &Path, needle: &str, case_sensitive: bool, whole_word: bool, max_file_size_bytes: u64) -> Vec<SearchResult> {
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
                
                // Check whole word match
                if whole_word {
                    let match_end = abs_pos + search_needle.len();
                    let before_ok = abs_pos == 0 || !content.chars().nth(abs_pos - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                    let after_ok = match_end >= content.len() || !content.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                    if !before_ok || !after_ok {
                        search_start = abs_pos + 1;
                        continue;
                    }
                }
                
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
                    let mut search_pos = 0;
                    while let Some(idx) = line[search_pos..].find(needle) {
                        let abs_idx = search_pos + idx;
                        // Check whole word
                        if whole_word {
                            let match_end = abs_idx + needle.len();
                            let before_ok = abs_idx == 0 || !line.chars().nth(abs_idx - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                            let after_ok = match_end >= line.len() || !line.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                            if before_ok && after_ok {
                                results.push(SearchResult { 
                                    path: path.to_path_buf(), 
                                    line: line_no, 
                                    col: abs_idx + 1, 
                                    preview: line.clone(),
                                    needle: needle.to_string(),
                                    case_sensitive,
                                });
                            }
                        } else {
                            results.push(SearchResult { 
                                path: path.to_path_buf(), 
                                line: line_no, 
                                col: abs_idx + 1, 
                                preview: line.clone(),
                                needle: needle.to_string(),
                                case_sensitive,
                            });
                        }
                        search_pos = abs_idx + 1;
                    }
                } else {
                    let l = line.to_lowercase();
                    let n = needle.to_lowercase();
                    let mut search_pos = 0;
                    while let Some(idx) = l[search_pos..].find(&n) {
                        let abs_idx = search_pos + idx;
                        // Check whole word
                        if whole_word {
                            let match_end = abs_idx + n.len();
                            let before_ok = abs_idx == 0 || !l.chars().nth(abs_idx - 1).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                            let after_ok = match_end >= l.len() || !l.chars().nth(match_end).map(|c| c.is_alphanumeric() || c == '_').unwrap_or(false);
                            if before_ok && after_ok {
                                results.push(SearchResult { 
                                    path: path.to_path_buf(), 
                                    line: line_no, 
                                    col: abs_idx + 1, 
                                    preview: line.clone(),
                                    needle: needle.to_string(),
                                    case_sensitive,
                                });
                            }
                        } else {
                            results.push(SearchResult { 
                                path: path.to_path_buf(), 
                                line: line_no, 
                                col: abs_idx + 1, 
                                preview: line.clone(),
                                needle: needle.to_string(),
                                case_sensitive,
                            });
                        }
                        search_pos = abs_idx + 1;
                    }
                }
            }
        }
    }
    results
}

/// Highlight all occurrences of needle in text with green background and white text
/// For multi-line needles, only highlight the portion that appears in the text
#[allow(dead_code)]
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

/// Reload a file in the editor if it's currently open
#[allow(dead_code)]
fn reload_file_in_editor(
    path: &Path,
    editor_notebook: &gtk::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
) {
    println!("reload_file_in_editor called for: {}", path.display());
    
    // Find the tab with this file
    let file_path_map = file_path_manager.borrow();
    let mut target_page_num: Option<u32> = None;
    
    println!("Checking {} open tabs", file_path_map.len());
    for (page_num, tab_path) in file_path_map.iter() {
        println!("  Tab {}: {}", page_num, tab_path.display());
        if tab_path == path {
            target_page_num = Some(*page_num);
            println!("  -> MATCH found at page {}", page_num);
            break;
        }
    }
    
    drop(file_path_map); // Release the borrow
    
    if let Some(page_num) = target_page_num {
        println!("File is open at page {}, attempting reload", page_num);
        // File is open, reload its content
        if let Some(page) = editor_notebook.nth_page(Some(page_num)) {
            println!("Got page widget");
            if let Some(scrolled) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                println!("Page is ScrolledWindow");
                if let Some(child) = scrolled.child() {
                    println!("ScrolledWindow has child");
                    // Try sourceview5::View first (most likely in this editor)
                    if let Some(source_view) = child.downcast_ref::<sourceview5::View>() {
                        println!("Child is SourceView");
                        let buffer = source_view.buffer();
                        
                        // Read the updated file content
                        if let Ok(new_content) = fs::read_to_string(path) {
                            println!("Read file content: {} bytes", new_content.len());
                            // Preserve cursor position if possible
                            let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
                            let cursor_offset = cursor_iter.offset();
                            
                            // Update buffer content
                            buffer.set_text(&new_content);
                            
                            // Restore cursor position
                            let new_cursor_iter = buffer.iter_at_offset(cursor_offset.min(buffer.char_count()));
                            buffer.place_cursor(&new_cursor_iter);
                            
                            println!("✓ Reloaded file in SourceView: {}", path.display());
                        } else {
                            eprintln!("✗ Failed to read updated file: {}", path.display());
                        }
                    }
                    // Also try TextView
                    else if let Some(text_view) = child.downcast_ref::<gtk4::TextView>() {
                        println!("Child is TextView");
                        let buffer = text_view.buffer();
                        
                        // Read the updated file content
                        if let Ok(new_content) = fs::read_to_string(path) {
                            println!("Read file content: {} bytes", new_content.len());
                            // Preserve cursor position if possible
                            let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());
                            let cursor_offset = cursor_iter.offset();
                            
                            // Update buffer content
                            buffer.set_text(&new_content);
                            
                            // Restore cursor position
                            let new_cursor_iter = buffer.iter_at_offset(cursor_offset.min(buffer.char_count()));
                            buffer.place_cursor(&new_cursor_iter);
                            
                            println!("✓ Reloaded file in TextView: {}", path.display());
                        } else {
                            eprintln!("✗ Failed to read updated file: {}", path.display());
                        }
                    } else {
                        eprintln!("✗ Child widget is neither TextView nor SourceView for: {}", path.display());
                        eprintln!("  Widget type: {:?}", child.type_());
                    }
                } else {
                    eprintln!("✗ ScrolledWindow has no child for: {}", path.display());
                }
            } else {
                eprintln!("✗ Page is not a ScrolledWindow for: {}", path.display());
            }
        } else {
            eprintln!("✗ Could not get page {} for: {}", page_num, path.display());
        }
    } else {
        println!("File is not currently open: {}", path.display());
    }
}

/// Replace text in an open buffer (without saving to disk)
fn replace_in_buffer(
    editor_notebook: &gtk::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
    path: &Path,
    line: usize,
    col: usize,
    needle: &str,
    replace_text: &str,
    case_sensitive: bool,
) -> Result<(), String> {
    // Find the tab with this file
    let file_path_map = file_path_manager.borrow();
    let mut target_page_num: Option<u32> = None;
    
    for (page_num, tab_path) in file_path_map.iter() {
        if tab_path == path {
            target_page_num = Some(*page_num);
            break;
        }
    }
    
    drop(file_path_map);
    
    let page_num = target_page_num.ok_or_else(|| format!("File not open: {}", path.display()))?;
    
    // Get the page widget
    let page = editor_notebook.nth_page(Some(page_num))
        .ok_or_else(|| "Could not get page".to_string())?;
    
    let scrolled = page.downcast_ref::<gtk4::ScrolledWindow>()
        .ok_or_else(|| "Page is not a ScrolledWindow".to_string())?;
    
    let child = scrolled.child()
        .ok_or_else(|| "ScrolledWindow has no child".to_string())?;
    
    // Try SourceView first
    if let Some(source_view) = child.downcast_ref::<sourceview5::View>() {
        let buffer = source_view.buffer();
        
        // Get start iterator at the beginning of the line
        let mut start_iter = buffer.start_iter();
        start_iter.set_line(line.saturating_sub(1) as i32);
        
        // Get the line text to verify the match
        let line_end = start_iter;
        let mut line_end = line_end;
        if !line_end.ends_line() {
            line_end.forward_to_line_end();
        }
        let line_text = buffer.text(&start_iter, &line_end, false).to_string();
        
        // Check if needle exists at the EXACT expected column (not searching)
        let col_offset = col.saturating_sub(1);
        if col_offset >= line_text.len() {
            return Err(format!("Column {} is beyond line length {} at line {}", col, line_text.len(), line));
        }
        
        let match_end = col_offset + needle.len();
        if match_end > line_text.len() {
            return Err(format!("Match would extend beyond line end at line {} col {}", line, col));
        }
        
        let actual_text = &line_text[col_offset..match_end];
        let matches = if case_sensitive {
            actual_text == needle
        } else {
            actual_text.to_lowercase() == needle.to_lowercase()
        };
        
        if !matches {
            return Err(format!("Text '{}' not found at exact position line {} col {} (found '{}')", needle, line, col, actual_text));
        }
        
        // Position the iterators at the exact match
        start_iter.set_line_offset(col_offset as i32);
        let mut end_iter = start_iter;
        end_iter.forward_chars(needle.chars().count() as i32);
        
        // Perform the replacement
        buffer.delete(&mut start_iter, &mut end_iter);
        buffer.insert(&mut start_iter, replace_text);
        
        return Ok(());
    }
    
    // Try TextView
    if let Some(text_view) = child.downcast_ref::<gtk4::TextView>() {
        let buffer = text_view.buffer();
        
        // Get start iterator at the beginning of the line
        let mut start_iter = buffer.start_iter();
        start_iter.set_line(line.saturating_sub(1) as i32);
        
        // Get the line text to verify the match
        let line_end = start_iter;
        let mut line_end = line_end;
        if !line_end.ends_line() {
            line_end.forward_to_line_end();
        }
        let line_text = buffer.text(&start_iter, &line_end, false).to_string();
        
        // Check if needle exists at the EXACT expected column (not searching)
        let col_offset = col.saturating_sub(1);
        if col_offset >= line_text.len() {
            return Err(format!("Column {} is beyond line length {} at line {}", col, line_text.len(), line));
        }
        
        let match_end = col_offset + needle.len();
        if match_end > line_text.len() {
            return Err(format!("Match would extend beyond line end at line {} col {}", line, col));
        }
        
        let actual_text = &line_text[col_offset..match_end];
        let matches = if case_sensitive {
            actual_text == needle
        } else {
            actual_text.to_lowercase() == needle.to_lowercase()
        };
        
        if !matches {
            return Err(format!("Text '{}' not found at exact position line {} col {} (found '{}')", needle, line, col, actual_text));
        }
        
        // Position the iterators at the exact match
        start_iter.set_line_offset(col_offset as i32);
        let mut end_iter = start_iter;
        end_iter.forward_chars(needle.chars().count() as i32);
        
        // Perform the replacement
        buffer.delete(&mut start_iter, &mut end_iter);
        buffer.insert(&mut start_iter, replace_text);
        
        return Ok(());
    }
    
    Err("Widget is neither TextView nor SourceView".to_string())
}

/// Replace a single occurrence in a file
#[allow(dead_code)]
fn perform_file_replace(
    path: &Path,
    line: usize,
    col: usize,
    needle: &str,
    replace_text: &str,
    case_sensitive: bool,
) -> Result<(), std::io::Error> {
    let content = fs::read_to_string(path)?;
    let lines: Vec<&str> = content.lines().collect();
    
    if line == 0 || line > lines.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Line out of range"));
    }
    
    let target_line = lines[line - 1];
    let col_index = col.saturating_sub(1);
    
    // Verify the match exists at this position
    let match_end = col_index + needle.len();
    if match_end > target_line.len() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Column out of range"));
    }
    
    let actual_text = &target_line[col_index..match_end];
    let matches = if case_sensitive {
        actual_text == needle
    } else {
        actual_text.to_lowercase() == needle.to_lowercase()
    };
    
    if !matches {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, "Text doesn't match at position"));
    }
    
    // Perform the replacement
    let new_line = format!(
        "{}{}{}",
        &target_line[..col_index],
        replace_text,
        &target_line[match_end..]
    );
    
    // Build new content with the replaced line
    let mut new_content = String::new();
    let target_line_index = line - 1;
    for (i, line_str) in lines.iter().enumerate() {
        if i == target_line_index {
            new_content.push_str(&new_line);
        } else {
            new_content.push_str(line_str);
        }
        if i < lines.len() - 1 || content.ends_with('\n') {
            new_content.push('\n');
        }
    }
    
    fs::write(path, new_content)?;
    
    Ok(())
}

/// Replace all occurrences in a file
#[allow(dead_code)]
fn perform_file_replace_all(
    path: &Path,
    matches: &[(usize, usize, String, bool)], // (line, col, needle, case_sensitive)
    replace_text: &str,
) -> Result<usize, std::io::Error> {
    let content = fs::read_to_string(path)?;
    let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    
    // Sort matches by line and column in reverse order to maintain offsets
    let mut sorted_matches = matches.to_vec();
    sorted_matches.sort_by(|a, b| {
        match b.0.cmp(&a.0) {
            std::cmp::Ordering::Equal => b.1.cmp(&a.1),
            other => other,
        }
    });
    
    let mut replaced = 0;
    
    for (line, col, needle, case_sensitive) in sorted_matches {
        if line == 0 || line > lines.len() {
            continue;
        }
        
        let line_index = line - 1;
        let col_index = col.saturating_sub(1);
        let target_line = &lines[line_index];
        
        let match_end = col_index + needle.len();
        if match_end > target_line.len() {
            continue;
        }
        
        let actual_text = &target_line[col_index..match_end];
        let matches = if case_sensitive {
            actual_text == needle
        } else {
            actual_text.to_lowercase() == needle.to_lowercase()
        };
        
        if !matches {
            continue;
        }
        
        // Perform the replacement
        let new_line = format!(
            "{}{}{}",
            &target_line[..col_index],
            replace_text,
            &target_line[match_end..]
        );
        
        lines[line_index] = new_line;
        replaced += 1;
    }
    
    let new_content = lines.join("\n");
    fs::write(path, new_content)?;
    
    Ok(replaced)
}

pub fn show_global_search_dialog(
    parent_window: &impl IsA<gtk::ApplicationWindow>,
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
    
    // Create new dialog
    let dialog = gtk::Dialog::builder()
        .transient_for(parent_window.as_ref().upcast_ref::<gtk4::Window>())
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
    
    // Create overlay for case sensitivity toggle button
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&search_scroller));
    
    // Toggle buttons overlaid on top-right
    let toggle_box = GtkBox::new(Orientation::Horizontal, 2);
    toggle_box.set_halign(gtk::Align::End);
    toggle_box.set_valign(gtk::Align::Start);
    toggle_box.set_margin_end(4);
    toggle_box.set_margin_top(4);
    
    let case_toggle = gtk::ToggleButton::with_label("Aa");
    case_toggle.set_tooltip_text(Some("Match case"));
    case_toggle.add_css_class("case-toggle-button");
    
    let whole_word_toggle = gtk::ToggleButton::with_label("Ab");
    whole_word_toggle.set_tooltip_text(Some("Match whole word"));
    whole_word_toggle.add_css_class("case-toggle-button");
    
    toggle_box.append(&case_toggle);
    toggle_box.append(&whole_word_toggle);
    overlay.add_overlay(&toggle_box);
    
    // Replace text area
    let replace_buffer = TextBuffer::new(None);
    let replace_text_view = TextView::with_buffer(&replace_buffer);
    replace_text_view.set_wrap_mode(gtk::WrapMode::Word);
    replace_text_view.set_accepts_tab(false);
    replace_text_view.add_css_class("monospace");
    
    let replace_scroller = ScrolledWindow::builder()
        .child(&replace_text_view)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .height_request(80)
        .hexpand(true)
        .build();
    replace_scroller.set_margin_bottom(8);
    
    // Controls row
    let controls = GtkBox::new(Orientation::Horizontal, 8);
    controls.set_margin_bottom(8);
    controls.append(&overlay);
    
    let controls_right = GtkBox::new(Orientation::Vertical, 4);
    let search_btn = Button::with_label("Search");
    search_btn.add_css_class("suggested-action");
    controls_right.append(&search_btn);
    controls.append(&controls_right);
    
    // Replace controls row
    let replace_controls = GtkBox::new(Orientation::Horizontal, 8);
    replace_controls.set_margin_bottom(8);
    replace_controls.append(&replace_scroller);
    
    let replace_controls_right = GtkBox::new(Orientation::Horizontal, 4);
    let replace_btn = Button::with_label("Replace");
    replace_btn.set_tooltip_text(Some("Replace selected match"));
    let replace_all_btn = Button::with_label("Replace All");
    replace_all_btn.set_tooltip_text(Some("Replace all matches in all files"));
    replace_all_btn.add_css_class("destructive-action");
    replace_controls_right.append(&replace_btn);
    replace_controls_right.append(&replace_all_btn);
    replace_controls.append(&replace_controls_right);

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
    vbox.append(&replace_controls);
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
    let start_search: Rc<dyn Fn(String, bool, bool)> = {
        let status = status.clone();
        let results_list = results_list.clone();
        let result_count_clone = result_count.clone();
        let receiver_rc_clone = receiver_rc.clone();
        let sender_rc_clone = sender_rc.clone();
        let current_dir_clone = current_dir.clone();
        let editor_notebook_clone = editor_notebook.clone();
        let file_path_manager_clone = file_path_manager.clone();
        Rc::new(move |query: String, case_sensitive: bool, whole_word: bool| {
            // Clear previous results
            while let Some(child) = results_list.first_child() { results_list.remove(&child); }
            *result_count_clone.borrow_mut() = 0;
            status.set_text("Searching...");
            
            // Create new channel for this search (this will automatically drop the old receiver,
            // causing the old timer to finish naturally)
            let (sender, receiver) = std::sync::mpsc::channel::<Option<SearchResult>>();
            *sender_rc_clone.borrow_mut() = Some(sender.clone());
            *receiver_rc_clone.borrow_mut() = Some(receiver);

            let root = current_dir_clone.borrow().clone();
            
            // Get open file buffers content (must be done in main thread)
            let mut open_files_content: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
            let file_path_map = file_path_manager_clone.borrow();
            for (page_num, path) in file_path_map.iter() {
                if let Some(page) = editor_notebook_clone.nth_page(Some(*page_num)) {
                    if let Some(scrolled) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                        if let Some(child) = scrolled.child() {
                            let content = if let Some(source_view) = child.downcast_ref::<sourceview5::View>() {
                                let buffer = source_view.buffer();
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
                            } else if let Some(text_view) = child.downcast_ref::<gtk4::TextView>() {
                                let buffer = text_view.buffer();
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
                            } else {
                                continue;
                            };
                            open_files_content.insert(path.clone(), content);
                        }
                    }
                }
            }
            drop(file_path_map);
            
            // Start search in background thread
            std::thread::spawn(move || {
                let mut files = Vec::with_capacity(2048);
                walk_dir_recursive(&root, &mut files, 20000);
                let mut found = 0usize;
                for p in files {
                    // Check if we have buffer content for this file
                    let results = if let Some(content) = open_files_content.get(&p) {
                        // Search in buffer content
                        search_in_content(&p, content, &query, case_sensitive, whole_word)
                    } else {
                        // Search in file on disk
                        search_file(&p, &query, case_sensitive, whole_word, 5 * 1024 * 1024)
                    };
                    
                    for r in results {
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
            let max_results = 500usize; // Limit displayed results to prevent UI slowdown
            glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let mut finished = false;
                let mut processed = 0usize;
                let current_count = *result_count_c.borrow();
                if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                    while processed < 50 && current_count + processed < max_results {
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
                                if count >= max_results {
                                    status_c.set_text(&format!("Showing first {} results (more available)", max_results));
                                } else {
                                    status_c.set_text(&format!("Found {} result{}", count, if count == 1 { "" } else { "s" }));
                                }
                                
                                // Select the first result automatically
                                if count == 1 {
                                    results_list_c.select_row(Some(&row));
                                    // Activate the row to open the file and jump to the occurrence
                                    row.activate();
                                }
                                
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
                
                // Check if we hit the limit
                let count = *result_count_c.borrow();
                if count >= max_results {
                    // Drain any remaining results from channel without processing
                    if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                        while rx.try_recv().is_ok() {}
                    }
                    finished = true;
                }
                
                if finished {
                    let count = *result_count_c.borrow();
                    if count == 0 {
                        status_c.set_text("No results found");
                    } else if count >= max_results {
                        status_c.set_text(&format!("Showing first {} results (search stopped at limit)", max_results));
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
    let case_toggle_weak = case_toggle.downgrade();
    let whole_word_toggle_weak = whole_word_toggle.downgrade();
    search_btn.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak.upgrade(), case_toggle_weak.upgrade(), whole_word_toggle_weak.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            (cb_clone_btn)(text, case_toggle.is_active(), whole_word_toggle.is_active());
        }
    });
    
    // Trigger search when toggle buttons are clicked
    let cb_clone_case = start_search.clone();
    let buffer_weak_case = search_buffer.downgrade();
    let case_toggle_weak_case = case_toggle.downgrade();
    let whole_word_toggle_weak_case = whole_word_toggle.downgrade();
    case_toggle.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_case.upgrade(), case_toggle_weak_case.upgrade(), whole_word_toggle_weak_case.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            if !text.trim().is_empty() {
                (cb_clone_case)(text, case_toggle.is_active(), whole_word_toggle.is_active());
            }
        }
    });
    
    let cb_clone_word = start_search.clone();
    let buffer_weak_word = search_buffer.downgrade();
    let case_toggle_weak_word = case_toggle.downgrade();
    let whole_word_toggle_weak_word = whole_word_toggle.downgrade();
    whole_word_toggle.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_word.upgrade(), case_toggle_weak_word.upgrade(), whole_word_toggle_weak_word.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            if !text.trim().is_empty() {
                (cb_clone_word)(text, case_toggle.is_active(), whole_word_toggle.is_active());
            }
        }
    });
    
    // Add keyboard handling to TextView: Enter to search, Shift+Enter for line break
    let key_controller = EventControllerKey::new();
    let cb_clone_enter = start_search.clone();
    let buffer_weak_key = search_buffer.downgrade();
    let case_toggle_weak_key = case_toggle.downgrade();
    let whole_word_toggle_weak_key = whole_word_toggle.downgrade();
    key_controller.connect_key_pressed(move |_controller, key, _code, modifier| {
        if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
            if modifier.contains(gdk::ModifierType::SHIFT_MASK) {
                // Shift+Enter: allow default behavior (insert line break)
                glib::Propagation::Proceed
            } else {
                // Enter: trigger search
                if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_key.upgrade(), case_toggle_weak_key.upgrade(), whole_word_toggle_weak_key.upgrade()) {
                    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
                    (cb_clone_enter)(text, case_toggle.is_active(), whole_word_toggle.is_active());
                }
                glib::Propagation::Stop
            }
        } else {
            glib::Propagation::Proceed
        }
    });
    search_text_view.add_controller(key_controller);

    // Replace button handler - replaces the selected result
    let replace_buffer_weak = replace_buffer.downgrade();
    let results_list_clone = results_list.clone();
    let editor_notebook_for_replace = editor_notebook.clone();
    let file_path_manager_for_replace = file_path_manager.clone();
    let search_buffer_for_refresh = search_buffer.clone();
    let case_toggle_for_refresh = case_toggle.clone();
    let whole_word_toggle_for_refresh = whole_word_toggle.clone();
    let start_search_for_refresh = start_search.clone();
    replace_btn.connect_clicked(move |_| {
        if let Some(replace_buffer) = replace_buffer_weak.upgrade() {
            let replace_text = replace_buffer.text(&replace_buffer.start_iter(), &replace_buffer.end_iter(), false).to_string();
            
            // Get the selected row
            if let Some(row) = results_list_clone.selected_row() {
                // Get the index of the current row before removing it
                let current_index = row.index();
                
                if let Some(child) = row.child() {
                    if let Some(vbox) = child.downcast_ref::<GtkBox>() {
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
                                        let case_sensitive: bool = parts[4].parse().unwrap_or(false);
                                        
                                        // Perform the replacement in buffer only
                                        match replace_in_buffer(
                                            &editor_notebook_for_replace,
                                            &file_path_manager_for_replace,
                                            &path,
                                            line,
                                            col,
                                            &needle,
                                            &replace_text,
                                            case_sensitive
                                        ) {
                                            Ok(_) => {
                                                // Remove the result from the list after successful replacement
                                                results_list_clone.remove(&row);
                                                
                                                // Select the next row (which now has the same index as the removed row)
                                                if let Some(next_row) = results_list_clone.row_at_index(current_index) {
                                                    results_list_clone.select_row(Some(&next_row));
                                                    // Trigger the activation to open the file and jump to the position
                                                    next_row.activate();
                                                }
                                                
                                                // Refresh the search to update results
                                                let search_text = search_buffer_for_refresh.text(&search_buffer_for_refresh.start_iter(), &search_buffer_for_refresh.end_iter(), false).to_string();
                                                if !search_text.trim().is_empty() {
                                                    (start_search_for_refresh)(search_text, case_toggle_for_refresh.is_active(), whole_word_toggle_for_refresh.is_active());
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to replace in {}: {}", path.display(), e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Replace All button handler - replaces all matches in all files (async in thread)
    let replace_buffer_weak_all = replace_buffer.downgrade();
    let results_list_clone_all = results_list.clone();
    let status_clone_for_replace_all = status.clone();
    let editor_notebook_for_replace_all = editor_notebook.clone();
    let file_path_manager_for_replace_all = file_path_manager.clone();
    let parent_window_for_replace_all = parent_window.clone();
    let active_tab_path_for_replace_all = active_tab_path.clone();
    let save_button_for_replace_all = save_button.clone();
    let save_as_button_for_replace_all = save_as_button.clone();
    let file_list_box_for_replace_all = file_list_box.clone();
    let current_dir_for_replace_all = current_dir.clone();
    replace_all_btn.connect_clicked(move |_| {
        if let Some(replace_buffer) = replace_buffer_weak_all.upgrade() {
            let replace_text = replace_buffer.text(&replace_buffer.start_iter(), &replace_buffer.end_iter(), false).to_string();
            
            // Collect all results
            let mut replacements = Vec::new();
            let mut index = 0;
            while let Some(row) = results_list_clone_all.row_at_index(index) {
                if let Some(child) = row.child() {
                    if let Some(vbox) = child.downcast_ref::<GtkBox>() {
                        if let Some(first_child) = vbox.first_child() {
                            if let Some(data_label) = first_child.downcast_ref::<Label>() {
                                if let Some(tt) = data_label.tooltip_text() {
                                    let parts: Vec<&str> = tt.splitn(5, '|').collect();
                                    if parts.len() >= 5 {
                                        let path = PathBuf::from(parts[0]);
                                        let line: usize = parts[1].parse().unwrap_or(1);
                                        let col: usize = parts[2].parse().unwrap_or(1);
                                        let needle = parts[3].to_string();
                                        let case_sensitive: bool = parts[4].parse().unwrap_or(false);
                                        replacements.push((path, line, col, needle, case_sensitive));
                                    }
                                }
                            }
                        }
                    }
                }
                index += 1;
            }
            
            println!("Replace All (dialog): collected {} replacements from UI", replacements.len());
            
            if replacements.is_empty() {
                status_clone_for_replace_all.set_text("No results to replace");
                return;
            }
            
            status_clone_for_replace_all.set_text("Processing replacements...");
            
            // Group replacements by file in background thread
            let editor_notebook_clone = editor_notebook_for_replace_all.clone();
            let file_path_manager_clone = file_path_manager_for_replace_all.clone();
            let parent_window_clone = parent_window_for_replace_all.clone();
            let active_tab_path_clone = active_tab_path_for_replace_all.clone();
            let save_button_clone = save_button_for_replace_all.clone();
            let save_as_button_clone = save_as_button_for_replace_all.clone();
            let file_list_box_clone = file_list_box_for_replace_all.clone();
            let current_dir_clone = current_dir_for_replace_all.clone();
            let results_list_for_clear = results_list_clone_all.clone();
            let status_for_update = status_clone_for_replace_all.clone();
            
            let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, Vec<(usize, usize, String, bool)>)>();
            let (done_tx, done_rx) = std::sync::mpsc::channel::<usize>();
            
            // Spawn thread to group replacements and read file contents
            std::thread::spawn(move || {
                let mut files_map: std::collections::HashMap<PathBuf, Vec<(usize, usize, String, bool)>> = std::collections::HashMap::new();
                for (path, line, col, needle, case_sensitive) in replacements {
                    files_map.entry(path).or_insert_with(Vec::new).push((line, col, needle, case_sensitive));
                }
                
                // Send each file's replacements to main thread
                for (path, matches) in files_map {
                    let _ = tx.send((path, matches));
                }
                
                // Signal completion
                let _ = done_tx.send(0);
            });
            
            // Process replacements on main thread
            let total_replaced = Rc::new(RefCell::new(0usize));
            let total_replaced_clone = total_replaced.clone();
            let thread_done = Rc::new(RefCell::new(false));
            let thread_done_clone = thread_done.clone();
            
            glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                // Check if thread signaled completion (only once)
                if !*thread_done_clone.borrow() && done_rx.try_recv().is_ok() {
                    *thread_done_clone.borrow_mut() = true;
                    println!("Dialog: Background thread completed");
                }
                
                // Process a batch of file replacements
                let mut processed = 0;
                while processed < 5 {
                    match rx.try_recv() {
                        Ok((path, mut matches)) => {
                            println!("Dialog: Processing {} matches in file: {}", matches.len(), path.display());
                            
                            // Sort matches in reverse order (bottom to top) to preserve positions
                            matches.sort_by(|a, b| {
                                b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1))
                            });
                            
                            // Open file if not already open
                            let file_path_map = file_path_manager_clone.borrow();
                            let is_open = file_path_map.values().any(|p| p == &path);
                            drop(file_path_map);
                            
                            if !is_open {
                                // Open the file first
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let mime = mime_guess::from_path(&path).first_or_octet_stream();
                                    crate::handlers::open_or_focus_tab(
                                        &editor_notebook_clone,
                                        &path,
                                        &content,
                                        &active_tab_path_clone,
                                        &file_path_manager_clone,
                                        &save_button_clone,
                                        &save_as_button_clone,
                                        &mime,
                                        &parent_window_clone,
                                        &file_list_box_clone,
                                        &current_dir_clone,
                                        None,
                                    );
                                } else {
                                    eprintln!("Failed to read file {}", path.display());
                                    processed += 1;
                                    continue;
                                }
                            }
                            
                            // Replace all occurrences in the buffer for this file (bottom to top)
                            for (line, col, needle, case_sensitive) in matches {
                                match replace_in_buffer(
                                    &editor_notebook_clone,
                                    &file_path_manager_clone,
                                    &path,
                                    line,
                                    col,
                                    &needle,
                                    &replace_text,
                                    case_sensitive
                                ) {
                                    Ok(_) => {
                                        *total_replaced_clone.borrow_mut() += 1;
                                        println!("Dialog: Successfully replaced at {}:{} in {}", line, col, path.display());
                                    }
                                    Err(e) => eprintln!("Dialog: Failed to replace at {}:{} in {}: {}", line, col, path.display(), e),
                                }
                            }
                            
                            processed += 1;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    }
                }
                
                // Check if we're completely done (thread finished AND no more data)
                if *thread_done_clone.borrow() && rx.try_recv().is_err() {
                    // Clear all results from the list
                    while let Some(row) = results_list_for_clear.row_at_index(0) {
                        results_list_for_clear.remove(&row);
                    }
                    
                    let total = *total_replaced_clone.borrow();
                    status_for_update.set_text(&format!("Replaced {} occurrence{}", 
                        total, 
                        if total == 1 { "" } else { "s" }
                    ));
                    return glib::ControlFlow::Break;
                }
                
                glib::ControlFlow::Continue
            });
        }
    });
    
    // Clone search elements for refresh after Replace All
    let search_buffer_for_refresh_all = search_buffer.clone();
    let case_toggle_for_refresh_all = case_toggle.clone();
    let whole_word_toggle_for_refresh_all = whole_word_toggle.clone();
    let start_search_for_refresh_all = start_search.clone();
    
    // Connect to refresh search after Replace All completes
    let status_for_watch = status.clone();
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        let status_text = status_for_watch.text();
        if status_text.starts_with("Replaced") && status_text.contains("occurrence") {
            // Replace All just completed, refresh search
            let search_text = search_buffer_for_refresh_all.text(&search_buffer_for_refresh_all.start_iter(), &search_buffer_for_refresh_all.end_iter(), false).to_string();
            if !search_text.trim().is_empty() {
                (start_search_for_refresh_all)(search_text, case_toggle_for_refresh_all.is_active(), whole_word_toggle_for_refresh_all.is_active());
            }
            glib::ControlFlow::Break
        } else if status_text.starts_with("Processing replacements") {
            // Still processing, keep watching
            glib::ControlFlow::Continue
        } else {
            // Not in replace mode, stop watching
            glib::ControlFlow::Break
        }
    });

    dialog.add_button("Close", gtk::ResponseType::Close);
    dialog.connect_response(|d, _| {
        d.set_visible(false);
    });
    dialog.present();
}

/// Creates the global search panel UI (for embedding in the activity bar sidebar)
/// Returns the panel container that can be added to the sidebar stack
pub fn create_global_search_panel(
    parent_window: &impl IsA<gtk::ApplicationWindow>,
    current_dir: &Rc<RefCell<PathBuf>>,
    editor_notebook: &gtk::Notebook,
    file_path_manager: &Rc<RefCell<std::collections::HashMap<u32, PathBuf>>>,
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
    save_button: &gtk::Button,
    save_as_button: &gtk::Button,
    file_list_box: &gtk::ListBox,
) -> GtkBox {
    // Create the template-based panel
    let panel = SearchPanel::new();
    
    // Get references to widgets
    let search_buffer = panel.search_buffer();
    let search_text_view = panel.imp().search_text_view.get();  // Get the TextView widget
    let replace_buffer = panel.replace_buffer();
    let case_toggle = panel.case_toggle();
    let whole_word_toggle = panel.whole_word_toggle();
    let search_btn = panel.search_btn();
    let replace_btn = panel.replace_btn();
    let replace_all_btn = panel.replace_all_btn();
    let status = panel.status_label();
    let results_list = panel.results_list();
    
    // Restore search state from settings
    let settings = crate::settings::get_settings();
    case_toggle.set_active(settings.get_search_case_sensitive());
    whole_word_toggle.set_active(settings.get_search_whole_word());
    let saved_query = settings.get_search_query();
    if !saved_query.is_empty() {
        search_buffer.set_text(&saved_query);
    }

    // Channel for results - will be recreated for each search
    let sender_rc: Rc<RefCell<Option<std::sync::mpsc::Sender<Option<SearchResult>>>>> = Rc::new(RefCell::new(None));
    let receiver_rc: Rc<RefCell<Option<std::sync::mpsc::Receiver<Option<SearchResult>>>>> = Rc::new(RefCell::new(None));

    // Clone status label before it's moved into closures
    let status_for_replace = status.clone();

    // Open result handler (row activation)
    let parent_window_c = parent_window.clone();
    let editor_notebook_c = editor_notebook.clone();
    let file_path_manager_c = file_path_manager.clone();
    let active_tab_path_c = active_tab_path.clone();
    let save_button_c = save_button.clone();
    let save_as_button_c = save_as_button.clone();
    let file_list_box_c = file_list_box.clone();
    let current_dir_c = current_dir.clone();

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
                                    }
                                } else {
                                    eprintln!("Not a text file or couldn't open: {}", path.display());
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Search logic closure
    let start_search = {
        let status_c = status.clone();
        let results_list_c = results_list.clone();
        let sender_rc_clone = sender_rc.clone();
        let receiver_rc_clone = receiver_rc.clone();
        let current_dir_c = current_dir.clone();
        let editor_notebook_c = editor_notebook.clone();
        let file_path_manager_c = file_path_manager.clone();
        
        move |needle: String, case_sensitive: bool, whole_word: bool| {
            // Clear previous results
            while let Some(row) = results_list_c.row_at_index(0) {
                results_list_c.remove(&row);
            }
            
            if needle.trim().is_empty() {
                status_c.set_text("Enter text to search");
                return;
            }
            
            status_c.set_text("Searching...");
            
            // Get search folder (use current directory)
            let folder_to_search = current_dir_c.borrow().clone();
            
            // Create new channel for this search
            let (sender, receiver) = std::sync::mpsc::channel();
            *sender_rc_clone.borrow_mut() = Some(sender.clone());
            *receiver_rc_clone.borrow_mut() = Some(receiver);
            
            // Get open file buffers content (must be done in main thread)
            let mut open_files_content: std::collections::HashMap<PathBuf, String> = std::collections::HashMap::new();
            let file_path_map = file_path_manager_c.borrow();
            for (page_num, path) in file_path_map.iter() {
                if let Some(page) = editor_notebook_c.nth_page(Some(*page_num)) {
                    if let Some(scrolled) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                        if let Some(child) = scrolled.child() {
                            let content = if let Some(source_view) = child.downcast_ref::<sourceview5::View>() {
                                let buffer = source_view.buffer();
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
                            } else if let Some(text_view) = child.downcast_ref::<gtk4::TextView>() {
                                let buffer = text_view.buffer();
                                buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string()
                            } else {
                                continue;
                            };
                            open_files_content.insert(path.clone(), content);
                        }
                    }
                }
            }
            drop(file_path_map);
            
            // Spawn search thread
            std::thread::spawn(move || {
                let mut files = Vec::new();
                walk_dir_recursive(&folder_to_search, &mut files, 10000);
                
                for file_path in files {
                    // Check if we have buffer content for this file
                    let results = if let Some(content) = open_files_content.get(&file_path) {
                        // Search in buffer content
                        search_in_content(&file_path, content, &needle, case_sensitive, whole_word)
                    } else {
                        // Search in file on disk
                        search_file(&file_path, &needle, case_sensitive, whole_word, 1_000_000)
                    };
                    
                    for sr in results {
                        let _ = sender.send(Some(sr));
                    }
                }
                let _ = sender.send(None); // Signal completion
            });
            
            // Set up UI update timer
            let result_count_clone = Rc::new(RefCell::new(0usize));
            let results_list_c = results_list_c.clone();
            let status_c = status.clone();
            let receiver_rc_c = receiver_rc_clone.clone();
            let result_count_c = result_count_clone.clone();
            let max_results = 500usize; // Limit displayed results to prevent UI slowdown
            glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
                let mut finished = false;
                let mut processed = 0usize;
                let current_count = *result_count_c.borrow();
                if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                    while processed < 50 && current_count + processed < max_results {
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
                                
                                // Preview text (with proper trimming)
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
                                if count >= max_results {
                                    status_c.set_text(&format!("Showing first {} results (more available)", max_results));
                                } else {
                                    status_c.set_text(&format!("Found {} result{}", count, if count == 1 { "" } else { "s" }));
                                }
                                
                                // Select the first result automatically
                                if count == 1 {
                                    results_list_c.select_row(Some(&row));
                                    // Activate the row to open the file and jump to the occurrence
                                    row.activate();
                                }
                                
                                processed += 1;
                            }
                            Ok(None) => { finished = true; break; }
                            Err(std::sync::mpsc::TryRecvError::Empty) => break,
                            Err(std::sync::mpsc::TryRecvError::Disconnected) => { finished = true; break; }
                        }
                    }
                }
                
                // Check if we hit the limit
                let count = *result_count_c.borrow();
                if count >= max_results {
                    // Drain any remaining results from channel without processing
                    if let Some(rx) = receiver_rc_c.borrow().as_ref() {
                        while rx.try_recv().is_ok() {}
                    }
                    finished = true;
                }
                
                if finished {
                    let count = *result_count_c.borrow();
                    if count == 0 {
                        status_c.set_text("No results found");
                    } else if count >= max_results {
                        status_c.set_text(&format!("Showing first {} results (search stopped at limit)", max_results));
                    } else {
                        status_c.set_text(&format!("Search complete - {} result{} found", count, if count == 1 { "" } else { "s" }));
                    }
                    glib::ControlFlow::Break
                } else {
                    glib::ControlFlow::Continue
                }
            });
        }
    };

    let cb_clone_btn = start_search.clone();
    let buffer_weak = search_buffer.downgrade();
    let case_toggle_weak = case_toggle.downgrade();
    let whole_word_toggle_weak = whole_word_toggle.downgrade();
    search_btn.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak.upgrade(), case_toggle_weak.upgrade(), whole_word_toggle_weak.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            (cb_clone_btn)(text, case_toggle.is_active(), whole_word_toggle.is_active());
        }
    });
    
    // Trigger search when toggle buttons are clicked
    let cb_clone_case = start_search.clone();
    let buffer_weak_case = search_buffer.downgrade();
    let case_toggle_weak_case = case_toggle.downgrade();
    let whole_word_toggle_weak_case = whole_word_toggle.downgrade();
    case_toggle.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_case.upgrade(), case_toggle_weak_case.upgrade(), whole_word_toggle_weak_case.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            if !text.trim().is_empty() {
                (cb_clone_case)(text, case_toggle.is_active(), whole_word_toggle.is_active());
            }
        }
    });
    
    let cb_clone_word = start_search.clone();
    let buffer_weak_word = search_buffer.downgrade();
    let case_toggle_weak_word = case_toggle.downgrade();
    let whole_word_toggle_weak_word = whole_word_toggle.downgrade();
    whole_word_toggle.connect_clicked(move |_| {
        if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_word.upgrade(), case_toggle_weak_word.upgrade(), whole_word_toggle_weak_word.upgrade()) {
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
            if !text.trim().is_empty() {
                (cb_clone_word)(text, case_toggle.is_active(), whole_word_toggle.is_active());
            }
        }
    });
    
    // Add keyboard handling to TextView: Enter to search, Shift+Enter for line break
    let key_controller = EventControllerKey::new();
    let cb_clone_enter = start_search.clone();
    let buffer_weak_key = search_buffer.downgrade();
    let case_toggle_weak_key = case_toggle.downgrade();
    let whole_word_toggle_weak_key = whole_word_toggle.downgrade();
    key_controller.connect_key_pressed(move |_controller, key, _code, modifier| {
        if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
            if modifier.contains(gdk::ModifierType::SHIFT_MASK) {
                // Shift+Enter: allow default behavior (insert line break)
                glib::Propagation::Proceed
            } else {
                // Enter: trigger search
                if let (Some(buffer), Some(case_toggle), Some(whole_word_toggle)) = (buffer_weak_key.upgrade(), case_toggle_weak_key.upgrade(), whole_word_toggle_weak_key.upgrade()) {
                    let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false).to_string();
                    (cb_clone_enter)(text, case_toggle.is_active(), whole_word_toggle.is_active());
                }
                glib::Propagation::Stop
            }
        } else {
            glib::Propagation::Proceed
        }
    });
    search_text_view.add_controller(key_controller);
    
    // Replace button handler - replaces the selected result
    let replace_buffer_weak = replace_buffer.downgrade();
    let results_list_clone = results_list.clone();
    let editor_notebook_for_replace = editor_notebook.clone();
    let file_path_manager_for_replace = file_path_manager.clone();
    let search_buffer_for_refresh = search_buffer.clone();
    let case_toggle_for_refresh = case_toggle.clone();
    let whole_word_toggle_for_refresh = whole_word_toggle.clone();
    let start_search_for_refresh = start_search.clone();
    replace_btn.connect_clicked(move |_| {
        if let Some(replace_buffer) = replace_buffer_weak.upgrade() {
            let replace_text = replace_buffer.text(&replace_buffer.start_iter(), &replace_buffer.end_iter(), false).to_string();
            
            // Get the selected row
            if let Some(row) = results_list_clone.selected_row() {
                // Get the index of the current row before removing it
                let current_index = row.index();
                
                if let Some(child) = row.child() {
                    if let Some(vbox) = child.downcast_ref::<GtkBox>() {
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
                                        let case_sensitive: bool = parts[4].parse().unwrap_or(false);
                                        
                                        // Perform the replacement in buffer only
                                        match replace_in_buffer(
                                            &editor_notebook_for_replace,
                                            &file_path_manager_for_replace,
                                            &path,
                                            line,
                                            col,
                                            &needle,
                                            &replace_text,
                                            case_sensitive
                                        ) {
                                            Ok(_) => {
                                                // Remove the result from the list after successful replacement
                                                results_list_clone.remove(&row);
                                                
                                                // Select the next row (which now has the same index as the removed row)
                                                if let Some(next_row) = results_list_clone.row_at_index(current_index) {
                                                    results_list_clone.select_row(Some(&next_row));
                                                    // Trigger the activation to open the file and jump to the position
                                                    next_row.activate();
                                                }
                                                
                                                // Refresh the search to update results
                                                let search_text = search_buffer_for_refresh.text(&search_buffer_for_refresh.start_iter(), &search_buffer_for_refresh.end_iter(), false).to_string();
                                                if !search_text.trim().is_empty() {
                                                    (start_search_for_refresh)(search_text, case_toggle_for_refresh.is_active(), whole_word_toggle_for_refresh.is_active());
                                                }
                                            }
                                            Err(e) => {
                                                eprintln!("Failed to replace in {}: {}", path.display(), e);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // Replace All button handler - replaces all matches in all files (async in thread)
    let replace_buffer_weak_all = replace_buffer.downgrade();
    let results_list_clone_all = results_list.clone();
    let status_clone_for_replace_all = status_for_replace.clone();
    let editor_notebook_for_replace_all = editor_notebook.clone();
    let file_path_manager_for_replace_all = file_path_manager.clone();
    let parent_window_for_replace_all = parent_window.clone();
    let active_tab_path_for_replace_all = active_tab_path.clone();
    let save_button_for_replace_all = save_button.clone();
    let save_as_button_for_replace_all = save_as_button.clone();
    let file_list_box_for_replace_all = file_list_box.clone();
    let current_dir_for_replace_all = current_dir.clone();
    let search_buffer_for_replace_all = search_buffer.clone();
    let case_toggle_for_replace_all = case_toggle.clone();
    let whole_word_toggle_for_replace_all = whole_word_toggle.clone();
    let start_search_for_replace_all = start_search.clone();
    replace_all_btn.connect_clicked(move |_| {
        if let Some(replace_buffer) = replace_buffer_weak_all.upgrade() {
            let replace_text = replace_buffer.text(&replace_buffer.start_iter(), &replace_buffer.end_iter(), false).to_string();
            
            // Collect all results
            let mut replacements = Vec::new();
            let mut index = 0;
            while let Some(row) = results_list_clone_all.row_at_index(index) {
                if let Some(child) = row.child() {
                    if let Some(vbox) = child.downcast_ref::<GtkBox>() {
                        if let Some(first_child) = vbox.first_child() {
                            if let Some(data_label) = first_child.downcast_ref::<Label>() {
                                if let Some(tt) = data_label.tooltip_text() {
                                    let parts: Vec<&str> = tt.splitn(5, '|').collect();
                                    if parts.len() >= 5 {
                                        let path = PathBuf::from(parts[0]);
                                        let line: usize = parts[1].parse().unwrap_or(1);
                                        let col: usize = parts[2].parse().unwrap_or(1);
                                        let needle = parts[3].to_string();
                                        let case_sensitive: bool = parts[4].parse().unwrap_or(false);
                                        replacements.push((path, line, col, needle, case_sensitive));
                                    }
                                }
                            }
                        }
                    }
                }
                index += 1;
            }
            
            println!("Replace All (panel): collected {} replacements from UI", replacements.len());
            
            if replacements.is_empty() {
                status_clone_for_replace_all.set_text("No results to replace");
                return;
            }
            
            status_clone_for_replace_all.set_text("Processing replacements...");
            
            // Group replacements by file in background thread
            let editor_notebook_clone = editor_notebook_for_replace_all.clone();
            let file_path_manager_clone = file_path_manager_for_replace_all.clone();
            let parent_window_clone = parent_window_for_replace_all.clone();
            let active_tab_path_clone = active_tab_path_for_replace_all.clone();
            let save_button_clone = save_button_for_replace_all.clone();
            let save_as_button_clone = save_as_button_for_replace_all.clone();
            let file_list_box_clone = file_list_box_for_replace_all.clone();
            let current_dir_clone = current_dir_for_replace_all.clone();
            let results_list_for_clear = results_list_clone_all.clone();
            let status_for_update = status_clone_for_replace_all.clone();
            
            let (tx, rx) = std::sync::mpsc::channel::<(PathBuf, Vec<(usize, usize, String, bool)>)>();
            let (done_tx, done_rx) = std::sync::mpsc::channel::<usize>();
            
            // Spawn thread to group replacements and read file contents
            std::thread::spawn(move || {
                let mut files_map: std::collections::HashMap<PathBuf, Vec<(usize, usize, String, bool)>> = std::collections::HashMap::new();
                for (path, line, col, needle, case_sensitive) in replacements {
                    files_map.entry(path).or_insert_with(Vec::new).push((line, col, needle, case_sensitive));
                }
                
                // Send each file's replacements to main thread
                for (path, matches) in files_map {
                    let _ = tx.send((path, matches));
                }
                
                // Signal completion
                let _ = done_tx.send(0);
            });
            
            // Process replacements on main thread
            let total_replaced = Rc::new(RefCell::new(0usize));
            let total_replaced_clone = total_replaced.clone();
            let thread_done = Rc::new(RefCell::new(false));
            let thread_done_clone = thread_done.clone();
            
            // Clone search elements before moving into timeout closure
            let search_buffer_timeout = search_buffer_for_replace_all.clone();
            let case_toggle_timeout = case_toggle_for_replace_all.clone();
            let whole_word_toggle_timeout = whole_word_toggle_for_replace_all.clone();
            let start_search_timeout = start_search_for_replace_all.clone();
            
            glib::timeout_add_local(std::time::Duration::from_millis(10), move || {
                // Check if thread signaled completion (only once)
                if !*thread_done_clone.borrow() && done_rx.try_recv().is_ok() {
                    *thread_done_clone.borrow_mut() = true;
                    println!("Panel: Background thread completed");
                }
                
                // Process a batch of file replacements
                let mut processed = 0;
                while processed < 5 {
                    match rx.try_recv() {
                        Ok((path, mut matches)) => {
                            println!("Panel: Processing {} matches in file: {}", matches.len(), path.display());
                            
                            // Sort matches in reverse order (bottom to top) to preserve positions
                            matches.sort_by(|a, b| {
                                b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1))
                            });
                            
                            // Open file if not already open
                            let file_path_map = file_path_manager_clone.borrow();
                            let is_open = file_path_map.values().any(|p| p == &path);
                            drop(file_path_map);
                            
                            if !is_open {
                                // Open the file first
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    let mime = mime_guess::from_path(&path).first_or_octet_stream();
                                    crate::handlers::open_or_focus_tab(
                                        &editor_notebook_clone,
                                        &path,
                                        &content,
                                        &active_tab_path_clone,
                                        &file_path_manager_clone,
                                        &save_button_clone,
                                        &save_as_button_clone,
                                        &mime,
                                        &parent_window_clone,
                                        &file_list_box_clone,
                                        &current_dir_clone,
                                        None,
                                    );
                                } else {
                                    eprintln!("Failed to read file {}", path.display());
                                    processed += 1;
                                    continue;
                                }
                            }
                            
                            // Replace all occurrences in the buffer for this file (bottom to top)
                            for (line, col, needle, case_sensitive) in matches {
                                match replace_in_buffer(
                                    &editor_notebook_clone,
                                    &file_path_manager_clone,
                                    &path,
                                    line,
                                    col,
                                    &needle,
                                    &replace_text,
                                    case_sensitive
                                ) {
                                    Ok(_) => {
                                        *total_replaced_clone.borrow_mut() += 1;
                                        println!("Panel: Successfully replaced at {}:{} in {}", line, col, path.display());
                                    }
                                    Err(e) => eprintln!("Panel: Failed to replace at {}:{} in {}: {}", line, col, path.display(), e),
                                }
                            }
                            
                            processed += 1;
                        }
                        Err(std::sync::mpsc::TryRecvError::Empty) => break,
                        Err(std::sync::mpsc::TryRecvError::Disconnected) => break,
                    }
                }
                
                // Check if we're completely done (thread finished AND no more data)
                if *thread_done_clone.borrow() && rx.try_recv().is_err() {
                    // Clear all results from the list
                    while let Some(row) = results_list_for_clear.row_at_index(0) {
                        results_list_for_clear.remove(&row);
                    }
                    
                    let total = *total_replaced_clone.borrow();
                    status_for_update.set_text(&format!("Replaced {} occurrence{}", 
                        total, 
                        if total == 1 { "" } else { "s" }
                    ));
                    
                    // Refresh the search after replace all completes
                    let search_buffer_inner = search_buffer_timeout.clone();
                    let case_toggle_inner = case_toggle_timeout.clone();
                    let whole_word_toggle_inner = whole_word_toggle_timeout.clone();
                    let start_search_inner = start_search_timeout.clone();
                    glib::idle_add_local_once(move || {
                        let search_text = search_buffer_inner.text(&search_buffer_inner.start_iter(), &search_buffer_inner.end_iter(), false).to_string();
                        if !search_text.trim().is_empty() {
                            (start_search_inner)(search_text, case_toggle_inner.is_active(), whole_word_toggle_inner.is_active());
                        }
                    });
                    
                    return glib::ControlFlow::Break;
                }
                
                glib::ControlFlow::Continue
            });
        }
    });
    
    // Save search state when toggle buttons change
    case_toggle.connect_toggled(|btn| {
        let mut settings = crate::settings::get_settings_mut();
        settings.set_search_case_sensitive(btn.is_active());
        let _ = settings.save();
    });
    
    whole_word_toggle.connect_toggled(|btn| {
        let mut settings = crate::settings::get_settings_mut();
        settings.set_search_whole_word(btn.is_active());
        let _ = settings.save();
    });
    
    // Save search query when buffer changes (with debouncing via idle)
    let buffer_for_save = search_buffer.clone();
    search_buffer.connect_changed(move |_| {
        let buffer_clone = buffer_for_save.clone();
        glib::idle_add_local_once(move || {
            let text = buffer_clone.text(&buffer_clone.start_iter(), &buffer_clone.end_iter(), false).to_string();
            let mut settings = crate::settings::get_settings_mut();
            settings.set_search_query(&text);
            let _ = settings.save();
        });
    });

    // Return the panel as a GtkBox
    panel.upcast::<GtkBox>()
}
