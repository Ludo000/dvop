// UI components and setup functions for completion
// This module handles the visual aspects and setup of code completion

use sourceview5::{prelude::*, View, Buffer};
use gtk4::{gdk, Popover, ListBox, Label, ScrolledWindow, Image, Box as GtkBox, Orientation};
use glib;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{get_language_keywords, get_language_snippets, get_keyword_documentation};

// Static flag to prevent recursive completion triggering
static COMPLETION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Completion item types for different kinds of completions
#[derive(Clone, Debug)]
enum CompletionItem {
    Keyword(String),
    Snippet(String, String), // (trigger, content)
    BufferWord(String),
}

/// Extract the programming language from buffer language setting
fn get_buffer_language(buffer: &Buffer) -> String {
    if let Some(language) = buffer.language() {
        let lang_id = language.id().to_string();
        match lang_id.as_str() {
            "rust" => "rust".to_string(),
            "javascript" | "js" => "javascript".to_string(),
            "typescript" | "ts" => "javascript".to_string(), // Use JS completions for TS
            "python" | "python3" => "python".to_string(),
            "c" => "c".to_string(),
            "cpp" | "c++" => "cpp".to_string(),
            "java" => "java".to_string(),
            "html" => "html".to_string(),
            "css" => "css".to_string(),
            _ => "generic".to_string(),
        }
    } else {
        "generic".to_string()
    }
}

/// Setup completion for a source view with proper provider configuration  
pub fn setup_completion(source_view: &View) {
    println!("=== SETTING UP MANUAL COMPLETION ONLY ===");
    let buffer = source_view.buffer();
    
    // Cast buffer to SourceView Buffer
    if let Some(source_buffer) = buffer.downcast_ref::<Buffer>() {
        println!("Buffer cast successful, manual completion ready...");
        
        // Get the language for context (but don't set up auto-completion)
        let language = get_buffer_language(source_buffer);
        println!("Language detected: {}", language);
        
        // Note: We're NOT setting up the automatic CompletionWords providers
        // Only manual completion via Ctrl+Space will be available
        
        println!("Manual completion configuration complete");
        println!("Use Ctrl+Space or F1 to trigger completion manually");
    } else {
        println!("WARNING: Could not setup completion - buffer is not a SourceView buffer");
    }
    println!("=== MANUAL COMPLETION SETUP COMPLETE ===");
}

/// Enhanced completion setup with file-specific behavior
pub fn setup_completion_for_file(source_view: &View, file_path: Option<&Path>) {
    setup_completion(source_view);
    
    if let Some(path) = file_path {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        println!("Setting up manual completion for file type: {}", extension);
        
        // Note: Only manual completion (Ctrl+Space) is available
        // No automatic completion providers are configured
        
        match extension {
            "rs" => {
                println!("Manual Rust completion enabled");
            },
            "js" | "ts" | "jsx" | "tsx" => {
                println!("Manual JavaScript/TypeScript completion enabled");
            },
            "py" => {
                println!("Manual Python completion enabled");
            },
            "java" => {
                println!("Manual Java completion enabled");
            },
            "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" => {
                println!("Manual C/C++ completion enabled");
            },
            "html" | "htm" => {
                println!("Manual HTML completion enabled");
            },
            "css" | "scss" | "sass" | "less" => {
                println!("Manual CSS completion enabled");
            },
            _ => {
                println!("Manual completion enabled for file type: {}", extension);
            }
        }
    }
}

