// UI components and setup functions for completion
// This module handles the visual aspects and setup of code completion

use sourceview5::{prelude::*, View, Buffer};
use gtk4::{gdk, Popover, ListBox, Label, ScrolledWindow, Image, Box as GtkBox, Orientation, pango};
use glib;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use super::{get_language_keywords, get_language_snippets};

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
    
    // Get language for documentation
    let buffer = source_view.buffer();
    let language = if let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() {
        get_buffer_language(source_buffer)
    } else {
        "generic".to_string()
    };
    
    // Create popover
    let popover = Popover::new();
    println!("Popover created");
    
    popover.set_parent(source_view);
    println!("Popover parent set");
    
    popover.set_autohide(true);
    println!("Popover autohide set");
    
    // Create scrolled window for suggestions
    let scrolled = ScrolledWindow::builder()
        .max_content_height(500)  // Significantly increased height for more visible items
        .max_content_width(1400)  // Much wider for extensive documentation display
        .min_content_height(300)  // Increased minimum height
        .min_content_width(1200)  // Significantly increased minimum width for documentation
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
    list_box.set_size_request(1200, -1);  // Much wider for extensive documentation
    
    println!("ListBox created");
    
    // Add suggestions to list
    for (i, (display_text, completion_item)) in suggestions_with_content.iter().enumerate() {
        println!("Adding suggestion {}: {}", i, display_text);
        
        // Create a horizontal box to hold icon, text, and documentation
        let item_box = GtkBox::new(Orientation::Horizontal, 16);  // Increased spacing for larger popup
        item_box.set_margin_start(16);   // Increased margins for better visual hierarchy
        item_box.set_margin_end(16);
        item_box.set_margin_top(8);      // Increased vertical spacing for taller popup
        item_box.set_margin_bottom(8);
        
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
        
        // Create label for the main text with compact fixed width
        let label = Label::builder()
            .label(display_text)
            .xalign(0.0)
            .hexpand(false)
            .width_chars(20)  // Reduced width to give more room to documentation
            .build();
        
        // Add CSS class for bold styling
        label.add_css_class("completion-label");
        
        // Get and add enhanced documentation
        let doc_text = match completion_item {
            CompletionItem::Keyword(keyword) => {
                get_enhanced_keyword_documentation(&language, keyword)
            },
            CompletionItem::Snippet(trigger, content) => {
                get_enhanced_snippet_documentation(&language, trigger, content)
            },
            CompletionItem::BufferWord(word) => {
                format!("{} - Word found in current buffer. This identifier is already used elsewhere in your code.", word)
            }
        };
        
        // Create documentation label with much more horizontal space
        let doc_label = Label::builder()
            .label(&doc_text)
            .xalign(0.0)
            .hexpand(true)
            .wrap(true)                              // Enable wrapping for long documentation
            .wrap_mode(pango::WrapMode::Word)        // Wrap at word boundaries
            .max_width_chars(120)                    // Much more characters for extensive documentation in larger popup
            .build();
        
        // Style the documentation label to be smaller and dimmed
        doc_label.add_css_class("completion-doc");
        
        // Add some CSS styling for better appearance with more spacing
        let css_provider = gtk4::CssProvider::new();
        css_provider.load_from_data(
            ".completion-label { 
                font-weight: bold;
                font-size: 1.0em;
                color: @theme_fg_color;
            }
            .completion-doc { 
                font-size: 0.95em; 
                font-weight: 700;
                color: alpha(@theme_fg_color, 0.75); 
                margin-left: 40px;
                line-height: 1.4;
                padding-right: 20px;
                padding-top: 2px;
                padding-bottom: 2px;
            }"
        );
        
        if let Some(display) = gdk::Display::default() {
            gtk4::style_context_add_provider_for_display(
                &display,
                &css_provider,
                gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
            );
        }
        
        // Add icon, label, and documentation to the horizontal box
        item_box.append(&icon);
        item_box.append(&label);
        item_box.append(&doc_label);
        
        list_box.append(&item_box);
    }
    
    // Select first row by default
    if let Some(first_row) = list_box.row_at_index(0) {
        list_box.select_row(Some(&first_row));
    }
    
    scrolled.set_child(Some(&list_box));
    popover.set_child(Some(&scrolled));
    println!("Popover content set with documentation");
    
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
    
    println!("Custom completion popup displayed with {} suggestions and documentation", suggestions_with_content.len());
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
    get_enhanced_keyword_documentation(language, keyword)
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

