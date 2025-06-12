// Code completion functionality for the text editor
// This module provides intelligent code completion using GTK SourceView's completion framework

use sourceview5::{prelude::*, CompletionWords, View, Buffer, CompletionProvider};
use gtk4::{gdk, Popover, ListBox, Label, ScrolledWindow};
use glib;
use std::collections::HashMap;
use std::path::Path;
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

// Static flag to prevent recursive completion triggering
static COMPLETION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

/// Language-specific keyword definitions
static LANGUAGE_KEYWORDS: Lazy<HashMap<&'static str, Vec<&'static str>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Rust keywords
    map.insert("rust", vec![
        "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
        "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod",
        "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super",
        "trait", "true", "type", "unsafe", "use", "where", "while", "abstract", "become",
        "box", "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield",
        "String", "Vec", "HashMap", "HashSet", "Option", "Result", "println!", "eprintln!", "format!",
        "vec!", "Some", "None", "Ok", "Err", "Clone", "Copy", "Debug", "Default", "PartialEq",
        "Eq", "PartialOrd", "Ord", "Hash", "Display", "From", "Into", "TryFrom", "TryInto",
        "Box", "Arc", "Rc", "RefCell", "Cell", "Mutex", "RwLock", "thread", "spawn", "join"
    ]);
    
    // JavaScript/TypeScript keywords
    map.insert("javascript", vec![
        "abstract", "arguments", "boolean", "break", "byte", "case", "catch", "char", "class",
        "const", "continue", "debugger", "default", "delete", "do", "double", "else", "enum",
        "eval", "export", "extends", "false", "final", "finally", "float", "for", "function",
        "goto", "if", "implements", "import", "in", "instanceof", "int", "interface", "let",
        "long", "native", "new", "null", "package", "private", "protected", "public", "return",
        "short", "static", "super", "switch", "synchronized", "this", "throw", "throws",
        "transient", "true", "try", "typeof", "var", "void", "volatile", "while", "with", "yield",
        "console", "document", "window", "Array", "Object", "Promise", "async", "await",
        "setTimeout", "setInterval", "clearTimeout", "clearInterval", "fetch", "JSON",
        "localStorage", "sessionStorage", "getElementById", "querySelector", "addEventListener"
    ]);
    
    // Python keywords
    map.insert("python", vec![
        "False", "None", "True", "and", "as", "assert", "async", "await", "break", "class",
        "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global",
        "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise",
        "return", "try", "while", "with", "yield", "print", "len", "range", "enumerate",
        "zip", "map", "filter", "sorted", "reversed", "sum", "min", "max", "abs", "round",
        "int", "float", "str", "list", "tuple", "dict", "set", "bool", "type", "isinstance",
        "hasattr", "getattr", "setattr", "delattr", "open", "file", "input", "raw_input",
        "__init__", "__str__", "__repr__", "__len__", "__getitem__", "__setitem__", "__iter__"
    ]);
    
    // C/C++ keywords
    map.insert("c", vec![
        "auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else",
        "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register",
        "restrict", "return", "short", "signed", "sizeof", "static", "struct", "switch",
        "typedef", "union", "unsigned", "void", "volatile", "while", "_Alignas", "_Alignof",
        "_Atomic", "_Static_assert", "_Noreturn", "_Thread_local", "_Generic", "printf",
        "scanf", "malloc", "free", "sizeof", "NULL", "true", "false", "bool", "size_t",
        "FILE", "stdin", "stdout", "stderr", "fopen", "fclose", "fread", "fwrite", "fprintf"
    ]);
    
    // C++ additional keywords
    map.insert("cpp", vec![
        "alignas", "alignof", "and", "and_eq", "asm", "atomic_cancel", "atomic_commit",
        "atomic_noexcept", "auto", "bitand", "bitor", "bool", "break", "case", "catch",
        "char", "char8_t", "char16_t", "char32_t", "class", "compl", "concept", "const",
        "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await",
        "co_return", "co_yield", "decltype", "default", "delete", "do", "double",
        "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
        "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace",
        "new", "noexcept", "not", "not_eq", "nullptr", "operator", "or", "or_eq",
        "private", "protected", "public", "reflexpr", "register", "reinterpret_cast",
        "requires", "return", "short", "signed", "sizeof", "static", "static_assert",
        "static_cast", "struct", "switch", "synchronized", "template", "this", "thread_local",
        "throw", "true", "try", "typedef", "typeid", "typename", "union", "unsigned",
        "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq",
        "std", "cout", "cin", "endl", "vector", "string", "map", "set", "pair", "make_pair",
        "unique_ptr", "shared_ptr", "weak_ptr", "make_unique", "make_shared"
    ]);
    
    // Java keywords
    map.insert("java", vec![
        "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class",
        "const", "continue", "default", "do", "double", "else", "enum", "extends", "final",
        "finally", "float", "for", "goto", "if", "implements", "import", "instanceof", "int",
        "interface", "long", "native", "new", "package", "private", "protected", "public",
        "return", "short", "static", "strictfp", "super", "switch", "synchronized", "this",
        "throw", "throws", "transient", "try", "void", "volatile", "while", "true", "false",
        "null", "String", "System", "out", "println", "print", "Scanner", "ArrayList",
        "HashMap", "HashSet", "List", "Map", "Set", "Collection", "Iterator", "Exception",
        "Object", "Class", "Thread", "Runnable", "Integer", "Double", "Boolean", "Character"
    ]);
    
    // HTML keywords
    map.insert("html", vec![
        "html", "head", "title", "body", "div", "span", "p", "a", "img", "ul", "ol", "li",
        "h1", "h2", "h3", "h4", "h5", "h6", "table", "tr", "td", "th", "thead", "tbody",
        "form", "input", "button", "select", "option", "textarea", "label", "fieldset",
        "legend", "nav", "header", "footer", "section", "article", "aside", "main", "figure",
        "figcaption", "audio", "video", "source", "canvas", "svg", "script", "style", "link",
        "meta", "br", "hr", "strong", "em", "b", "i", "u", "small", "mark", "del", "ins",
        "sub", "sup", "blockquote", "cite", "q", "abbr", "address", "time", "code", "pre",
        "kbd", "samp", "var", "dfn", "data", "output", "progress", "meter", "details", "summary",
        "iframe", "embed", "object", "param", "area", "map", "track", "wbr", "ruby", "rt", "rp"
    ]);
    
    // CSS keywords and properties
    map.insert("css", vec![
        "color", "background", "background-color", "background-image", "background-repeat",
        "background-position", "background-size", "border", "border-color", "border-style",
        "border-width", "border-radius", "margin", "padding", "width", "height", "min-width",
        "max-width", "min-height", "max-height", "display", "position", "top", "right",
        "bottom", "left", "float", "clear", "overflow", "z-index", "font", "font-family",
        "font-size", "font-weight", "font-style", "text-align", "text-decoration",
        "text-transform", "line-height", "letter-spacing", "word-spacing", "white-space",
        "vertical-align", "list-style", "table-layout", "border-collapse", "border-spacing",
        "caption-side", "empty-cells", "cursor", "outline", "visibility", "opacity",
        "box-shadow", "text-shadow", "transform", "transition", "animation", "flex",
        "grid", "align-items", "justify-content", "flex-direction", "flex-wrap",
        "align-content", "justify-items", "place-items", "gap", "row-gap", "column-gap"
    ]);
    
    map
});

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