/// Manual trigger for completion - creates a custom visible popup
pub fn trigger_completion(source_view: &View) {
    // Check if completion is already in progress to prevent recursive calls
    if COMPLETION_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        println!("Completion already in progress, skipping...");
        return;
    }
    
    println!("=== CREATING CUSTOM COMPLETION POPUP ===");
    println!("Function called successfully!");
    
    // Get current buffer and cursor position
    let buffer = source_view.buffer();
    let cursor_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&cursor_mark);
    
    // Get text around cursor for context
    let mut start_iter = cursor_iter;
    for _ in 0..10 {
        if start_iter.is_start() { break; }
        start_iter.backward_char();
    }
    
    let context_text = buffer.text(&start_iter, &cursor_iter, false);
    println!("Context around cursor: '{}'", context_text);
    
    // Find the word prefix being typed - improved algorithm
    let mut word_start = cursor_iter;
    let mut moved_back = false;
    
    // Move backward to find the start of the current word
    while !word_start.is_start() {
        let prev_iter = {
            let mut temp = word_start;
            temp.backward_char();
            temp
        };
        let ch = prev_iter.char();
        
        println!("Checking character at offset {}: '{}' (code: {})", prev_iter.offset(), ch, ch as u32);
        
        // Only include alphanumeric characters and underscores in words
        if ch.is_alphanumeric() || ch == '_' {
            word_start.backward_char();
            moved_back = true;
            println!("Moved back, word_start now at offset: {}", word_start.offset());
        } else {
            // We've hit a non-word character, stop here
            println!("Found word boundary at character: '{}', stopping", ch);
            break;
        }
    }
    
    // Get the actual word being typed
    let prefix = buffer.text(&word_start, &cursor_iter, false);
    
    println!("=== WORD BOUNDARY ANALYSIS ===");
    println!("Cursor position: {}", cursor_iter.offset());
    println!("Word start position: {}", word_start.offset());
    println!("Moved back: {}", moved_back);
    println!("Prefix found: '{}'", prefix);
    println!("Prefix length: {}", prefix.len());
    
    // Get some context around the word
    let mut context_start = word_start;
    for _ in 0..5 {
        if context_start.is_start() { break; }
        context_start.backward_char();
    }
    let mut context_end = cursor_iter;
    for _ in 0..5 {
        if context_end.is_end() { break; }
        context_end.forward_char();
    }
    let context = buffer.text(&context_start, &context_end, false);
    println!("Context: '{}'", context);
    println!("=================================");
    
    // Get language-specific keywords
    let language = if let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() {
        get_buffer_language(source_buffer)
    } else {
        "generic".to_string()
    };
    
    println!("Language detected: {}", language);
    
    // Collect completion suggestions with their types
    let mut completion_items = Vec::new();
    let prefix_lower = prefix.to_lowercase();
    
    // Add language keywords that match the prefix
    let keywords = get_language_keywords(&language);
    if !keywords.is_empty() {
        println!("Found {} keywords for language {}", keywords.len(), language);
        for keyword in keywords {
            if prefix.is_empty() || keyword.to_lowercase().starts_with(&prefix_lower) {
                completion_items.push(CompletionItem::Keyword(keyword.to_string()));
            }
        }
    } else {
        println!("No keywords found for language: {}", language);
    }
    
    // Add language snippets that match the prefix
    let snippets = get_language_snippets(&language);
    if !snippets.is_empty() {
        println!("Found {} snippets for language {}", snippets.len(), language);
        for (trigger, content) in snippets {
            if prefix.is_empty() || trigger.to_lowercase().starts_with(&prefix_lower) {
                completion_items.push(CompletionItem::Snippet(trigger.to_string(), content.to_string()));
            }
        }
    } else {
        println!("No snippets found for language: {}", language);
    }
    
    // Add buffer words that match the prefix
    let buffer_text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    let words: Vec<&str> = buffer_text.split_whitespace().collect();
    println!("Buffer contains {} words", words.len());
    
    for word in words {
        let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        if clean_word.len() > 2 
            && clean_word != prefix  // Don't suggest the same word being typed
            && (prefix.is_empty() || clean_word.to_lowercase().starts_with(&prefix_lower)) 
            && !completion_items.iter().any(|item| {
                match item {
                    CompletionItem::Keyword(k) => k == clean_word,
                    CompletionItem::Snippet(s, _) => s == clean_word,
                    CompletionItem::BufferWord(w) => w == clean_word,
                }
            }) {
            completion_items.push(CompletionItem::BufferWord(clean_word.to_string()));
        }
    }
    
    // Convert completion items to display strings and prepare for insertion
    let mut suggestions_with_content: Vec<(String, CompletionItem)> = Vec::new();
    
    for item in completion_items {
        let display_text = match &item {
            CompletionItem::Keyword(k) => format!("{} (keyword)", k),
            CompletionItem::Snippet(trigger, _) => format!("{} (snippet)", trigger),
            CompletionItem::BufferWord(w) => w.clone(),
        };
        suggestions_with_content.push((display_text, item));
    }
    
    // If no suggestions, add some default ones only if no prefix
    if suggestions_with_content.is_empty() {
        if prefix.is_empty() {
            // Show comprehensive default suggestions based on language
            let defaults = match language.as_str() {
                "rust" => vec![
                    ("fn (snippet)".to_string(), CompletionItem::Snippet("fn".to_string(), 
                        "fn function_name(parameters) -> return_type {\n    // body\n}".to_string())),
                    ("struct (snippet)".to_string(), CompletionItem::Snippet("struct".to_string(),
                        "struct Name {\n    field: Type,\n}".to_string())),
                    ("impl (snippet)".to_string(), CompletionItem::Snippet("impl".to_string(),
                        "impl Type {\n    // methods\n}".to_string())),
                    ("match (snippet)".to_string(), CompletionItem::Snippet("match".to_string(),
                        "match expression {\n    pattern => result,\n}".to_string())),
                    ("if (snippet)".to_string(), CompletionItem::Snippet("if".to_string(),
                        "if condition {\n    // body\n}".to_string())),
                    ("for (snippet)".to_string(), CompletionItem::Snippet("for".to_string(),
                        "for item in iterator {\n    // body\n}".to_string())),
                    ("async_fn (snippet)".to_string(), CompletionItem::Snippet("async_fn".to_string(),
                        "async fn function_name(parameters) -> return_type {\n    // async body\n}".to_string())),
                    ("test (snippet)".to_string(), CompletionItem::Snippet("test".to_string(),
                        "#[test]\nfn test_name() {\n    // test code\n}".to_string())),
                    ("vec_new (snippet)".to_string(), CompletionItem::Snippet("vec_new".to_string(),
                        "let vec = Vec::new();".to_string())),
                    ("result (snippet)".to_string(), CompletionItem::Snippet("result".to_string(),
                        "Result<T, E>".to_string())),
                    ("custom_error (snippet)".to_string(), CompletionItem::Snippet("custom_error".to_string(),
                        "use std::fmt;\n\n#[derive(Debug)]\npub enum MyError {\n    InvalidInput(String),\n}".to_string())),
                    ("channel (snippet)".to_string(), CompletionItem::Snippet("channel".to_string(),
                        "use tokio::sync::mpsc;\n\nlet (tx, mut rx) = mpsc::channel::<MessageType>(32);".to_string())),
                    ("trait (snippet)".to_string(), CompletionItem::Snippet("trait".to_string(),
                        "trait TraitName {\n    // trait methods\n}".to_string())),
                    ("enum (snippet)".to_string(), CompletionItem::Snippet("enum".to_string(),
                        "enum Name {\n    Variant1,\n    Variant2(Type),\n}".to_string())),
                    ("derive (snippet)".to_string(), CompletionItem::Snippet("derive".to_string(),
                        "#[derive(Debug, Clone)]\nstruct Name {\n    field: Type,\n}".to_string())),
                    ("builder (snippet)".to_string(), CompletionItem::Snippet("builder".to_string(),
                        "pub struct Builder {\n    field: Option<Type>,\n}".to_string())),
                    ("let (keyword)".to_string(), CompletionItem::Keyword("let".to_string())),
                    ("mut (keyword)".to_string(), CompletionItem::Keyword("mut".to_string())),
                    ("pub (keyword)".to_string(), CompletionItem::Keyword("pub".to_string())),
                    ("use (keyword)".to_string(), CompletionItem::Keyword("use".to_string())),
                ],
                _ => vec![
                    ("fn (snippet)".to_string(), CompletionItem::Snippet("fn".to_string(), 
                        "fn function_name(parameters) -> return_type {\n    // body\n}".to_string())),
                    ("let (keyword)".to_string(), CompletionItem::Keyword("let".to_string())),
                    ("mut (keyword)".to_string(), CompletionItem::Keyword("mut".to_string())),
                    ("if (keyword)".to_string(), CompletionItem::Keyword("if".to_string())),
                    ("else (keyword)".to_string(), CompletionItem::Keyword("else".to_string())),
                    ("match (snippet)".to_string(), CompletionItem::Snippet("match".to_string(),
                        "match expression {\n    pattern => result,\n}".to_string())),
                ]
            };
            suggestions_with_content = defaults;
            println!("Using default {} suggestions for empty prefix", language);
        } else {
            println!("No suggestions found for prefix: '{}'", prefix);
            // Don't add test_completion fallback if we have a specific prefix
            COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
            return;
        }
    }
    
    // Sort suggestions by display text
    suggestions_with_content.sort_by(|a, b| a.0.cmp(&b.0));
    suggestions_with_content.truncate(20); // Increase to 20 suggestions to test scrolling
    
    println!("Found {} completion suggestions: {:?}", 
             suggestions_with_content.len(), 
             suggestions_with_content.iter().map(|(display, _)| display).collect::<Vec<_>>());
    
    if suggestions_with_content.is_empty() {
        println!("No completion suggestions found - not showing popup");
        // Reset the completion flag since we're not showing a popup
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
        return;
    }
    
    // Create custom completion popup
    println!("About to create popup...");
    create_completion_popup(source_view, &suggestions_with_content, &prefix, word_start.offset(), cursor_iter.offset());
    
    // Reset the completion flag after a short delay
    glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
    });
    
    println!("=== CUSTOM COMPLETION POPUP CREATED ===");
}