/// Get enhanced documentation for keywords with detailed explanations
fn get_enhanced_keyword_documentation(language: &str, keyword: &str) -> String {
    match (language, keyword) {
        // Rust keywords with detailed explanations of what they do
        ("rust", "let") => "let - Creates a new variable binding. By default variables are immutable. Use 'let mut' to make them changeable. Example: let name = \"Alice\"; let mut count = 0;".to_string(),
        ("rust", "mut") => "mut - Makes a variable mutable so you can change its value after creation. Essential for variables that need to be modified. Example: let mut score = 0; score += 10;".to_string(),
        ("rust", "fn") => "fn - Defines a function that can be called to execute code. Functions help organize code and enable reuse. Example: fn greet(name: &str) { println!(\"Hello, {}!\", name); }".to_string(),
        ("rust", "pub") => "pub - Makes items (functions, structs, modules) accessible from outside their current module. Controls code visibility and API design. Example: pub fn public_api() {}".to_string(),
        ("rust", "use") => "use - Imports items from other modules into the current scope, avoiding repetitive module paths. Example: use std::collections::HashMap; // Now you can use HashMap directly".to_string(),
        ("rust", "struct") => "struct - Creates a custom data type by grouping related fields together. The foundation of data modeling in Rust. Example: struct Person { name: String, age: u32 }".to_string(),
        ("rust", "enum") => "enum - Defines a type that can be one of several different variants. Perfect for representing choices or states. Example: enum Status { Loading, Success(String), Error }".to_string(),
        ("rust", "impl") => "impl - Adds methods and functionality to structs and enums. This is how you define behavior for your custom types. Example: impl Person { fn greet(&self) { println!(\"Hi, I'm {}\", self.name); } }".to_string(),
        ("rust", "trait") => "trait - Defines shared behavior that different types can implement. Like interfaces in other languages but more powerful. Example: trait Drawable { fn draw(&self); }".to_string(),
        ("rust", "if") => "if - Executes code conditionally based on a boolean expression. The cornerstone of program logic and decision making. Example: if temperature > 30 { println!(\"It's hot!\"); }".to_string(),
        ("rust", "else") => "else - Provides alternative code to execute when the if condition is false. Handles the 'otherwise' case. Example: if sunny { go_outside(); } else { stay_inside(); }".to_string(),
        ("rust", "match") => "match - Compares a value against patterns and executes corresponding code. More powerful than switch statements - ensures all cases are handled. Example: match result { Ok(value) => use_value(value), Err(e) => handle_error(e) }".to_string(),
        ("rust", "loop") => "loop - Creates an infinite loop that runs until you explicitly break out of it. Useful for event loops and continuous processing. Example: loop { if should_stop() { break; } process(); }".to_string(),
        ("rust", "while") => "while - Repeats code as long as a condition remains true. Perfect for processing until a goal is reached. Example: while !queue.is_empty() { process(queue.pop()); }".to_string(),
        ("rust", "for") => "for - Iterates over collections, ranges, or anything that implements Iterator. The most common way to loop in Rust. Example: for item in &my_list { println!(\"{}\", item); }".to_string(),
        ("rust", "break") => "break - Immediately exits the current loop, skipping any remaining iterations. Use to stop looping when a condition is met. Example: for x in data { if x == target { break; } }".to_string(),
        ("rust", "continue") => "continue - Skips the rest of the current loop iteration and jumps to the next one. Use to skip processing certain items. Example: for x in data { if x < 0 { continue; } process(x); }".to_string(),
        ("rust", "return") => "return - Immediately exits a function and optionally returns a value to the caller. Example: fn divide(a: f64, b: f64) -> Option<f64> { if b == 0.0 { return None; } Some(a / b) }".to_string(),
        ("rust", "const") => "const - Declares a constant value that's computed at compile time and never changes. Perfect for configuration values. Example: const MAX_USERS: usize = 1000;".to_string(),
        ("rust", "static") => "static - Creates a global variable that exists for the entire program lifetime. Use sparingly for truly global state. Example: static COUNTER: AtomicUsize = AtomicUsize::new(0);".to_string(),
        ("rust", "mod") => "mod - Organizes code into modules for better structure and namespace management. Helps keep large projects organized. Example: mod database { pub fn connect() {} }".to_string(),
        ("rust", "crate") => "crate - Refers to the root of the current compilation unit. Use to access items from your crate's root module. Example: crate::utils::helper_function();".to_string(),
        ("rust", "super") => "super - Refers to the parent module in the module hierarchy. Use to access items one level up. Example: super::parent_function();".to_string(),
        ("rust", "self") => "self - Refers to the current module or the instance of a struct in methods. Context-dependent but essential. Example: fn method(&self) { self.field }".to_string(),
        ("rust", "Self") => "Self - A type alias for the current type. Makes code cleaner in impl blocks and trait implementations. Example: fn new() -> Self { Self { field: 0 } }".to_string(),
        ("rust", "where") => "where - Adds constraints to generic types, making complex type bounds more readable. Example: fn process<T>(item: T) where T: Clone + Debug { /* */ }".to_string(),
        ("rust", "unsafe") => "unsafe - Allows operations that bypass Rust's safety guarantees. Use only when absolutely necessary and you understand the risks. Example: unsafe { *raw_pointer }".to_string(),
        ("rust", "extern") => "extern - Declares functions from other languages (like C) or specifies calling conventions. Used for FFI (Foreign Function Interface). Example: extern \"C\" { fn printf(format: *const c_char, ...) -> c_int; }".to_string(),
        ("rust", "ref") => "ref - Creates a reference to a value in pattern matching instead of moving it. Useful for borrowing in matches. Example: match &value { ref inner => use_reference(inner) }".to_string(),
        ("rust", "move") => "move - Forces a closure to take ownership of all captured variables instead of borrowing them. Example: let closure = move |x| { expensive_data.process(x) };".to_string(),
        ("rust", "async") => "async - Makes a function asynchronous, returning a Future that can be awaited. Essential for concurrent programming. Example: async fn fetch_data() -> Result<String, Error> { /* */ }".to_string(),
        ("rust", "await") => "await - Waits for an async operation to complete, yielding control to other tasks while waiting. Example: let data = fetch_from_server().await?;".to_string(),
        ("rust", "dyn") => "dyn - Indicates a trait object for dynamic dispatch at runtime. Use when you need different types behind the same interface. Example: Box<dyn Draw>".to_string(),
        
        ("rust", "as") => "as - Performs explicit type conversion between compatible types. Use when you need to convert one type to another safely. Example: let x = 42i32 as f64; // converts integer to float".to_string(),
        ("rust", "in") => "in - Specifies what to iterate over in for loops. Connects the loop variable with the collection being iterated. Example: for item in &collection { process(item); }".to_string(),
        ("rust", "type") => "type - Creates a type alias to give a new name to an existing type. Makes complex types easier to work with. Example: type UserId = u64; type Result<T> = std::result::Result<T, MyError>;".to_string(),
        
        // Common Rust types with focus on what they do and when to use them
        ("rust", "String") => "String - Stores owned, growable UTF-8 text that you can modify. Use when you need to build, modify, or own text data. Example: let mut name = String::from(\"Hello\"); name.push_str(\", World!\");".to_string(),
        ("rust", "str") => "str - References immutable text data (string slice). Use &str for reading text without owning it. Example: fn greet(name: &str) { println!(\"Hello, {}!\", name); }".to_string(),
        ("rust", "Vec") => "Vec - Stores a growable list of items on the heap. Use when you need a dynamic array that can change size. Example: let mut scores = Vec::new(); scores.push(95); scores.push(87);".to_string(),
        ("rust", "Option") => "Option - Represents a value that might exist (Some) or not exist (None). Use instead of null to safely handle missing values. Example: fn find_user(id: u32) -> Option<User> { /* returns Some(user) or None */ }".to_string(),
        ("rust", "Result") => "Result - Represents either success (Ok) or failure (Err). Use for operations that can fail and need error handling. Example: fn parse_number(s: &str) -> Result<i32, ParseIntError> { s.parse() }".to_string(),
        ("rust", "Some") => "Some - Wraps a value inside an Option to indicate that the value exists. Use when returning a successful result from Option-returning functions. Example: Some(42)".to_string(),
        ("rust", "None") => "None - Represents the absence of a value in an Option. Use when a function couldn't produce a meaningful result. Example: if list.is_empty() { None } else { Some(list[0]) }".to_string(),
        ("rust", "Ok") => "Ok - Wraps a successful value in a Result. Use when an operation completed successfully. Example: Ok(\"Success!\".to_string())".to_string(),
        ("rust", "Err") => "Err - Wraps an error value in a Result. Use when an operation failed and you need to return error information. Example: Err(\"File not found\".to_string())".to_string(),
        ("rust", "Box") => "Box - Allocates values on the heap instead of the stack. Use for large objects, recursive data structures, or trait objects. Example: let big_data = Box::new([0; 1000000]);".to_string(),
        ("rust", "Arc") => "Arc - Enables multiple owners to share read-only data across threads safely. Use when multiple threads need access to the same data. Example: let shared_data = Arc::new(expensive_computation());".to_string(),
        ("rust", "Rc") => "Rc - Enables multiple owners to share data within a single thread. Use when you need shared ownership but don't need thread safety. Example: let shared = Rc::new(data); let copy = shared.clone();".to_string(),
        ("rust", "HashMap") => "HashMap - Stores key-value pairs with fast lookups, insertions, and deletions. Use when you need to associate keys with values efficiently. Example: let mut scores = HashMap::new(); scores.insert(\"Alice\", 95);".to_string(),
        ("rust", "HashSet") => "HashSet - Stores unique values with fast membership testing. Use when you need to track what items you've seen or eliminate duplicates. Example: let mut seen = HashSet::new(); seen.insert(user_id);".to_string(),
        
        // Numeric types with practical guidance on when to use them
        ("rust", "i8") => "i8 - Stores small signed integers (-128 to 127). Use when memory is tight and you know values will be small. Example: let temperature_offset: i8 = -40;".to_string(),
        ("rust", "i16") => "i16 - Stores medium signed integers (-32,768 to 32,767). Use for coordinates, small measurements, or when i8 is too small. Example: let altitude: i16 = 8848;".to_string(),
        ("rust", "i32") => "i32 - The default integer type for most whole numbers. Good performance and sufficient range for most applications. Example: let count = 42; // automatically i32".to_string(),
        ("rust", "i64") => "i64 - Stores large signed integers. Use for timestamps, file sizes, large counts, or when i32 might overflow. Example: let timestamp: i64 = 1640995200;".to_string(),
        ("rust", "i128") => "i128 - Stores extremely large signed integers. Use for cryptographic operations or when even i64 isn't enough. Example: let huge_number: i128 = 1_000_000_000_000_000_000_000_000;".to_string(),
        ("rust", "isize") => "isize - Integer the same size as a pointer (32-bit on 32-bit systems, 64-bit on 64-bit). Use for array indexing and memory offsets. Example: let index: isize = vec.len() as isize - 1;".to_string(),
        ("rust", "u8") => "u8 - Stores small unsigned integers (0 to 255). Perfect for bytes, RGB color values, or small positive counts. Example: let red: u8 = 255; let byte_data: Vec<u8> = vec![72, 101, 108, 108, 111];".to_string(),
        ("rust", "u16") => "u16 - Stores medium unsigned integers (0 to 65,535). Use for port numbers, Unicode code points, or moderate counts. Example: let port: u16 = 8080;".to_string(),
        ("rust", "u32") => "u32 - Stores large unsigned integers. Use for IDs, large counts, or when you need more range than u16. Example: let user_id: u32 = 1_234_567;".to_string(),
        ("rust", "u64") => "u64 - Stores very large unsigned integers. Use for file sizes, large timestamps, or very big counts. Example: let file_size: u64 = 1_073_741_824; // 1GB".to_string(),
        ("rust", "u128") => "u128 - Stores extremely large unsigned integers. Use for cryptographic keys or when u64 isn't sufficient. Example: let crypto_key: u128 = 340_282_366_920_938_463_463_374_607_431_768_211_455;".to_string(),
        ("rust", "usize") => "usize - Unsigned integer the same size as a pointer. Use for array lengths, vector indices, and memory sizes. Example: let len: usize = vec.len(); let item = vec[len - 1];".to_string(),
        ("rust", "f32") => "f32 - Stores single-precision floating point numbers. Use when memory is tight or GPU compatibility is needed. Example: let pi: f32 = 3.14159; let coordinates: [f32; 3] = [1.0, 2.5, 3.7];".to_string(),
        ("rust", "f64") => "f64 - The default floating point type with double precision. Use for scientific calculations or when precision matters. Example: let precise_pi = 3.141592653589793; // automatically f64".to_string(),
        ("rust", "bool") => "bool - Stores true or false values for logical operations. The foundation of all conditional logic and decision making. Example: let is_valid = true; if is_valid { proceed(); }".to_string(),
        ("rust", "char") => "char - Stores a single Unicode character (4 bytes). Use for individual characters, emojis, or text processing. Example: let letter: char = 'A'; let emoji: char = '🦀'; let unicode: char = 'α';".to_string(),
        
        // Common traits with practical explanations of what they enable
        ("rust", "Clone") => "Clone - Enables explicit duplication of values with .clone(). Use when you need to make copies of data structures. Example: #[derive(Clone)] struct User { name: String } let user2 = user1.clone();".to_string(),
        ("rust", "Copy") => "Copy - Enables implicit copying for simple types. Values are automatically copied instead of moved. Example: i32, bool, and other primitives implement Copy automatically.".to_string(),
        ("rust", "Debug") => "Debug - Enables debug formatting with {:?} and {:#?}. Essential for troubleshooting and development. Example: #[derive(Debug)] struct Point(i32, i32); println!(\"{:?}\", point);".to_string(),
        ("rust", "Display") => "Display - Enables user-friendly formatting with {}. Implement this to control how your types appear to users. Example: impl Display for User { fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, \"User: {}\", self.name) } }".to_string(),
        ("rust", "Default") => "Default - Provides a default value for your type. Use when your type has a sensible 'empty' or 'initial' state. Example: #[derive(Default)] struct Config { timeout: u64 } let config = Config::default();".to_string(),
        ("rust", "PartialEq") => "PartialEq - Enables equality comparison with == and !=. Most types should implement this for comparison operations. Example: #[derive(PartialEq)] struct Point(i32, i32); assert_eq!(Point(1, 2), Point(1, 2));".to_string(),
        ("rust", "Eq") => "Eq - Indicates full equality (reflexive, symmetric, transitive). Implement when your type has proper mathematical equality. Example: integers and strings implement Eq.".to_string(),
        ("rust", "PartialOrd") => "PartialOrd - Enables ordering comparisons with <, <=, >, >=. Use when your type has a natural ordering. Example: #[derive(PartialOrd)] enables sorting operations.".to_string(),
        ("rust", "Ord") => "Ord - Provides total ordering for your type. Required for use as keys in BTreeMap or sorting operations. Example: strings and numbers implement Ord for consistent sorting.".to_string(),
        ("rust", "Hash") => "Hash - Enables use as keys in HashMap and HashSet. Implement when your type should be hashable for fast lookups. Example: #[derive(Hash)] struct UserId(u64); let mut map = HashMap::new(); map.insert(user_id, data);".to_string(),
        ("rust", "Iterator") => "Iterator - Enables your type to be used in for loops and with iterator methods. Implement to make your collections iterable. Example: impl Iterator for MyRange { type Item = i32; fn next(&mut self) -> Option<i32> { /* */ } }".to_string(),
        ("rust", "Send") => "Send - Indicates that ownership of this type can be transferred between threads safely. Most types implement this automatically. Example: enables moving data to spawned threads.".to_string(),
        ("rust", "Sync") => "Sync - Indicates that references to this type can be shared between threads safely. Example: Arc<T> is Send when T is Sync, enabling shared read access across threads.".to_string(),
        ("rust", "Sized") => "Sized - Indicates that the type has a known size at compile time. Most types implement this automatically. Example: enables storage on the stack and in arrays.".to_string(),
        
        // Standard library items with practical usage guidance
        ("rust", "std") => "std - The Rust standard library containing all built-in functionality. Import specific modules to access collections, I/O, networking, and more. Example: use std::collections::HashMap; use std::fs::File;".to_string(),
        ("rust", "println") => "println! - Prints text to the console with automatic newline. Essential for debugging and user output. Example: println!(\"User {} has {} points\", name, score);".to_string(),
        ("rust", "print") => "print! - Prints text to the console without newline. Use when you want to build output incrementally. Example: print!(\"Loading\"); for _ in 0..3 { print!(\".\"); } println!(\"done!\");".to_string(),
        ("rust", "eprintln") => "eprintln! - Prints error messages to stderr with newline. Use for error output that should be separate from normal output. Example: eprintln!(\"Error: Failed to open file {}\", filename);".to_string(),
        ("rust", "panic") => "panic! - Immediately crashes the program with an error message. Use for unrecoverable errors. Example: panic!(\"Critical error: database connection lost\");".to_string(),
        ("rust", "assert") => "assert! - Crashes the program if a condition is false. Use to enforce invariants and catch bugs during development. Example: assert!(balance >= 0, \"Account balance cannot be negative\");".to_string(),
        ("rust", "assert_eq") => "assert_eq! - Crashes if two values are not equal. Perfect for testing expected outcomes. Example: assert_eq!(calculate_total(&items), 150, \"Total calculation failed\");".to_string(),
        ("rust", "dbg") => "dbg! - Prints a value for debugging and returns it unchanged. Use for quick debugging without changing program flow. Example: let result = dbg!(expensive_calculation()); // prints and returns result".to_string(),
        
        // JavaScript/TypeScript keywords
        ("javascript" | "typescript", "function") => "function - Declares a function. Functions are reusable blocks of code. Example: function greet(name) { return 'Hello ' + name; }".to_string(),
        ("javascript" | "typescript", "var") => "var - Declares a variable with function scope (deprecated, use let/const instead). Example: var x = 5;".to_string(),
        ("javascript" | "typescript", "let") => "let - Declares a block-scoped variable that can be reassigned. Example: let count = 0; count++;".to_string(),
        ("javascript" | "typescript", "const") => "const - Declares a block-scoped constant that cannot be reassigned. Example: const PI = 3.14159;".to_string(),
        ("javascript" | "typescript", "if") => "if - Conditional statement that executes code based on a condition. Example: if (x > 0) { console.log('positive'); }".to_string(),
        ("javascript" | "typescript", "else") => "else - Provides alternative execution path when if condition is false. Example: if (x > 0) { } else { console.log('not positive'); }".to_string(),
        ("javascript" | "typescript", "for") => "for - Loop that repeats code a specific number of times. Example: for (let i = 0; i < 10; i++) { console.log(i); }".to_string(),
        ("javascript" | "typescript", "while") => "while - Loop that repeats while a condition is true. Example: while (x < 10) { x++; }".to_string(),
        ("javascript" | "typescript", "return") => "return - Returns a value from a function and exits the function. Example: return x * 2;".to_string(),
        ("javascript" | "typescript", "class") => "class - Declares a class for object-oriented programming. Example: class Person { constructor(name) { this.name = name; } }".to_string(),
        ("javascript" | "typescript", "extends") => "extends - Creates a class that inherits from another class. Example: class Student extends Person { }".to_string(),
        ("javascript" | "typescript", "super") => "super - Refers to the parent class, used to call parent constructor or methods. Example: super(name);".to_string(),
        ("javascript" | "typescript", "this") => "this - Refers to the current object instance in methods and constructors. Context-dependent keyword.".to_string(),
        ("javascript" | "typescript", "new") => "new - Creates a new instance of a class or constructor function. Example: const person = new Person('John');".to_string(),
        ("javascript" | "typescript", "try") => "try - Begins a block that might throw an exception. Must be followed by catch and/or finally. Example: try { riskyOperation(); }".to_string(),
        ("javascript" | "typescript", "catch") => "catch - Handles exceptions thrown in a try block. Example: catch (error) { console.error(error); }".to_string(),
        ("javascript" | "typescript", "finally") => "finally - Code that always executes after try/catch, regardless of success or failure. Example: finally { cleanup(); }".to_string(),
        ("javascript" | "typescript", "throw") => "throw - Throws an exception/error. Example: throw new Error('Something went wrong');".to_string(),
        ("javascript" | "typescript", "async") => "async - Declares an asynchronous function that returns a Promise. Example: async function fetchData() { }".to_string(),
        ("javascript" | "typescript", "await") => "await - Waits for a Promise to resolve. Can only be used inside async functions. Example: const data = await fetchData();".to_string(),
        ("javascript" | "typescript", "import") => "import - Imports modules, functions, or variables from other files. Example: import { useState } from 'react';".to_string(),
        ("javascript" | "typescript", "export") => "export - Exports functions, variables, or classes to be used in other modules. Example: export const myFunction = () => {};".to_string(),
        
        // Python keywords
        ("python", "def") => "def - Defines a function. Functions encapsulate reusable code. Example: def greet(name): return f'Hello {name}'".to_string(),
        ("python", "class") => "class - Defines a class for object-oriented programming. Example: class Person: def __init__(self, name): self.name = name".to_string(),
        ("python", "if") => "if - Conditional statement that executes code based on a condition. Example: if x > 0: print('positive')".to_string(),
        ("python", "else") => "else - Alternative execution path when if condition is false. Example: if x > 0: pass else: print('not positive')".to_string(),
        ("python", "elif") => "elif - Short for 'else if', allows chaining multiple conditions. Example: if x > 0: pass elif x < 0: print('negative')".to_string(),
        ("python", "for") => "for - Iterates over sequences like lists, tuples, or strings. Example: for item in my_list: print(item)".to_string(),
        ("python", "while") => "while - Repeats code while a condition is true. Example: while x < 10: x += 1".to_string(),
        ("python", "return") => "return - Returns a value from a function and exits the function. Example: return x * 2".to_string(),
        ("python", "import") => "import - Imports modules to use their functions and classes. Example: import math; import os".to_string(),
        ("python", "from") => "from - Imports specific items from a module. Example: from math import sqrt, pi".to_string(),
        ("python", "try") => "try - Begins exception handling block. Example: try: risky_operation()".to_string(),
        ("python", "except") => "except - Handles specific exceptions in try block. Example: except ValueError as e: print(f'Error: {e}')".to_string(),
        ("python", "finally") => "finally - Code that always runs after try/except. Example: finally: cleanup_resources()".to_string(),
        ("python", "with") => "with - Context manager for resource management. Automatically handles cleanup. Example: with open('file.txt') as f: content = f.read()".to_string(),
        ("python", "lambda") => "lambda - Creates anonymous functions for short, simple operations. Example: square = lambda x: x**2".to_string(),
        ("python", "yield") => "yield - Creates a generator function that can pause and resume execution. Example: def count(): yield 1; yield 2".to_string(),
        ("python", "async") => "async - Declares an asynchronous function for concurrent programming. Example: async def fetch_data(): pass".to_string(),
        ("python", "await") => "await - Waits for an asynchronous operation to complete. Example: result = await fetch_data()".to_string(),
        
        // C/C++ keywords
        ("c" | "cpp", "int") => "int - Integer data type for whole numbers. Example: int count = 5;".to_string(),
        ("c" | "cpp", "char") => "char - Character data type for single characters. Example: char letter = 'A';".to_string(),
        ("c" | "cpp", "float") => "float - Single-precision floating-point number. Example: float price = 19.99f;".to_string(),
        ("c" | "cpp", "double") => "double - Double-precision floating-point number with higher precision than float. Example: double pi = 3.14159265359;".to_string(),
        ("c" | "cpp", "void") => "void - Indicates no value/return type. Used for functions that don't return anything. Example: void printHello() { printf(\"Hello\"); }".to_string(),
        ("c" | "cpp", "if") => "if - Conditional statement for decision making. Example: if (x > 0) { printf(\"positive\"); }".to_string(),
        ("c" | "cpp", "else") => "else - Alternative path when if condition is false. Example: if (x > 0) { } else { printf(\"not positive\"); }".to_string(),
        ("c" | "cpp", "for") => "for - Loop with initialization, condition, and increment. Example: for (int i = 0; i < 10; i++) { printf(\"%d\", i); }".to_string(),
        ("c" | "cpp", "while") => "while - Loop that continues while condition is true. Example: while (x < 10) { x++; }".to_string(),
        ("c" | "cpp", "return") => "return - Returns value from function and exits. Example: return x + y;".to_string(),
        ("c" | "cpp", "struct") => "struct - Groups related variables under one name. Example: struct Point { int x, y; };".to_string(),
        ("c" | "cpp", "const") => "const - Makes variables immutable after initialization. Example: const int MAX_SIZE = 100;".to_string(),
        ("c" | "cpp", "static") => "static - Variable retains value between function calls, or limits scope to file. Example: static int counter = 0;".to_string(),
        ("c" | "cpp", "sizeof") => "sizeof - Returns the size in bytes of a data type or variable. Example: int size = sizeof(int);".to_string(),
        
        // CSS properties (treating them as keywords)
        ("css", "color") => "color - Sets the text color of an element. Example: color: blue; or color: #ff0000;".to_string(),
        ("css", "background") => "background - Shorthand for all background properties. Example: background: #f0f0f0 url('image.png') no-repeat;".to_string(),
        ("css", "margin") => "margin - Sets the outer spacing around an element. Example: margin: 10px; or margin: 10px 20px;".to_string(),
        ("css", "padding") => "padding - Sets the inner spacing within an element. Example: padding: 15px; or padding: 10px 5px;".to_string(),
        ("css", "border") => "border - Sets the border around an element. Example: border: 1px solid black;".to_string(),
        ("css", "width") => "width - Sets the width of an element. Example: width: 100px; or width: 50%;".to_string(),
        ("css", "height") => "height - Sets the height of an element. Example: height: 200px; or height: auto;".to_string(),
        ("css", "display") => "display - Controls how an element is displayed. Example: display: block; display: inline; display: flex;".to_string(),
        ("css", "position") => "position - Sets the positioning method for an element. Example: position: relative; position: absolute;".to_string(),
        ("css", "font-size") => "font-size - Sets the size of the font. Example: font-size: 16px; or font-size: 1.2em;".to_string(),
        
        // HTML tags (treating them as keywords)
        ("html", "div") => "div - Generic container element for grouping other elements. Example: <div class=\"container\">content</div>".to_string(),
        ("html", "span") => "span - Inline container for styling parts of text. Example: <span style=\"color: red;\">important text</span>".to_string(),
        ("html", "p") => "p - Paragraph element for text content. Example: <p>This is a paragraph of text.</p>".to_string(),
        ("html", "h1") => "h1 - Main heading element, largest and most important heading. Example: <h1>Main Title</h1>".to_string(),
        ("html", "a") => "a - Anchor/link element for navigation. Example: <a href=\"https://example.com\">Click here</a>".to_string(),
        ("html", "img") => "img - Image element for displaying pictures. Example: <img src=\"photo.jpg\" alt=\"Description\">".to_string(),
        ("html", "button") => "button - Interactive button element. Example: <button onclick=\"doSomething()\">Click me</button>".to_string(),
        ("html", "input") => "input - Form input element for user data entry. Example: <input type=\"text\" placeholder=\"Enter name\">".to_string(),
        ("html", "form") => "form - Container for form controls like inputs and buttons. Example: <form action=\"/submit\" method=\"post\">".to_string(),
        ("html", "ul") => "ul - Unordered list container for list items. Example: <ul><li>Item 1</li><li>Item 2</li></ul>".to_string(),
        ("html", "li") => "li - List item element, used inside ul or ol. Example: <li>List item content</li>".to_string(),
        
        // Generic fallback - provide meaningful default documentation
        _ => {
            match language {
                "rust" => {
                    // Provide detailed, educational documentation for unknown Rust elements
                    if keyword.chars().next().map_or(false, |c| c.is_uppercase()) {
                        if keyword.len() <= 3 {
                            format!("{} - Rust type or trait\n\nA built-in or commonly used type/trait. Types define data structures while traits define shared behavior. Example usage: let x: {} = ...;", keyword, keyword)
                        } else if keyword.ends_with("Error") || keyword.ends_with("Result") {
                            format!("{} - Rust error handling type\n\nPart of Rust's error handling system. Used with Result<T, E> for recoverable errors. Example: fn operation() -> Result<(), {}> {{ ... }}", keyword, keyword)
                        } else if keyword.contains("Iterator") || keyword.contains("Iter") {
                            format!("{} - Rust iterator type\n\nPart of Rust's iterator system for processing sequences of data. Provides lazy evaluation and functional programming patterns. Example: collection.iter().map(...).collect()", keyword)
                        } else if keyword.contains("Stream") || keyword.contains("Future") {
                            format!("{} - Rust async type\n\nPart of Rust's asynchronous programming model. Used with async/await for non-blocking operations. Example: let result = {}.await;", keyword, keyword.to_lowercase())
                        } else {
                            format!("{} - Rust type or trait\n\nA custom type, struct, enum, or trait defined in your codebase or imported from a crate. Types define data structures while traits define shared behavior across types.", keyword)
                        }
                    } else if keyword.ends_with('!') {
                        let macro_name = &keyword[..keyword.len()-1];
                        if macro_name.contains("assert") {
                            format!("{} - Rust assertion macro\n\nVerifies conditions at runtime. Panics if the condition is false, helping catch bugs during development. Example: {}(condition, \"error message\");", keyword, keyword)
                        } else if macro_name.contains("print") {
                            format!("{} - Rust output macro\n\nPrints formatted text to stdout/stderr. Part of Rust's formatted printing system. Example: {}(\"Hello {{}}\", name);", keyword, keyword)
                        } else if macro_name == "vec" {
                            format!("{} - Rust vector creation macro\n\nCreates a Vec<T> with initial values. More convenient than Vec::new() when you have initial data. Example: let v = vec![1, 2, 3];", keyword)
                        } else {
                            format!("{} - Rust macro\n\nA compile-time code generation tool that expands to other Rust code. Macros enable metaprogramming and reduce code duplication. Example: {}!(arguments);", keyword, keyword)
                        }
                    } else if keyword.len() <= 4 && keyword.chars().all(|c| c.is_lowercase() || c.is_numeric()) {
                        if keyword.contains("mut") {
                            format!("{} - Rust mutability keyword\n\nEnables modification of variables or references. Rust defaults to immutability for safety. Example: let mut x = 5; x = 10;", keyword)
                        } else if keyword == "ref" {
                            format!("{} - Rust reference keyword\n\nCreates a reference in pattern matching. Used to borrow instead of moving values. Example: match value {{ ref x => ... }}", keyword)
                        } else if keyword.starts_with("i") || keyword.starts_with("u") || keyword.starts_with("f") {
                            format!("{} - Rust numeric type\n\nA primitive integer or floating-point type. The number indicates bit width. Example: let x: {} = 42;", keyword, keyword)
                        } else {
                            format!("{} - Rust language keyword\n\nA reserved word with special meaning in Rust's syntax and semantics. Used for control flow, declarations, or language constructs.", keyword)
                        }
                    } else if keyword.contains("_") && keyword.chars().all(|c| c.is_lowercase() || c == '_') {
                        format!("{} - Rust function or method\n\nA function or method following Rust naming conventions. Functions perform operations and can return values. Example: let result = {}();", keyword, keyword)
                    } else {
                        format!("{} - Rust identifier\n\nA user-defined name in Rust code. Could be a variable, function, module, or custom type. Follows Rust's naming conventions and ownership rules.", keyword)
                    }
                },
                "javascript" | "typescript" => {
                    if keyword == "const" || keyword == "let" || keyword == "var" {
                        format!("{} - JavaScript variable declaration\n\nDeclares variables with different scoping and mutability rules. 'const' is immutable, 'let' is block-scoped, 'var' is function-scoped.", keyword)
                    } else if keyword == "async" || keyword == "await" {
                        format!("{} - JavaScript async keyword\n\nUsed for asynchronous programming with Promises. 'async' marks functions as asynchronous, 'await' pauses execution until Promise resolves.", keyword)
                    } else {
                        format!("{} - JavaScript/TypeScript language construct\n\nA reserved word or built-in feature used for programming logic, type definitions, or control flow in JavaScript/TypeScript.", keyword)
                    }
                },
                "python" => {
                    if keyword == "def" || keyword == "class" {
                        format!("{} - Python definition keyword\n\nDefines functions ('def') or classes ('class'). Used to create reusable code blocks and object-oriented structures.", keyword)
                    } else if keyword == "import" || keyword == "from" {
                        format!("{} - Python import keyword\n\nImports modules or specific items from modules. Essential for code organization and using external libraries.", keyword)
                    } else {
                        format!("{} - Python language keyword\n\nA reserved word with special meaning in Python. Used for control flow, object-oriented programming, or language constructs.", keyword)
                    }
                },
                "c" | "cpp" => {
                    if keyword == "int" || keyword == "char" || keyword == "float" || keyword == "double" {
                        format!("{} - C/C++ primitive type\n\nA fundamental data type for storing numbers or characters. Forms the basis of more complex data structures.", keyword)
                    } else if keyword == "struct" || keyword == "class" {
                        format!("{} - C/C++ composite type\n\nDefines custom data structures that group related data together. 'struct' members are public by default, 'class' members are private.", keyword)
                    } else {
                        format!("{} - C/C++ language keyword\n\nA reserved word used for type declarations, memory management, control flow, or object-oriented programming.", keyword)
                    }
                },
                "css" => {
                    if keyword.contains("color") || keyword.contains("background") {
                        format!("{} - CSS styling property\n\nControls visual appearance of elements. Used to set colors, backgrounds, and visual effects on web pages.", keyword)
                    } else if keyword.contains("margin") || keyword.contains("padding") || keyword.contains("border") {
                        format!("{} - CSS layout property\n\nControls spacing and positioning of elements. Part of the CSS box model for layout design.", keyword)
                    } else {
                        format!("{} - CSS property or value\n\nUsed for styling and layout of web elements. Controls appearance, positioning, and behavior of HTML elements.", keyword)
                    }
                },
                "html" => {
                    if keyword.starts_with('<') && keyword.ends_with('>') {
                        format!("{} - HTML element tag\n\nDefines the structure and content of web pages. Each tag has specific semantic meaning and styling properties.", keyword)
                    } else {
                        format!("{} - HTML element or attribute\n\nUsed for structuring web content and markup. Defines the semantic structure and metadata of web documents.", keyword)
                    }
                },
                _ => format!("{} - Programming construct\n\nA language-specific keyword, type, or identifier with particular meaning and usage patterns in {} programming.", keyword, language)
            }
        }
    }
}