/// Populate buffer with language keywords for better completion
/// This adds some sample words directly to the buffer to ensure completion has content
pub fn populate_buffer_with_keywords(buffer: &Buffer) {
    let language = get_buffer_language(buffer);
    println!("Setting up keywords for language: {}", language);
    
    // Add some basic completion words to the buffer content
    // This ensures the CompletionWords provider has something to work with
    let basic_words = match language.as_str() {
        "rust" => "fn let mut if else match Vec String Option Result println macro struct impl trait",
        "javascript" => "function var let const if else for while class async await",
        "python" => "def class if else for while import from return print",
        _ => "function if else for while class"
    };
    
    // Get current buffer text
    let current_text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
    
    // Only add words if buffer is mostly empty
    if current_text.trim().len() < 100 {
        let mut end_iter = buffer.end_iter();
        let comment_words = format!("\n/* Completion: {} */\n", basic_words);
        buffer.insert(&mut end_iter, &comment_words);
        
        // Move cursor back to a reasonable position
        let start_iter = buffer.start_iter();
        buffer.place_cursor(&start_iter);
    }
    
    if let Some(keywords) = LANGUAGE_KEYWORDS.get(language.as_str()) {
        println!("Added {} keywords as completion hints for {}", keywords.len(), language);
    } else {
        println!("No keywords found for language: {}", language);
    }
}