/// Create a custom completion popup using GTK Popover
fn create_completion_popup(source_view: &View, suggestions_with_content: &[(String, CompletionItem)], _prefix: &str, word_start_offset: i32, cursor_offset: i32) {
    println!("=== CREATING POPOVER ===");
    
    // Create popover
    let popover = Popover::new();
    println!("Popover created");
    
    popover.set_parent(source_view);
    println!("Popover parent set");
    
    popover.set_autohide(true);
    println!("Popover autohide set");
    
    // Create scrolled window for suggestions
    let scrolled = ScrolledWindow::builder()
        .max_content_height(200)
        .max_content_width(300)
        .min_content_height(200)
        .min_content_width(250)
        .propagate_natural_height(false)
        .propagate_natural_width(false)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .overlay_scrolling(true)
        .build();
    println!("ScrolledWindow created");
    
    // Create list box for suggestions
    let list_box = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .show_separators(false)
        .build();
    
    // Ensure the list box can be scrolled
    list_box.set_size_request(250, -1);
    
    println!("ListBox created");
    
    // Add suggestions to list
    for (i, (display_text, completion_item)) in suggestions_with_content.iter().enumerate() {
        println!("Adding suggestion {}: {}", i, display_text);
        
        // Create a horizontal box to hold icon and text
        let item_box = GtkBox::new(Orientation::Horizontal, 8);
        item_box.set_margin_start(8);
        item_box.set_margin_end(8);
        item_box.set_margin_top(4);
        item_box.set_margin_bottom(4);
        
        // Create appropriate icon based on completion type
        let icon = match completion_item {
            CompletionItem::Keyword(_) => {
                // Use a wrench/tool icon for language keywords (reserved words)
                Image::from_icon_name("insert-text-symbolic")
            },
            CompletionItem::Snippet(_, _) => {
                // Use a template/code block icon for code snippets
                Image::from_icon_name("text-x-script-symbolic")
            },
            CompletionItem::BufferWord(_) => {
                // Use a text file icon for words from the current buffer
                Image::from_icon_name("text-x-generic-symbolic")
            },
        };
        
        // Set icon size
        icon.set_icon_size(gtk4::IconSize::Normal);
        
        // Create label for the text
        let label = Label::builder()
            .label(display_text)
            .xalign(0.0)
            .build();
        
        // Add icon and label to the box
        item_box.append(&icon);
        item_box.append(&label);
        
        list_box.append(&item_box);
    }
    
    // Select first row by default
    if let Some(first_row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first_row));
    }
    
    scrolled.set_child(Some(&list_box));
    popover.set_child(Some(&scrolled));
    println!("Popover content set");
    
    // Handle selection
    let buffer = source_view.buffer();
    let suggestions_clone = suggestions_with_content.to_vec();
    let popover_for_close = popover.clone();
    
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some((display_text, completion_item)) = suggestions_clone.get(index) {
            println!("Selected completion: {}", display_text);
            println!("Replacing text from offset {} to {}", word_start_offset, cursor_offset);
            
            // Get the text that will be replaced for debugging
            let start_iter = buffer.iter_at_offset(word_start_offset);
            let end_iter = buffer.iter_at_offset(cursor_offset);
            let text_to_replace = buffer.text(&start_iter, &end_iter, false);
            println!("Text being replaced: '{}'", text_to_replace);
            
            // Determine what to insert based on completion type
            let text_to_insert = match completion_item {
                CompletionItem::Keyword(keyword) => keyword.clone(),
                CompletionItem::BufferWord(word) => word.clone(),
                CompletionItem::Snippet(_, content) => {
                    // Process snippet - remove placeholders for now and replace with simple text
                    expand_snippet_content(content)
                }
            };
            
            // Replace the prefix with the selected suggestion/snippet
            let mut start_iter = buffer.iter_at_offset(word_start_offset);
            let mut end_iter = buffer.iter_at_offset(cursor_offset);
            
            buffer.delete(&mut start_iter, &mut end_iter);
            let mut insert_iter = buffer.iter_at_offset(word_start_offset);
            buffer.insert(&mut insert_iter, &text_to_insert);
            
            println!("Inserted: '{}'", text_to_insert);
            
            // Reset completion flag
            COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
            
            // Close popover
            popover_for_close.popdown();
        }
    });
    
    // Reset completion flag when popover is closed
    let popover_close_handler = popover.clone();
    popover_close_handler.connect_closed(move |_| {
        println!("Popover closed - resetting completion flag");
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
    });
    
    // Calculate cursor position for better popover positioning
    let buffer = source_view.buffer();
    let cursor_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&cursor_mark);
    
    // Get cursor rectangle in buffer coordinates first
    let cursor_rect = source_view.iter_location(&cursor_iter);
    println!("Cursor location (buffer coords): x={}, y={}, width={}, height={}", 
             cursor_rect.x(), cursor_rect.y(), cursor_rect.width(), cursor_rect.height());
    
    // Convert buffer coordinates to widget coordinates
    let (widget_x, widget_y) = source_view.buffer_to_window_coords(
        gtk4::TextWindowType::Widget,
        cursor_rect.x(),
        cursor_rect.y()
    );
    
    println!("Cursor location (widget coords): x={}, y={}", widget_x, widget_y);
    
    // Position the popover below the cursor
    let pointing_rect = gdk::Rectangle::new(
        widget_x,
        widget_y + cursor_rect.height(),
        cursor_rect.width().max(1),  // Ensure minimum width
        1
    );
    popover.set_pointing_to(Some(&pointing_rect));
    println!("Popover positioned at widget coordinates: x={}, y={}", widget_x, widget_y + cursor_rect.height());
    
    // Handle keyboard navigation in the popover
    let key_controller = gtk4::EventControllerKey::new();
    let popover_clone = popover.clone();
    let list_box_clone = list_box.clone();
    let scrolled_clone = scrolled.clone();
    
    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        println!("Popover key pressed: {:?}", keyval);
        match keyval {
            gdk::Key::Escape => {
                println!("Escape pressed - closing popover");
                COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
                popover_clone.popdown();
                glib::Propagation::Stop
            },
            gdk::Key::Return | gdk::Key::Tab => {
                println!("Return/Tab pressed - activating selection");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    selected_row.activate();
                }
                glib::Propagation::Stop
            },
            gdk::Key::Down => {
                println!("Down arrow - moving to next item");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    let next_index = selected_row.index() + 1;
                    if let Some(next_row) = list_box_clone.row_at_index(next_index) {
                        list_box_clone.select_row(Some(&next_row));
                        // Scroll to make the selected row visible
                        scroll_to_row(&scrolled_clone, &next_row);
                    }
                }
                glib::Propagation::Stop
            },
            gdk::Key::Up => {
                println!("Up arrow - moving to previous item");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    let prev_index = selected_row.index().saturating_sub(1);
                    if let Some(prev_row) = list_box_clone.row_at_index(prev_index) {
                        list_box_clone.select_row(Some(&prev_row));
                        // Scroll to make the selected row visible
                        scroll_to_row(&scrolled_clone, &prev_row);
                    }
                }
                glib::Propagation::Stop
            },
            _ => glib::Propagation::Proceed
        }
    });
    
    list_box.add_controller(key_controller);
    println!("Key controller added to list box");
    
    // Show the popover
    println!("About to show popover...");
    popover.popup();
    println!("Popover.popup() called");
    
    // Give focus to the list box for keyboard navigation
    list_box.grab_focus();
    println!("Focus grabbed by list box");
    
    // Additional debugging
    println!("Popover is_visible: {}", popover.is_visible());
    println!("ListBox has_focus: {}", list_box.has_focus());
    
    println!("Custom completion popup displayed with {} suggestions", suggestions_with_content.len());
}