/// Get enhanced documentation for code snippets
fn get_enhanced_snippet_documentation(language: &str, trigger: &str, content: &str) -> String {
    match (language, trigger) {
        ("rust", "fn") => "fn (snippet) - Function template. Creates a complete function definition with parameters and return type. Functions are the primary way to organize and reuse code in Rust.".to_string(),
        ("rust", "struct") => "struct (snippet) - Struct template. Creates a custom data type with named fields. Structs are used to group related data together.".to_string(),
        ("rust", "enum") => "enum (snippet) - Enum template. Creates a type with multiple variants. Enums are perfect for representing data that can be one of several types.".to_string(),
        ("rust", "impl") => "impl (snippet) - Implementation block template. Used to define methods and associated functions for structs and enums.".to_string(),
        ("rust", "trait") => "trait (snippet) - Trait template. Defines shared behavior that different types can implement, similar to interfaces in other languages.".to_string(),
        ("rust", "match") => "match (snippet) - Pattern matching template. Provides exhaustive pattern matching for enums and other types with automatic error checking.".to_string(),
        ("rust", "if") => "if (snippet) - Conditional template. Creates an if statement with optional else clause for branching logic.".to_string(),
        ("rust", "for") => "for (snippet) - For loop template. Creates iteration over collections, ranges, or iterators with automatic type inference.".to_string(),
        ("rust", "while") => "while (snippet) - While loop template. Creates a loop that continues while a condition remains true.".to_string(),
        ("rust", "loop") => "loop (snippet) - Infinite loop template. Creates a loop that runs forever until explicitly broken, useful for event loops.".to_string(),
        ("rust", "mod") => "mod (snippet) - Module template. Creates a module to organize code into logical namespaces and control visibility.".to_string(),
        ("rust", "test") => "test (snippet) - Unit test template. Creates a test function with the #[test] attribute for automated testing.".to_string(),
        ("rust", "derive") => "derive (snippet) - Derive macro template. Automatically implements common traits like Debug, Clone, PartialEq for structs and enums.".to_string(),
        ("rust", "macro") => "macro (snippet) - Macro definition template. Creates reusable code generation patterns for reducing repetition.".to_string(),
        
        ("javascript" | "typescript", "function") => "function (snippet) - Function declaration template. Creates a named function with parameters and return capability.".to_string(),
        ("javascript" | "typescript", "arrow") => "arrow (snippet) - Arrow function template. Creates a concise function syntax with lexical 'this' binding.".to_string(),
        ("javascript" | "typescript", "class") => "class (snippet) - Class template. Creates a class definition with constructor and methods for object-oriented programming.".to_string(),
        ("javascript" | "typescript", "if") => "if (snippet) - Conditional template. Creates an if statement with optional else for decision making.".to_string(),
        ("javascript" | "typescript", "for") => "for (snippet) - For loop template. Creates iteration with initialization, condition, and increment.".to_string(),
        ("javascript" | "typescript", "while") => "while (snippet) - While loop template. Creates a loop that continues while condition is true.".to_string(),
        ("javascript" | "typescript", "try") => "try (snippet) - Exception handling template. Creates try-catch block for handling errors gracefully.".to_string(),
        ("javascript" | "typescript", "async") => "async (snippet) - Async function template. Creates asynchronous function for handling promises and await operations.".to_string(),
        
        ("python", "def") => "def (snippet) - Function definition template. Creates a function with parameters, docstring, and return statement.".to_string(),
        ("python", "class") => "class (snippet) - Class template. Creates a class definition with __init__ method and basic structure.".to_string(),
        ("python", "if") => "if (snippet) - Conditional template. Creates if-elif-else structure for multi-branch decision making.".to_string(),
        ("python", "for") => "for (snippet) - For loop template. Creates iteration over sequences like lists, tuples, or ranges.".to_string(),
        ("python", "while") => "while (snippet) - While loop template. Creates a loop with condition and optional break logic.".to_string(),
        ("python", "try") => "try (snippet) - Exception handling template. Creates try-except-finally block for robust error handling.".to_string(),
        ("python", "with") => "with (snippet) - Context manager template. Creates with statement for automatic resource management.".to_string(),
        ("python", "lambda") => "lambda (snippet) - Lambda function template. Creates anonymous function for short, simple operations.".to_string(),
        
        ("html", "html5") => "html5 (snippet) - Complete HTML5 document template with DOCTYPE, head, and body structure.".to_string(),
        ("html", "div") => "div (snippet) - Division element template with class and content placeholder for layout structure.".to_string(),
        ("html", "form") => "form (snippet) - Form template with action, method, and common input elements for user data collection.".to_string(),
        ("html", "table") => "table (snippet) - Table template with header and body rows for displaying tabular data.".to_string(),
        
        ("css", "flexbox") => "flexbox (snippet) - Flexbox layout template with container and flex item properties for modern layouts.".to_string(),
        ("css", "grid") => "grid (snippet) - CSS Grid template with container and grid item properties for complex layouts.".to_string(),
        ("css", "animation") => "animation (snippet) - CSS animation template with keyframes and animation properties for dynamic effects.".to_string(),
        ("css", "media") => "media (snippet) - Media query template for responsive design across different screen sizes.".to_string(),
        
        _ => {
            // Enhanced fallback for unknown snippets based on language
            match language {
                "rust" => format!("{} (snippet) - Rust code template. Provides boilerplate code structure for common Rust programming patterns.", trigger),
                "javascript" | "typescript" => format!("{} (snippet) - JavaScript/TypeScript code template. Generates common code patterns and structures.", trigger),
                "python" => format!("{} (snippet) - Python code template. Creates reusable code blocks for common Python constructs.", trigger),
                "c" | "cpp" => format!("{} (snippet) - C/C++ code template. Provides standard code patterns for C/C++ development.", trigger),
                "css" => format!("{} (snippet) - CSS template. Generates common styling patterns and property combinations.", trigger),
                "html" => format!("{} (snippet) - HTML template. Creates standard HTML element structures and layouts.", trigger),
                _ => format!("{} (snippet) - Code template for {}. Generates commonly used code patterns and structures.", trigger, language)
            }
        }
    }
}