/// Custom completion words provider that includes language keywords
pub struct CustomCompletionWords {
    completion_words: CompletionWords,
    buffer: Buffer,
}

impl CustomCompletionWords {
    pub fn new(buffer: &Buffer) -> Self {
        let completion_words = CompletionWords::new(Some("Custom Keywords"));
        
        // Register the buffer for word completion
        completion_words.register(buffer);
        
        // Add language-specific keywords
        let language = get_buffer_language(buffer);
        if let Some(keywords) = LANGUAGE_KEYWORDS.get(language.as_str()) {
            for _keyword in keywords {
                // CompletionWords doesn't have add_word method in this version
                // Instead, it automatically extracts words from registered buffers
                // We can create a temporary buffer with keywords and register it
                let keyword_buffer = Buffer::new(None);
                let keywords_text = keywords.join(" ");
                keyword_buffer.set_text(&keywords_text);
                completion_words.register(&keyword_buffer);
            }
            println!("Added {} keywords for language: {}", keywords.len(), language);
        }
        
        Self {
            completion_words,
            buffer: buffer.clone(),
        }
    }
    
    pub fn provider(&self) -> &CompletionWords {
        &self.completion_words
    }
}

/// Custom completion provider for language-specific keywords and snippets
use sourceview5::CompletionContext;

#[derive(Debug)]
pub struct CustomKeywordProvider {
    language: String,
}

impl CustomKeywordProvider {
    pub fn new(language: String) -> Self {
        Self { language }
    }
    
    pub fn get_proposals(&self, _context: &CompletionContext) -> Vec<String> {
        let mut proposals = Vec::new();
        
        // Get keywords for the current language
        if let Some(keywords) = LANGUAGE_KEYWORDS.get(self.language.as_str()) {
            // Return all keywords for now (we can add filtering later)
            proposals.extend(keywords.iter().map(|k| k.to_string()));
        }
        
        proposals
    }
}

/// Setup completion for a source view with proper provider configuration  
pub fn setup_completion(source_view: &View) {
    println!("=== SETTING UP COMPLETION ===");
    let completion = source_view.completion();
    let buffer = source_view.buffer();
    
    // Cast buffer to SourceView Buffer
    if let Some(source_buffer) = buffer.downcast_ref::<Buffer>() {
        println!("Buffer cast successful, setting up completion...");
        
        // Populate buffer with keywords for better completion
        populate_buffer_with_keywords(source_buffer);
        
        // Create completion words provider (this will automatically scan buffer words)
        let completion_words = CompletionWords::new(Some("Buffer Words"));
        completion_words.register(source_buffer);
        
        // Note: minimum_word_size property removed as it's not available in this GTK version
        
        // Add the words completion provider
        completion.add_provider(&completion_words.upcast::<CompletionProvider>());
        println!("Added CompletionWords provider to completion");
        
        // Configure completion behavior for better visibility and responsiveness
        completion.set_page_size(15); // Show 15 proposals per page
        completion.set_show_icons(true); // Show icons in completion
        completion.set_remember_info_visibility(true); // Remember info panel state
        
        // Create a second completion provider with language keywords
        let language = get_buffer_language(source_buffer);
        if let Some(keywords) = LANGUAGE_KEYWORDS.get(language.as_str()) {
            // Create a temporary buffer with all keywords separated by spaces and newlines
            let keyword_buffer = Buffer::new(None);
            let mut keywords_text = String::new();
            for (i, keyword) in keywords.iter().enumerate() {
                if i > 0 {
                    keywords_text.push(' ');
                    if i % 10 == 0 { // Add newlines every 10 keywords for better parsing
                        keywords_text.push('\n');
                    }
                }
                keywords_text.push_str(keyword);
            }
            keywords_text.push('\n'); // End with newline
            keyword_buffer.set_text(&keywords_text);
            
            // Create another CompletionWords for keywords
            let keyword_completion = CompletionWords::new(Some("Language Keywords"));
            keyword_completion.register(&keyword_buffer);
            
            completion.add_provider(&keyword_completion.upcast::<CompletionProvider>());
            println!("Added keyword completion provider with {} keywords for {}", keywords.len(), language);
        }
        
        // Note: auto-complete property not available in this GTK version
        
        println!("Completion configuration complete");
        println!("Code completion enabled for source view");
    } else {
        println!("WARNING: Could not setup completion - buffer is not a SourceView buffer");
    }
    println!("=== COMPLETION SETUP COMPLETE ===");
}