/// Helper function to scroll to a specific row in the scrolled window
fn scroll_to_row(scrolled: &ScrolledWindow, row: &gtk4::ListBoxRow) {
    // Get the row's allocation (position and size)
    let row_allocation = row.allocation();
    let row_height = row_allocation.height() as f64;
    let row_y = row_allocation.y() as f64;
    
    // Get the scrolled window's viewport
    if let Some(_viewport) = scrolled.child() {
        let adjustment = scrolled.vadjustment();
        let current_scroll = adjustment.value();
        let page_size = adjustment.page_size();
        
        // Calculate if we need to scroll
        let visible_top = current_scroll;
        let visible_bottom = current_scroll + page_size;
        
        // If the row is above the visible area, scroll up to it
        if row_y < visible_top {
            adjustment.set_value(row_y);
        }
        // If the row is below the visible area, scroll down to show it
        else if row_y + row_height > visible_bottom {
            let new_scroll = (row_y + row_height) - page_size;
            adjustment.set_value(new_scroll.max(0.0));
        }
        // If the row is already visible, don't scroll
    }
}

/// Setup keyboard shortcuts for completion with manual trigger only
pub fn setup_completion_shortcuts(source_view: &View) {
    println!("Setting up completion keyboard shortcuts...");
    
    // Create key controller with high priority to ensure it gets events
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);
    
    let source_view_clone = source_view.clone();
    key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
        // Debug key press
        println!("Key pressed: {:?}, state: {:?}", keyval, state);
        
        // Check for Ctrl+Space (manual trigger)
        if keyval == gdk::Key::space 
            && state.contains(gdk::ModifierType::CONTROL_MASK) {
            println!("*** Ctrl+Space detected! Triggering manual completion ***");
            
            // Use timeout to ensure the key event is fully processed first
            let sv = source_view_clone.clone();
            glib::idle_add_local_once(move || {
                trigger_completion(&sv);
            });
            
            return glib::Propagation::Stop;
        }
        
        // Check for F1 key as alternative trigger for testing
        if keyval == gdk::Key::F1 {
            println!("*** F1 detected! Triggering test completion ***");
            let sv = source_view_clone.clone();
            glib::idle_add_local_once(move || {
                trigger_completion(&sv);
            });
            return glib::Propagation::Stop;
        }
        
        // Let other keys through
        glib::Propagation::Proceed
    });
    
    source_view.add_controller(key_controller);
    
    println!("Completion keyboard shortcuts enabled:");
    println!("  - Ctrl+Space for manual trigger");
    println!("  - F1 for testing trigger"); 
    println!("  - Auto-completion has been DISABLED");
}