/// Enhanced completion setup with file-specific behavior
pub fn setup_completion_for_file(source_view: &View, file_path: Option<&Path>) {
    setup_completion(source_view);
    
    if let Some(path) = file_path {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        println!("Setting up enhanced completion for file type: {}", extension);
        
        // The completion provider will automatically detect the language
        // from the syntax highlighting and provide appropriate completions
        
        // Enable additional features based on file type
        let completion = source_view.completion();
        
        match extension {
            "rs" => {
                // Rust-specific completion settings
                completion.set_page_size(20); // More proposals for Rust
                println!("Enhanced Rust completion enabled");
            },
            "js" | "ts" | "jsx" | "tsx" => {
                // JavaScript/TypeScript-specific settings
                println!("Enhanced JavaScript/TypeScript completion enabled");
            },
            "py" => {
                // Python-specific settings
                completion.set_page_size(12);
                println!("Enhanced Python completion enabled");
            },
            "java" => {
                // Java-specific settings
                completion.set_page_size(18);
                println!("Enhanced Java completion enabled");
            },
            "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" => {
                // C/C++-specific settings
                completion.set_page_size(16);
                println!("Enhanced C/C++ completion enabled");
            },
            "html" | "htm" => {
                // HTML-specific settings
                println!("Enhanced HTML completion enabled");
            },
            "css" | "scss" | "sass" | "less" => {
                // CSS-specific settings
                println!("Enhanced CSS completion enabled");
            },
            _ => {
                println!("Generic completion enabled for file type: {}", extension);
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
    
    // Collect completion suggestions
    let mut suggestions = Vec::new();
    let prefix_lower = prefix.to_lowercase();
    
    // Add language keywords that match the prefix
    if let Some(keywords) = LANGUAGE_KEYWORDS.get(language.as_str()) {
        println!("Found {} keywords for language {}", keywords.len(), language);
        for keyword in keywords {
            if prefix.is_empty() || keyword.to_lowercase().starts_with(&prefix_lower) {
                suggestions.push(keyword.to_string());
            }
        }
    } else {
        println!("No keywords found for language: {}", language);
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
            && !suggestions.contains(&clean_word.to_string()) {
            suggestions.push(clean_word.to_string());
        }
    }
    
    // If no prefix, add some default suggestions only if no other suggestions found
    if suggestions.is_empty() {
        if prefix.is_empty() {
            // Only show default suggestions if there's no prefix at all
            suggestions = vec![
                "fn".to_string(),
                "let".to_string(), 
                "mut".to_string(),
                "if".to_string(),
                "else".to_string(),
                "match".to_string(),
                "Vec".to_string(),
                "String".to_string(),
                "println!".to_string(),
                "struct".to_string()
            ];
            println!("Using default suggestions for empty prefix");
        } else {
            println!("No suggestions found for prefix: '{}'", prefix);
            // Don't add test_completion fallback if we have a specific prefix
            return;
        }
    }
    
    // Sort suggestions
    suggestions.sort();
    suggestions.truncate(15); // Limit to 15 suggestions
    
    println!("Found {} completion suggestions: {:?}", suggestions.len(), suggestions);
    
    if suggestions.is_empty() {
        println!("No completion suggestions found - not showing popup");
        // Reset the completion flag since we're not showing a popup
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
        return;
    }
    
    // Create custom completion popup
    println!("About to create popup...");
    create_completion_popup(source_view, &suggestions, &prefix, word_start.offset(), cursor_iter.offset());
    
    // Reset the completion flag after a short delay
    glib::timeout_add_local_once(std::time::Duration::from_millis(200), move || {
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
    });
    
    println!("=== CUSTOM COMPLETION POPUP CREATED ===");
}

/// Create a custom completion popup using GTK Popover
fn create_completion_popup(source_view: &View, suggestions: &[String], _prefix: &str, word_start_offset: i32, cursor_offset: i32) {
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
        .propagate_natural_height(true)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .build();
    println!("ScrolledWindow created");
    
    // Create list box for suggestions
    let list_box = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .build();
    println!("ListBox created");
    
    // Add suggestions to list
    for (i, suggestion) in suggestions.iter().enumerate() {
        println!("Adding suggestion {}: {}", i, suggestion);
        let label = Label::builder()
            .label(suggestion)
            .xalign(0.0)
            .margin_start(8)
            .margin_end(8)
            .margin_top(4)
            .margin_bottom(4)
            .build();
        
        list_box.append(&label);
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
    let suggestions_clone = suggestions.to_vec();
    let popover_for_close = popover.clone();
    
    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some(suggestion) = suggestions_clone.get(index) {
            println!("Selected completion: {}", suggestion);
            println!("Replacing text from offset {} to {}", word_start_offset, cursor_offset);
            
            // Get the text that will be replaced for debugging
            let start_iter = buffer.iter_at_offset(word_start_offset);
            let end_iter = buffer.iter_at_offset(cursor_offset);
            let text_to_replace = buffer.text(&start_iter, &end_iter, false);
            println!("Text being replaced: '{}'", text_to_replace);
            
            // Replace the prefix with the selected suggestion
            let mut start_iter = buffer.iter_at_offset(word_start_offset);
            let mut end_iter = buffer.iter_at_offset(cursor_offset);
            
            buffer.delete(&mut start_iter, &mut end_iter);
            let mut insert_iter = buffer.iter_at_offset(word_start_offset);
            buffer.insert(&mut insert_iter, suggestion);
            
            println!("Inserted: '{}'", suggestion);
            
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
    
    // Get cursor rectangle in source view coordinates
    let cursor_rect = source_view.iter_location(&cursor_iter);
    println!("Cursor location: x={}, y={}, width={}, height={}", 
             cursor_rect.x(), cursor_rect.y(), cursor_rect.width(), cursor_rect.height());
    
    // Position popover at cursor location
    let pointing_rect = gdk::Rectangle::new(
        cursor_rect.x(),
        cursor_rect.y() + cursor_rect.height(), // Position below cursor
        1,
        1
    );
    popover.set_pointing_to(Some(&pointing_rect));
    println!("Popover positioned at cursor location");
    
    // Handle keyboard navigation in the popover
    let key_controller = gtk4::EventControllerKey::new();
    let popover_clone = popover.clone();
    let list_box_clone = list_box.clone();
    
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
    
    println!("Custom completion popup displayed with {} suggestions", suggestions.len());
}

/// Setup keyboard shortcuts for completion with improved auto-trigger
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
    
    // Also set up buffer change monitoring for auto-completion triggers
    let buffer = source_view.buffer();
    let source_view_for_buffer = source_view.clone();
    
    buffer.connect_changed(move |buffer| {
        // Check if completion is already in progress
        if COMPLETION_IN_PROGRESS.load(Ordering::SeqCst) {
            return;
        }
        
        // Get the cursor position and check what was just typed
        let cursor_mark = buffer.get_insert();
        let cursor_iter = buffer.iter_at_mark(&cursor_mark);
        
        // Look at the character before the cursor
        if !cursor_iter.is_start() {
            let mut prev_iter = cursor_iter;
            prev_iter.backward_char();
            let prev_char = prev_iter.char();
            
            // Check if this character should trigger completion
            let should_trigger = match prev_char {
                '.' => {
                    println!("Detected '.' - checking for auto-trigger");
                    true
                },
                ':' => {
                    // Check if it's :: (Rust path separator)
                    if !prev_iter.is_start() {
                        let mut prev_prev = prev_iter;
                        prev_prev.backward_char();
                        let is_double_colon = prev_prev.char() == ':';
                        if is_double_colon {
                            println!("Detected '::' - checking for auto-trigger");
                        }
                        is_double_colon
                    } else {
                        false
                    }
                },
                '>' => {
                    // Check if it's -> (arrow operator)
                    if !prev_iter.is_start() {
                        let mut prev_prev = prev_iter;
                        prev_prev.backward_char();
                        let is_arrow = prev_prev.char() == '-';
                        if is_arrow {
                            println!("Detected '->' - checking for auto-trigger");
                        }
                        is_arrow
                    } else {
                        false
                    }
                },
                _ => false,
            };
            
            if should_trigger {
                println!("*** Auto-trigger completion after character: '{}' ***", prev_char);
                let sv = source_view_for_buffer.clone();
                glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                    trigger_completion(&sv);
                });
            }
        }
    });
    
    println!("Completion keyboard shortcuts enabled:");
    println!("  - Ctrl+Space for manual trigger");
    println!("  - F1 for testing trigger"); 
    println!("  - Auto-trigger on ., ::, ->");
}

/// Create completion documentation
pub fn get_completion_documentation(keyword: &str, language: &str) -> String {
    match (language, keyword) {
        // Rust documentation
        ("rust", "fn") => "fn keyword - Define a function\n\nSyntax: fn name(params) -> return_type { body }".to_string(),
        ("rust", "struct") => "struct keyword - Define a structure\n\nSyntax: struct Name { field: Type }".to_string(),
        ("rust", "enum") => "enum keyword - Define an enumeration\n\nSyntax: enum Name { Variant1, Variant2 }".to_string(),
        ("rust", "impl") => "impl keyword - Implement methods for a type\n\nSyntax: impl Type { fn method(&self) {} }".to_string(),
        ("rust", "match") => "match keyword - Pattern matching\n\nSyntax: match expr { pattern => result }".to_string(),
        ("rust", "Vec") => "Vec<T> - A growable array type\n\nExample: let v = Vec::new(); v.push(1);".to_string(),
        ("rust", "Option") => "Option<T> - Represents optional values\n\nVariants: Some(T), None".to_string(),
        ("rust", "Result") => "Result<T, E> - Error handling type\n\nVariants: Ok(T), Err(E)".to_string(),
        
        // JavaScript documentation
        ("javascript", "function") => "function keyword - Define a function\n\nSyntax: function name(params) { body }".to_string(),
        ("javascript", "class") => "class keyword - Define a class\n\nSyntax: class Name { constructor() {} }".to_string(),
        ("javascript", "async") => "async keyword - Define an asynchronous function\n\nSyntax: async function name() { await expr; }".to_string(),
        ("javascript", "Promise") => "Promise - Represents asynchronous operations\n\nMethods: then(), catch(), finally()".to_string(),
        ("javascript", "console") => "console object - Provides debugging methods\n\nMethods: log(), error(), warn(), info()".to_string(),
        
        // Python documentation
        ("python", "def") => "def keyword - Define a function\n\nSyntax: def name(params): body".to_string(),
        ("python", "class") => "class keyword - Define a class\n\nSyntax: class Name: def __init__(self): pass".to_string(),
        ("python", "import") => "import keyword - Import modules\n\nSyntax: import module or from module import name".to_string(),
        ("python", "list") => "list - Mutable sequence type\n\nMethods: append(), extend(), insert(), remove()".to_string(),
        ("python", "dict") => "dict - Mapping type (dictionary)\n\nMethods: keys(), values(), items(), get()".to_string(),
        
        // C/C++ documentation
        ("c", "printf") => "printf() - Print formatted output\n\nSyntax: printf(\"format\", args...)".to_string(),
        ("c", "malloc") => "malloc() - Allocate memory\n\nSyntax: void* malloc(size_t size)".to_string(),
        ("cpp", "std") => "std namespace - Standard library namespace\n\nContains: vector, string, map, cout, cin, etc.".to_string(),
        ("cpp", "vector") => "std::vector - Dynamic array\n\nMethods: push_back(), pop_back(), size(), clear()".to_string(),
        
        // Java documentation
        ("java", "class") => "class keyword - Define a class\n\nSyntax: public class Name { }".to_string(),
        ("java", "public") => "public modifier - Makes members accessible everywhere".to_string(),
        ("java", "private") => "private modifier - Restricts access to the same class".to_string(),
        ("java", "String") => "String class - Immutable character sequence\n\nMethods: length(), charAt(), substring()".to_string(),
        
        // HTML documentation
        ("html", "div") => "<div> element - Generic container\n\nUsage: <div>content</div>".to_string(),
        ("html", "span") => "<span> element - Inline container\n\nUsage: <span>text</span>".to_string(),
        ("html", "form") => "<form> element - User input form\n\nAttributes: action, method".to_string(),
        
        // CSS documentation
        ("css", "display") => "display property - Controls element layout\n\nValues: block, inline, flex, grid, none".to_string(),
        ("css", "position") => "position property - Element positioning\n\nValues: static, relative, absolute, fixed, sticky".to_string(),
        ("css", "flex") => "flex property - Flexible box layout\n\nShorthand for: flex-grow flex-shrink flex-basis".to_string(),
        
        _ => format!("{} - {} keyword/identifier", keyword, language),
    }
}

/// Common code snippets for different languages
static CODE_SNIPPETS: Lazy<HashMap<&'static str, Vec<(&'static str, &'static str)>>> = Lazy::new(|| {
    let mut map = HashMap::new();
    
    // Rust snippets
    map.insert("rust", vec![
        ("fn", "fn ${1:function_name}(${2:parameters}) -> ${3:return_type} {\n    ${4:// body}\n}"),
        ("struct", "struct ${1:Name} {\n    ${2:field}: ${3:Type},\n}"),
        ("impl", "impl ${1:Type} {\n    ${2:// methods}\n}"),
        ("match", "match ${1:expression} {\n    ${2:pattern} => ${3:result},\n}"),
        ("if", "if ${1:condition} {\n    ${2:// body}\n}"),
        ("for", "for ${1:item} in ${2:iterator} {\n    ${3:// body}\n}"),
        ("while", "while ${1:condition} {\n    ${2:// body}\n}"),
        ("loop", "loop {\n    ${1:// body}\n    break;\n}"),
        ("main", "fn main() {\n    ${1:// code}\n}"),
        ("test", "#[test]\nfn ${1:test_name}() {\n    ${2:// test code}\n}"),
    ]);
    
    // JavaScript snippets
    map.insert("javascript", vec![
        ("function", "function ${1:name}(${2:parameters}) {\n    ${3:// body}\n}"),
        ("arrow", "(${1:parameters}) => {\n    ${2:// body}\n}"),
        ("class", "class ${1:Name} {\n    constructor(${2:parameters}) {\n        ${3:// constructor}\n    }\n}"),
        ("if", "if (${1:condition}) {\n    ${2:// body}\n}"),
        ("for", "for (${1:let i = 0}; ${2:i < length}; ${3:i++}) {\n    ${4:// body}\n}"),
        ("foreach", "${1:array}.forEach((${2:item}) => {\n    ${3:// body}\n});"),
        ("promise", "new Promise((resolve, reject) => {\n    ${1:// async code}\n});"),
    ]);
    
    // Python snippets
    map.insert("python", vec![
        ("def", "def ${1:function_name}(${2:parameters}):\n    ${3:pass}"),
        ("class", "class ${1:ClassName}:\n    def __init__(self${2:, parameters}):\n        ${3:pass}"),
        ("if", "if ${1:condition}:\n    ${2:pass}"),
        ("for", "for ${1:item} in ${2:iterable}:\n    ${3:pass}"),
        ("while", "while ${1:condition}:\n    ${2:pass}"),
        ("try", "try:\n    ${1:pass}\nexcept ${2:Exception} as e:\n    ${3:pass}"),
        ("with", "with ${1:expression} as ${2:variable}:\n    ${3:pass}"),
    ]);
    
    map
});