/// Get completion documentation
pub fn get_completion_documentation(keyword: &str, language: &str) -> String {
    get_keyword_documentation(language, keyword)
}

/// Expand snippet content by removing placeholders and converting to simple text
/// For now, this is a basic implementation that removes ${n:placeholder} syntax
fn expand_snippet_content(content: &str) -> String {
    // Use regex to find and replace all snippet placeholders ${n:default_text}
    // For now, we'll use a simple parser since regex is not available
    
    let mut result = String::new();
    let chars: Vec<char> = content.chars().collect();
    let mut i = 0;
    
    while i < chars.len() {
        if i + 2 < chars.len() && chars[i] == '$' && chars[i + 1] == '{' {
            // Find the closing brace
            let mut j = i + 2;
            let mut brace_count = 1;
            
            while j < chars.len() && brace_count > 0 {
                if chars[j] == '{' {
                    brace_count += 1;
                } else if chars[j] == '}' {
                    brace_count -= 1;
                }
                j += 1;
            }
            
            if brace_count == 0 {
                // Extract the placeholder content
                let placeholder: String = chars[i + 2..j - 1].iter().collect();
                
                // Parse ${n:default_text} format
                if let Some(colon_pos) = placeholder.find(':') {
                    // Extract the default text after the colon
                    let default_text = &placeholder[colon_pos + 1..];
                    result.push_str(default_text);
                } else {
                    // Just a number, use generic placeholder
                    result.push_str("placeholder");
                }
                
                i = j;
                continue;
            }
        }
        
        result.push(chars[i]);
        i += 1;
    }
    
    result
}
