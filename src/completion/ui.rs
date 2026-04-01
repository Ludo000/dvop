//! # Completion UI — Popup & Key Handling
//!
//! Builds the GTK4 popover that shows completion suggestions and handles
//! keyboard navigation (Up/Down to select, Enter to insert, Escape to close).
//!
//! ## Completion Pipeline
//!
//! 1. `setup_completion_shortcuts()` registers an `EventControllerKey` on
//!    the `sourceview5::View` that intercepts Ctrl+Space.
//! 2. `trigger_completion()` collects candidates from JSON data + buffer words.
//! 3. A `Popover` containing a `ListBox` of styled rows is positioned at the
//!    cursor and shown.
//! 4. When the user selects an item, `insert_completion()` replaces the
//!    current word prefix with the chosen text (or expands a snippet).
//!
//! ## Recursion Guard
//!
//! `COMPLETION_IN_PROGRESS` (`AtomicBool`) prevents re-entrant calls when
//! inserting a completion triggers another `changed` signal on the buffer.
//!
//! See FEATURES.md: Feature #111 — Code Completion
//! See FEATURES.md: Feature #113 — Snippet Expansion

use glib;
use gtk4::{
    gdk, pango, Box as GtkBox, Image, Label, ListBox, Orientation, Popover, ScrolledWindow,
};
use sourceview5::{prelude::*, Buffer, View};
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// Debug logging macro — disabled by default. Set DVOP_COMPLETION_DEBUG=1 to enable.
macro_rules! completion_debug {
    ($($arg:tt)*) => {
        // Uncomment the line below to enable completion debug logging:
        // std::println!($($arg)*);
    };
}

use super::ImportItem;

// Static flag to prevent recursive completion triggering
static COMPLETION_IN_PROGRESS: AtomicBool = AtomicBool::new(false);

// Track the current popover so we can clean it up before creating a new one.
// Without this, old popovers accumulate in the GTK widget tree causing hangs.
thread_local! {
    static CURRENT_POPOVER: RefCell<Option<Popover>> = RefCell::new(None);
}

/// Dismiss and unparent any existing completion popover.
fn dismiss_current_popover() {
    CURRENT_POPOVER.with(|cell| {
        if let Some(old) = cell.borrow_mut().take() {
            // popdown() triggers the `closed` signal whose handler calls unparent().
            // Do NOT call unparent() here — that would double-unparent.
            old.popdown();
        }
    });
}

/// Completion item types for different kinds of completions
#[derive(Clone, Debug)]
enum CompletionItem {
    Keyword(String),
    Snippet(String, String), // (trigger, content)
    BufferWord(String),
    ImportItem(ImportItem),
    ImportModule(String), // Module path
}

/// A scored completion candidate for intelligent ranking.
#[derive(Clone, Debug)]
struct ScoredItem {
    item: CompletionItem,
    score: i32,
}

// ── Cursor context analysis ──────────────────────────────────────

/// What kind of code position the cursor is at.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CursorContext {
    /// After `:`, `->`, `<`, generic/type annotation position
    TypePosition,
    /// Start of a line or after `{`, `;` — expecting a statement keyword
    StatementStart,
    /// After `=`, `(`, `,`, `return`, `=>` — expecting an expression/value
    ExpressionPosition,
    /// After `.` — method/field access (not yet supported by JSON data)
    DotAccess,
    /// Default / unknown
    General,
}

/// Analyse the text immediately before the cursor to determine context.
fn analyse_cursor_context(context_before_word: &str) -> CursorContext {
    // Get the text before the current word, trimmed
    let trimmed = context_before_word.trim_end();

    if trimmed.is_empty() {
        return CursorContext::StatementStart;
    }

    let last_char = trimmed.chars().last().unwrap_or(' ');

    // Dot access: `foo.`
    if last_char == '.' {
        return CursorContext::DotAccess;
    }

    // Type position: after `:`, `->`, `<` (generics)
    if last_char == ':' || last_char == '<' {
        return CursorContext::TypePosition;
    }
    if trimmed.ends_with("->") {
        return CursorContext::TypePosition;
    }

    // Expression position: after `=`, `(`, `,`, `=>`, `return`, `!`
    if matches!(last_char, '=' | '(' | ',' | '!' ) {
        return CursorContext::ExpressionPosition;
    }
    if trimmed.ends_with("=>") || trimmed.ends_with("return") {
        return CursorContext::ExpressionPosition;
    }

    // Statement start: beginning of line, after `{`, `}` , `;`
    if matches!(last_char, '{' | '}' | ';') {
        return CursorContext::StatementStart;
    }

    // Check if we're at the start of a logical line by looking for the last
    // newline and checking if everything after it is whitespace
    if let Some(nl) = trimmed.rfind('\n') {
        let after_nl = &trimmed[nl + 1..];
        if after_nl.trim().is_empty() {
            return CursorContext::StatementStart;
        }
    }

    CursorContext::General
}

// ── Fuzzy matching & scoring ─────────────────────────────────────

/// Compute a match score for `candidate` given the user-typed `query`.
/// Returns `None` if there is no reasonable match at all.
///
/// Scoring heuristics (higher = better):
///   +100  exact (case-insensitive) full match
///   +80   exact prefix match
///   +60   subsequence match with consecutive-char bonus
///   +20   case-sensitive prefix bonus
///   -1    per gap between matched characters (fuzzy penalty)
fn fuzzy_match_score(query: &str, candidate: &str) -> Option<i32> {
    if query.is_empty() {
        return Some(0);
    }

    let q_lower = query.to_lowercase();
    let c_lower = candidate.to_lowercase();

    // Exact full match
    if q_lower == c_lower {
        return Some(100);
    }

    // Prefix match
    if c_lower.starts_with(&q_lower) {
        let bonus = if candidate.starts_with(query) { 20 } else { 0 }; // case-sensitive bonus
        return Some(80 + bonus);
    }

    // Subsequence / fuzzy match  — every query char must appear in order
    let mut score: i32 = 60;
    let mut c_iter = c_lower.chars().enumerate().peekable();
    let mut prev_match_idx: Option<usize> = None;
    let mut consecutive = 0;

    for qch in q_lower.chars() {
        let mut found = false;
        for (ci, cch) in c_iter.by_ref() {
            if cch == qch {
                // Bonus for consecutive matched characters
                if let Some(prev) = prev_match_idx {
                    if ci == prev + 1 {
                        consecutive += 1;
                        score += 5 * consecutive; // accelerating bonus for runs
                    } else {
                        consecutive = 0;
                        score -= (ci - prev - 1).min(5) as i32; // gap penalty
                    }
                }
                // Bonus for matching at word boundaries (after `_` or uppercase transition)
                if ci == 0
                    || candidate.as_bytes().get(ci.wrapping_sub(1)).map_or(false, |&b| b == b'_')
                    || (candidate.as_bytes()[ci].is_ascii_uppercase()
                        && ci > 0
                        && candidate.as_bytes()[ci - 1].is_ascii_lowercase())
                {
                    score += 10;
                }
                prev_match_idx = Some(ci);
                found = true;
                break;
            }
        }
        if !found {
            return None; // query char not found — no match
        }
    }

    Some(score)
}

/// Context-aware bonus applied after the base fuzzy score.
fn context_bonus(item: &CompletionItem, ctx: CursorContext, keyword_type: Option<&str>, keyword_category: Option<&str>) -> i32 {
    match ctx {
        CursorContext::TypePosition => {
            match item {
                CompletionItem::Keyword(_) => {
                    // Boost type-related keywords in type position
                    match keyword_type {
                        Some("type") | Some("primitive") | Some("trait") => 30,
                        _ => match keyword_category {
                            Some("type_declaration") | Some("primitive_types") => 25,
                            _ => -10, // demote non-type keywords
                        },
                    }
                }
                CompletionItem::ImportItem(i) => {
                    match i.item_type.as_str() {
                        "struct" | "enum" | "trait" => 25,
                        _ => 0,
                    }
                }
                CompletionItem::BufferWord(_) => 5, // buffer words might be type names
                _ => 0,
            }
        }
        CursorContext::StatementStart => {
            match item {
                CompletionItem::Keyword(_) => {
                    match keyword_category {
                        Some("control_flow") | Some("function_declaration") | Some("variable_declaration") | Some("visibility") => 20,
                        Some("implementation") | Some("type_declaration") | Some("trait_declaration") => 15,
                        _ => 0,
                    }
                }
                CompletionItem::Snippet(_, _) => 15, // snippets are great at statement start
                _ => 0,
            }
        }
        CursorContext::ExpressionPosition => {
            match item {
                CompletionItem::Keyword(_) => {
                    match keyword_type {
                        Some("primitive") | Some("type") => 10, // constructors
                        _ => match keyword_category {
                            Some("literal_values") | Some("control_flow") => 10,
                            _ => -5,
                        },
                    }
                }
                CompletionItem::BufferWord(_) => 15, // likely a variable or function
                CompletionItem::Snippet(_, _) => 5,
                CompletionItem::ImportItem(i) => {
                    match i.item_type.as_str() {
                        "function" | "const" => 20,
                        "struct" | "enum" => 10,
                        _ => 0,
                    }
                }
                _ => 0,
            }
        }
        CursorContext::DotAccess => 0, // we have no method data from JSON
        CursorContext::General => 0,
    }
}

/// Get the primary name from a CompletionItem for display/sorting.
fn completion_item_name(item: &CompletionItem) -> String {
    match item {
        CompletionItem::Keyword(k) => k.clone(),
        CompletionItem::Snippet(t, _) => t.clone(),
        CompletionItem::BufferWord(w) => w.clone(),
        CompletionItem::ImportItem(i) => i.name.clone(),
        CompletionItem::ImportModule(m) => m.clone(),
    }
}

/// Extract the programming language from buffer language setting
fn get_buffer_language(buffer: &Buffer) -> String {
    let supported_languages = crate::completion::get_supported_languages();

    if let Some(language) = buffer.language() {
        let lang_id = language.id().to_string();
        let detected_lang = match lang_id.as_str() {
            "rust" => "rust".to_string(),
            "javascript" | "js" => "javascript".to_string(),
            "typescript" | "ts" => "javascript".to_string(), // Use JS completions for TS
            "python" | "python3" => "python".to_string(),
            "c" => "c".to_string(),
            "cpp" | "c++" => "cpp".to_string(),
            "java" => "java".to_string(),
            "html" => "html".to_string(),
            "css" => "css".to_string(),
            _ => "rust".to_string(), // Default to rust instead of generic
        };

        // Validate that the detected language is actually supported
        if supported_languages.contains(&detected_lang) {
            detected_lang
        } else {
            // Fall back to the first supported language if the detected one isn't available
            supported_languages.first()
                .unwrap_or(&"rust".to_string())
                .clone()
        }
    } else {
        // Default to first supported language when no language is detected
        supported_languages.first()
            .unwrap_or(&"rust".to_string())
            .clone()
    }
}

/// Registers manual completion on a `sourceview5::View` (no auto-trigger).
///
/// After calling this, the user can press Ctrl+Space or F1 to invoke
/// `trigger_completion()`. No automatic popup on typing.
///
/// See FEATURES.md: Feature #111 — Code Completion
pub fn setup_completion(source_view: &View) {
    completion_debug!("=== SETTING UP MANUAL COMPLETION ONLY ===");
    let buffer = source_view.buffer();

    // Cast buffer to SourceView Buffer
    if let Some(_source_buffer) = buffer.downcast_ref::<Buffer>() {
        // Manual completion via Ctrl+Space will be available
    } else {
        completion_debug!("WARNING: Could not setup completion - buffer is not a SourceView buffer");
    }
    completion_debug!("=== MANUAL COMPLETION SETUP COMPLETE ===");
}

/// Extends `setup_completion` with file-extension–specific logging.
///
/// Currently both paths use the same manual-only completion; this function
/// exists to allow per-language customisation in the future.
pub fn setup_completion_for_file(source_view: &View, file_path: Option<&Path>) {
    setup_completion(source_view);

    if let Some(path) = file_path {
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        completion_debug!("Setting up manual completion for file type: {}", extension);

        // Note: Only manual completion (Ctrl+Space) is available
        // No automatic completion providers are configured

        match extension {
            "rs" => {
                completion_debug!("Manual Rust completion enabled");
            }
            "js" | "ts" | "jsx" | "tsx" => {
                completion_debug!("Manual JavaScript/TypeScript completion enabled");
            }
            "py" => {
                completion_debug!("Manual Python completion enabled");
            }
            "java" => {
                completion_debug!("Manual Java completion enabled");
            }
            "c" | "cpp" | "cc" | "cxx" | "h" | "hpp" => {
                completion_debug!("Manual C/C++ completion enabled");
            }
            "html" | "htm" => {
                completion_debug!("Manual HTML completion enabled");
            }
            "css" | "scss" | "sass" | "less" => {
                completion_debug!("Manual CSS completion enabled");
            }
            _ => {
                completion_debug!("Manual completion enabled for file type: {}", extension);
            }
        }
    }
}

/// Shows the completion popup at the cursor position.
///
/// Gathers candidates from JSON keywords/snippets and buffer words,
/// filters them by the word prefix under the cursor, builds a `Popover`
/// with a `ListBox`, and displays it. Uses `COMPLETION_IN_PROGRESS` to
/// prevent re-entrant calls.
///
/// See FEATURES.md: Feature #111 — Code Completion
pub fn trigger_completion(source_view: &View) {
    // Bail out immediately if the extension is disabled
    if !crate::extensions::code_completion::is_enabled() {
        return;
    }

    // Check if completion is already in progress to prevent recursive calls
    if COMPLETION_IN_PROGRESS.swap(true, Ordering::SeqCst) {
        completion_debug!("Completion already in progress, skipping...");
        return;
    }

    completion_debug!("=== CREATING CUSTOM COMPLETION POPUP ===");
    completion_debug!("Function called successfully!");

    // Get current buffer and cursor position
    let buffer = source_view.buffer();
    let cursor_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&cursor_mark);

    // Get text around cursor for context
    let mut start_iter = cursor_iter;
    for _ in 0..50 {
        // Look back further for import context
        if start_iter.is_start() {
            break;
        }
        start_iter.backward_char();
    }

    let context_text = buffer.text(&start_iter, &cursor_iter, false);
    completion_debug!("Context around cursor: '{}'", context_text);

    // Check if we're in an import statement
    let is_import_context = detect_import_context(&context_text);
    completion_debug!("Import context detected: {}", is_import_context);

    let import_path = if is_import_context {
        extract_import_path(&context_text)
    } else {
        None
    };

    completion_debug!("Import path: {:?}", import_path);

    // Find the word prefix being typed - improved algorithm
    let mut word_start = cursor_iter;

    // Move backward to find the start of the current word
    while !word_start.is_start() {
        let prev_iter = {
            let mut temp = word_start;
            temp.backward_char();
            temp
        };
        let ch = prev_iter.char();

        completion_debug!(
            "Checking character at offset {}: '{}' (code: {})",
            prev_iter.offset(),
            ch,
            ch as u32
        );

        // Only include alphanumeric characters and underscores in words
        if ch.is_alphanumeric() || ch == '_' {
            word_start.backward_char();
            completion_debug!(
                "Moved back, word_start now at offset: {}",
                word_start.offset()
            );
        } else {
            // We've hit a non-word character, stop here
            completion_debug!("Found word boundary at character: '{}', stopping", ch);
            break;
        }
    }

    // Get the actual word being typed
    let prefix = buffer.text(&word_start, &cursor_iter, false);

    // Get the text *before* the current word for context analysis
    let line_start = {
        let mut ls = word_start;
        while !ls.is_start() {
            let mut tmp = ls;
            tmp.backward_char();
            if tmp.char() == '\n' {
                break;
            }
            ls = tmp;
        }
        ls
    };
    let text_before_word = buffer.text(&line_start, &word_start, false);
    let cursor_ctx = analyse_cursor_context(&text_before_word);
    completion_debug!("Cursor context: {:?}", cursor_ctx);

    // Get language-specific keywords
    let language = if let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() {
        get_buffer_language(source_buffer)
    } else {
        "generic".to_string()
    };

    completion_debug!("Language detected: {}", language);

    // Maximum number of suggestions to display
    const MAX_SUGGESTIONS: usize = 20;

    // ── Gather & score candidates ────────────────────────────────
    let mut scored: Vec<ScoredItem> = Vec::new();
    let prefix_str: String = prefix.to_string();

    {
        let mut manager = super::json_provider::get_completion_manager();
        let provider = manager.get_provider(&language);

        if is_import_context {
            completion_debug!("Processing import completions...");
            // Import context: use prefix matching (fuzzy is confusing for paths)
            if let Some(module_path) = import_path {
                if let Some(prov) = provider {
                    for item in prov.get_import_suggestions(&module_path) {
                        if let Some(base) = fuzzy_match_score(&prefix_str, &item.name) {
                            let ctx_bonus = context_bonus(
                                &CompletionItem::ImportItem(item.clone()),
                                cursor_ctx, None, None,
                            );
                            scored.push(ScoredItem {
                                item: CompletionItem::ImportItem(item),
                                score: base + ctx_bonus,
                            });
                        } else if prefix_str.is_empty() {
                            scored.push(ScoredItem {
                                item: CompletionItem::ImportItem(item),
                                score: 0,
                            });
                        }
                    }
                    for submodule in prov.get_submodules(&module_path) {
                        let full_path = if module_path.is_empty() {
                            submodule.clone()
                        } else {
                            format!("{}::{}", module_path, &submodule)
                        };
                        if let Some(base) = fuzzy_match_score(&prefix_str, &submodule) {
                            scored.push(ScoredItem {
                                item: CompletionItem::ImportModule(full_path),
                                score: base,
                            });
                        } else if prefix_str.is_empty() {
                            scored.push(ScoredItem {
                                item: CompletionItem::ImportModule(full_path),
                                score: 0,
                            });
                        }
                    }
                }
            } else if let Some(prov) = provider {
                for module in prov.find_matching_modules("") {
                    let last_seg = module.split("::").last().unwrap_or(&module);
                    if let Some(base) = fuzzy_match_score(&prefix_str, last_seg) {
                        scored.push(ScoredItem {
                            item: CompletionItem::ImportModule(module),
                            score: base,
                        });
                    } else if prefix_str.is_empty() {
                        scored.push(ScoredItem {
                            item: CompletionItem::ImportModule(module),
                            score: 0,
                        });
                    }
                }
            }
        } else {
            completion_debug!("Processing regular completions...");

            if let Some(prov) = provider {
                // Score keywords using fuzzy match + context
                for kw_data in prov.get_keyword_data() {
                    if let Some(base) = fuzzy_match_score(&prefix_str, &kw_data.keyword) {
                        let item = CompletionItem::Keyword(kw_data.keyword.clone());
                        let ctx_bonus = context_bonus(
                            &item,
                            cursor_ctx,
                            Some(&kw_data.r#type),
                            Some(&kw_data.category),
                        );
                        scored.push(ScoredItem { item, score: base + ctx_bonus });
                    } else if prefix_str.is_empty() {
                        let item = CompletionItem::Keyword(kw_data.keyword.clone());
                        let ctx_bonus = context_bonus(
                            &item,
                            cursor_ctx,
                            Some(&kw_data.r#type),
                            Some(&kw_data.category),
                        );
                        scored.push(ScoredItem { item, score: ctx_bonus });
                    }
                }

                // Score snippets
                for snippet in prov.get_snippet_data() {
                    if let Some(base) = fuzzy_match_score(&prefix_str, &snippet.trigger) {
                        let item = CompletionItem::Snippet(
                            snippet.trigger.clone(),
                            snippet.content.clone(),
                        );
                        let ctx_bonus = context_bonus(
                            &item,
                            cursor_ctx,
                            None,
                            Some(&snippet.category),
                        );
                        scored.push(ScoredItem { item, score: base + ctx_bonus });
                    } else if prefix_str.is_empty() {
                        let item = CompletionItem::Snippet(
                            snippet.trigger.clone(),
                            snippet.content.clone(),
                        );
                        let ctx_bonus = context_bonus(&item, cursor_ctx, None, Some(&snippet.category));
                        scored.push(ScoredItem { item, score: ctx_bonus });
                    }
                }
            }
        }
    } // mutex released here

    // ── Buffer words with proximity scoring ──────────────────────
    if !is_import_context && !prefix_str.is_empty() {
        let buffer_text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        let cursor_offset = cursor_iter.offset() as usize;
        let total_len = buffer_text.len();

        // Build a set of names already scored to avoid duplicates
        let mut seen: HashSet<String> = scored
            .iter()
            .map(|si| match &si.item {
                CompletionItem::Keyword(k) => k.clone(),
                CompletionItem::Snippet(s, _) => s.clone(),
                CompletionItem::BufferWord(w) => w.clone(),
                CompletionItem::ImportItem(i) => i.name.clone(),
                CompletionItem::ImportModule(m) => m.clone(),
            })
            .collect();

        // Track byte offset as we scan so we can compute proximity
        let mut byte_offset: usize = 0;
        for word in buffer_text.split_whitespace() {
            if scored.len() >= MAX_SUGGESTIONS * 3 {
                break; // gather a generous pool, we'll trim after sorting
            }
            // Advance byte_offset to find word position
            if let Some(pos) = buffer_text[byte_offset..].find(word) {
                byte_offset += pos;
            }
            let clean_word = word.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
            if clean_word.len() > 2 && clean_word != prefix_str {
                if let Some(base) = fuzzy_match_score(&prefix_str, clean_word) {
                    if seen.insert(clean_word.to_string()) {
                        // Proximity bonus: closer to cursor → higher score
                        let distance = if byte_offset < cursor_offset {
                            cursor_offset - byte_offset
                        } else {
                            byte_offset - cursor_offset
                        };
                        let proximity = if total_len > 0 {
                            // 0..20 bonus, max when distance==0
                            20 - ((distance * 20) / total_len.max(1)).min(20) as usize
                        } else {
                            0
                        };
                        scored.push(ScoredItem {
                            item: CompletionItem::BufferWord(clean_word.to_string()),
                            score: base + proximity as i32,
                        });
                    }
                }
            }
            byte_offset += word.len();
        }
    }

    // ── Sort by score (descending), then alphabetically for ties ─
    scored.sort_by(|a, b| {
        b.score.cmp(&a.score).then_with(|| {
            let name_a = completion_item_name(&a.item);
            let name_b = completion_item_name(&b.item);
            name_a.cmp(&name_b)
        })
    });
    scored.truncate(MAX_SUGGESTIONS);

    // Convert to display format
    let suggestions_with_content: Vec<(String, CompletionItem)> = scored
        .into_iter()
        .map(|si| {
            let display_text = match &si.item {
                CompletionItem::Keyword(k) => format!("{} (keyword)", k),
                CompletionItem::Snippet(trigger, _) => format!("{} (snippet)", trigger),
                CompletionItem::BufferWord(w) => w.clone(),
                CompletionItem::ImportItem(import_item) => {
                    format!("{} ({})", import_item.name, import_item.item_type)
                }
                CompletionItem::ImportModule(module) => {
                    let module_name = module.split("::").last().unwrap_or(module);
                    format!("{} (module)", module_name)
                }
            };
            (display_text, si.item)
        })
        .collect();

    completion_debug!(
        "Found {} completion suggestions: {:?}",
        suggestions_with_content.len(),
        suggestions_with_content
            .iter()
            .map(|(display, _)| display)
            .collect::<Vec<_>>()
    );

    if suggestions_with_content.is_empty() {
        // Dismiss any existing popover and reset
        dismiss_current_popover();
        COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
        return;
    }

    // Create custom completion popup
    create_completion_popup(
        source_view,
        &suggestions_with_content,
        &prefix,
        word_start.offset(),
        cursor_iter.offset(),
    );

    // Always reset the flag after the synchronous popup creation.
    // The flag only guards against re-entrant calls during this function.
    COMPLETION_IN_PROGRESS.store(false, Ordering::SeqCst);
}

/// Create a custom completion popup using GTK Popover
fn create_completion_popup(
    source_view: &View,
    suggestions_with_content: &[(String, CompletionItem)],
    _prefix: &str,
    word_start_offset: i32,
    cursor_offset: i32,
) {
    completion_debug!("=== CREATING POPOVER ===");

    // Get language for documentation
    let buffer = source_view.buffer();
    let language = if let Some(source_buffer) = buffer.downcast_ref::<sourceview5::Buffer>() {
        get_buffer_language(source_buffer)
    } else {
        "generic".to_string()
    };

    // Dismiss any previous popover before creating a new one
    dismiss_current_popover();

    // Create popover
    let popover = Popover::new();

    popover.set_parent(source_view);

    popover.set_autohide(true);

    // Get screen size to calculate appropriate popup dimensions
    let display = gdk::Display::default().expect("Failed to get display");
    let monitor = display
        .monitors()
        .item(0)
        .and_then(|obj| obj.downcast::<gdk::Monitor>().ok())
        .expect("Failed to get monitor");
    let geometry = monitor.geometry();
    let screen_width = geometry.width();
    let screen_height = geometry.height();

    completion_debug!("Screen size: {}x{}", screen_width, screen_height);

    // Calculate popup dimensions — cap at sane sizes for a completion popup
    let popup_width = ((screen_width as f32 * 0.45) as i32).min(700);
    // Max height: 50% of screen height, capped at 500px
    let max_height = ((screen_height as f32 * 0.5) as i32).min(500);

    completion_debug!(
        "Popup dimensions: width={}, height={}",
        popup_width, max_height
    );

    // Create scrolled window for suggestions with dynamic sizing
    let scrolled = ScrolledWindow::builder()
        .max_content_height(max_height)
        .max_content_width(popup_width)
        .min_content_width(popup_width)
        .propagate_natural_height(true)
        .propagate_natural_width(false)
        .hscrollbar_policy(gtk4::PolicyType::Never)
        .vscrollbar_policy(gtk4::PolicyType::Automatic)
        .overlay_scrolling(true)
        .build();
    completion_debug!("ScrolledWindow created (screen-adaptive)");

    // Create list box for suggestions
    let list_box = ListBox::builder()
        .selection_mode(gtk4::SelectionMode::Single)
        .show_separators(false)
        .build();

    // Set size based on screen dimensions
    list_box.set_size_request(popup_width, -1);

    completion_debug!("ListBox created with width: {}", popup_width);

    // Pre-compute CSS provider once (not per-row)
    let settings = crate::settings::get_settings();
    let font_size = settings.get_font_size();
    let css_provider = gtk4::CssProvider::new();
    let css_content = format!(
        ".completion-label {{ 
            font-weight: bold;
            font-size: {}pt;
            color: @theme_fg_color;
        }}
        .completion-doc {{ 
            font-size: {}pt; 
            font-weight: 700;
            color: alpha(@theme_fg_color, 0.75); 
            margin-left: 40px;
            line-height: 1.4;
            padding-right: 20px;
            padding-top: 2px;
            padding-bottom: 2px;
        }}",
        font_size, font_size
    );
    css_provider.load_from_data(&css_content);
    if let Some(display) = gdk::Display::default() {
        gtk4::style_context_add_provider_for_display(
            &display,
            &css_provider,
            gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }

    // Pre-compute label/doc widths once
    let label_width = 10;
    let doc_width = ((popup_width as f32 * 0.70 / 8.0) as i32).max(40);

    // Batch-fetch documentation for keywords and snippets in one lock
    let doc_cache: std::collections::HashMap<String, String> = {
        let mut manager = super::json_provider::get_completion_manager();
        let mut cache = std::collections::HashMap::new();
        if let Some(provider) = manager.get_provider(&language) {
            for (_, item) in suggestions_with_content.iter() {
                match item {
                    CompletionItem::Keyword(keyword) => {
                        cache.insert(keyword.clone(), provider.get_keyword_documentation(keyword));
                    }
                    CompletionItem::Snippet(trigger, _) => {
                        cache.insert(trigger.clone(), provider.get_snippet_documentation(trigger));
                    }
                    _ => {}
                }
            }
        }
        cache
    };

    // Add suggestions to list
    for (_i, (display_text, completion_item)) in suggestions_with_content.iter().enumerate() {

        // Create a horizontal box to hold icon, text, and documentation
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
            }
            CompletionItem::Snippet(_, _) => {
                // Use a template/code block icon for code snippets
                Image::from_icon_name("text-x-script-symbolic")
            }
            CompletionItem::BufferWord(_) => {
                // Use a text file icon for words from the current buffer
                Image::from_icon_name("text-x-generic-symbolic")
            }
            CompletionItem::ImportItem(import_item) => {
                // Use different icons based on import item type
                match import_item.item_type.as_str() {
                    "function" => Image::from_icon_name("applications-utilities-symbolic"),
                    "struct" | "enum" => Image::from_icon_name("document-properties-symbolic"),
                    "trait" => Image::from_icon_name("preferences-system-symbolic"),
                    "module" => Image::from_icon_name("folder-symbolic"),
                    "const" => Image::from_icon_name("dialog-information-symbolic"),
                    _ => Image::from_icon_name("insert-object-symbolic"),
                }
            }
            CompletionItem::ImportModule(_) => {
                // Use folder icon for modules
                Image::from_icon_name("folder-symbolic")
            }
        };

        // Set icon size
        icon.set_icon_size(gtk4::IconSize::Normal);

        // Create label for the main text with fixed width
        let label = Label::builder()
            .label(display_text)
            .xalign(0.0)
            .hexpand(false)
            .width_chars(label_width)
            .max_width_chars(label_width)
            .ellipsize(pango::EllipsizeMode::End)
            .build();

        // Add CSS class for bold styling
        label.add_css_class("completion-label");

        // Get documentation from batch cache or generate inline
        let doc_text = match completion_item {
            CompletionItem::Keyword(keyword) => {
                doc_cache.get(keyword).cloned().unwrap_or_else(|| keyword.clone())
            }
            CompletionItem::Snippet(trigger, _content) => {
                doc_cache.get(trigger).cloned().unwrap_or_else(|| trigger.clone())
            }
            CompletionItem::BufferWord(word) => {
                format!("{} - Word found in current buffer", word)
            }
            CompletionItem::ImportItem(import_item) => {
                format!(
                    "{} ({}) - {}",
                    import_item.name, import_item.item_type, import_item.description
                )
            }
            CompletionItem::ImportModule(module) => {
                format!("{} - Module available for import", module)
            }
        };

        // Create documentation label — use ellipsize instead of wrapping
        // to avoid expensive Pango line-break calculations
        let doc_label = Label::builder()
            .label(&doc_text)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(pango::EllipsizeMode::End)
            .max_width_chars(doc_width)
            .build();

        doc_label.add_css_class("completion-doc");

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
    completion_debug!("Popover content set with documentation");

    // Handle selection
    let buffer = source_view.buffer();
    let suggestions_clone = suggestions_with_content.to_vec();
    let popover_for_close = popover.clone();

    list_box.connect_row_activated(move |_, row| {
        let index = row.index() as usize;
        if let Some((_display_text, completion_item)) = suggestions_clone.get(index) {
            completion_debug!("Selected completion: {}", display_text);
            completion_debug!(
                "Replacing text from offset {} to {}",
                word_start_offset, cursor_offset
            );

            // Determine what to insert based on completion type
            let text_to_insert = match completion_item {
                CompletionItem::Keyword(keyword) => keyword.clone(),
                CompletionItem::BufferWord(word) => word.clone(),
                CompletionItem::Snippet(_, content) => {
                    // Process snippet - remove placeholders for now and replace with simple text
                    expand_snippet_content(content)
                }
                CompletionItem::ImportItem(import_item) => import_item.name.clone(),
                CompletionItem::ImportModule(module) => {
                    // For modules, just insert the module name (last component)
                    module.split("::").last().unwrap_or(module).to_string()
                }
            };

            // Replace the prefix with the selected suggestion/snippet
            let mut start_iter = buffer.iter_at_offset(word_start_offset);
            let mut end_iter = buffer.iter_at_offset(cursor_offset);

            buffer.delete(&mut start_iter, &mut end_iter);
            let mut insert_iter = buffer.iter_at_offset(word_start_offset);
            buffer.insert(&mut insert_iter, &text_to_insert);

            completion_debug!("Inserted: '{}'", text_to_insert);

            // Close popover
            popover_for_close.popdown();
        }
    });

    // Unparent popover when it closes so it doesn't leak in the widget tree
    popover.connect_closed(move |p| {
        CURRENT_POPOVER.with(|cell| {
            cell.borrow_mut().take();
        });
        p.unparent();
    });

    // Calculate cursor position for better popover positioning
    let buffer = source_view.buffer();
    let cursor_mark = buffer.get_insert();
    let cursor_iter = buffer.iter_at_mark(&cursor_mark);

    // Get cursor rectangle in buffer coordinates first
    let cursor_rect = source_view.iter_location(&cursor_iter);
    completion_debug!(
        "Cursor location (buffer coords): x={}, y={}, width={}, height={}",
        cursor_rect.x(),
        cursor_rect.y(),
        cursor_rect.width(),
        cursor_rect.height()
    );

    // Convert buffer coordinates to widget coordinates
    let (widget_x, widget_y) = source_view.buffer_to_window_coords(
        gtk4::TextWindowType::Widget,
        cursor_rect.x(),
        cursor_rect.y(),
    );

    completion_debug!(
        "Cursor location (widget coords): x={}, y={}",
        widget_x, widget_y
    );

    // Position the popover below the cursor
    let pointing_rect = gdk::Rectangle::new(
        widget_x,
        widget_y + cursor_rect.height(),
        cursor_rect.width().max(1), // Ensure minimum width
        1,
    );
    popover.set_pointing_to(Some(&pointing_rect));
    completion_debug!(
        "Popover positioned at widget coordinates: x={}, y={}",
        widget_x,
        widget_y + cursor_rect.height()
    );

    // Handle keyboard navigation in the popover
    let key_controller = gtk4::EventControllerKey::new();
    let popover_clone = popover.clone();
    let list_box_clone = list_box.clone();
    let scrolled_clone = scrolled.clone();

    key_controller.connect_key_pressed(move |_, keyval, _, _| {
        completion_debug!("Popover key pressed: {:?}", keyval);
        match keyval {
            gdk::Key::Escape => {
                popover_clone.popdown();
                glib::Propagation::Stop
            }
            gdk::Key::Return | gdk::Key::Tab => {
                completion_debug!("Return/Tab pressed - activating selection");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    selected_row.activate();
                }
                glib::Propagation::Stop
            }
            gdk::Key::Down => {
                completion_debug!("Down arrow - moving to next item");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    let next_index = selected_row.index() + 1;
                    if let Some(next_row) = list_box_clone.row_at_index(next_index) {
                        list_box_clone.select_row(Some(&next_row));
                        // Scroll to make the selected row visible
                        scroll_to_row(&scrolled_clone, &next_row);
                    }
                }
                glib::Propagation::Stop
            }
            gdk::Key::Up => {
                completion_debug!("Up arrow - moving to previous item");
                if let Some(selected_row) = list_box_clone.selected_row() {
                    let prev_index = selected_row.index().saturating_sub(1);
                    if let Some(prev_row) = list_box_clone.row_at_index(prev_index) {
                        list_box_clone.select_row(Some(&prev_row));
                        // Scroll to make the selected row visible
                        scroll_to_row(&scrolled_clone, &prev_row);
                    }
                }
                glib::Propagation::Stop
            }
            _ => glib::Propagation::Proceed,
        }
    });

    list_box.add_controller(key_controller);

    // Store popover reference and show it
    CURRENT_POPOVER.with(|cell| {
        *cell.borrow_mut() = Some(popover.clone());
    });
    popover.popup();

    // Give focus to the list box for keyboard navigation
    list_box.grab_focus();

    completion_debug!(
        "Custom completion popup displayed with {} suggestions and documentation",
        suggestions_with_content.len()
    );
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
    if !crate::extensions::code_completion::is_enabled() {
        return;
    }
    completion_debug!("Setting up completion keyboard shortcuts...");

    // Create key controller with high priority to ensure it gets events
    let key_controller = gtk4::EventControllerKey::new();
    key_controller.set_propagation_phase(gtk4::PropagationPhase::Capture);

    let source_view_clone = source_view.clone();
    key_controller.connect_key_pressed(move |_controller, keyval, _keycode, state| {
        // Debug key press
        completion_debug!("Key pressed: {:?}, state: {:?}", keyval, state);

        // Check for Ctrl+Space (manual trigger)
        if keyval == gdk::Key::space && state.contains(gdk::ModifierType::CONTROL_MASK) {
            completion_debug!("*** Ctrl+Space detected! Triggering manual completion ***");

            // Use timeout to ensure the key event is fully processed first
            let sv = source_view_clone.clone();
            glib::idle_add_local_once(move || {
                trigger_completion(&sv);
            });

            return glib::Propagation::Stop;
        }

        // Check for F1 key as alternative trigger for testing
        if keyval == gdk::Key::F1 {
            completion_debug!("*** F1 detected! Triggering test completion ***");
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

    completion_debug!("Completion keyboard shortcuts enabled:");
    completion_debug!("  - Ctrl+Space for manual trigger");
    completion_debug!("  - F1 for testing trigger");
    completion_debug!("  - Auto-completion has been DISABLED");
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

/// Detect if we're currently in an import context
fn detect_import_context(context: &str) -> bool {
    let trimmed = context.trim();

    // Find the current line (the last line in the context)
    let current_line = trimmed.lines().last().unwrap_or("");
    let current_line_trimmed = current_line.trim();

    // Rust: "use module::" syntax
    if current_line_trimmed.starts_with("use ") && current_line_trimmed.contains("::") {
        return true;
    }

    // JavaScript/TypeScript: "import { } from 'module'" or "import module from 'module'" or "const module = require('module')"
    if (current_line_trimmed.starts_with("import ") && current_line_trimmed.contains("from"))
        || (current_line_trimmed.contains("require("))
    {
        return true;
    }

    // Python: "from module import" or "import module"
    if (current_line_trimmed.starts_with("from ") && current_line_trimmed.contains(" import"))
        || (current_line_trimmed.starts_with("import ") && !current_line_trimmed.contains("from"))
    {
        return true;
    }

    false
}

/// Extract the module path from import context
fn extract_import_path(context: &str) -> Option<String> {
    let trimmed = context.trim();

    // Get the current line (the last line in the context)
    let current_line = trimmed.lines().last().unwrap_or("");
    let current_line_trimmed = current_line.trim();

    // Rust: "use module::" syntax
    if let Some(stripped) = current_line_trimmed.strip_prefix("use ") {
        let after_use = &stripped.trim();

        // Find the last :: to get the module path before it
        if let Some(last_double_colon) = after_use.rfind("::") {
            let module_path = &after_use[..last_double_colon];
            return Some(module_path.to_string());
        } else if after_use.is_empty() || after_use.chars().all(|c| c.is_whitespace()) {
            // Just "use " with nothing after - show root modules
            return Some(String::new());
        }
    }

    // JavaScript: "import { } from 'module'" or "import module from 'module'"
    if current_line_trimmed.starts_with("import ") && current_line_trimmed.contains("from") {
        // Extract module name from 'module' or "module"
        if let Some(from_pos) = current_line_trimmed.rfind("from") {
            let after_from = &current_line_trimmed[from_pos + 4..].trim();
            if let Some(quote_start) = after_from.find(['\'', '"']) {
                let quote_char = after_from.chars().nth(quote_start).unwrap();
                let module_part = &after_from[quote_start + 1..];
                if let Some(quote_end) = module_part.find(quote_char) {
                    return Some(module_part[..quote_end].to_string());
                }
            }
        }
    }

    // JavaScript: "const module = require('module')" or "import('module')"
    if let Some(require_pos) = current_line_trimmed.find("require(") {
        let after_require = &current_line_trimmed[require_pos + 8..];
        if let Some(quote_start) = after_require.find(['\'', '"']) {
            let quote_char = after_require.chars().nth(quote_start).unwrap();
            let module_part = &after_require[quote_start + 1..];
            if let Some(quote_end) = module_part.find(quote_char) {
                return Some(module_part[..quote_end].to_string());
            }
        }
    }

    // Python: "from module import" - extract the module before "import"
    if let Some(stripped) = current_line_trimmed.strip_prefix("from ") {
        if let Some(import_pos) = stripped.find(" import") {
            let module_path = &stripped[..import_pos].trim();
            return Some(module_path.to_string());
        }
    }

    // Python: "import module" - show available modules
    if let Some(stripped) = current_line_trimmed.strip_prefix("import ") {
        let module_part = stripped.trim();
        // If there's a dot, extract the base module
        if let Some(dot_pos) = module_part.rfind('.') {
            let base_module = &module_part[..dot_pos];
            return Some(base_module.to_string());
        } else if module_part.is_empty() {
            // Just "import " - show root modules
            return Some(String::new());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── fuzzy_match_score tests ──────────────────────────────────

    #[test]
    fn test_exact_match_scores_highest() {
        let score = fuzzy_match_score("let", "let").unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_exact_match_case_insensitive() {
        let score = fuzzy_match_score("Let", "let").unwrap();
        assert_eq!(score, 100);
    }

    #[test]
    fn test_prefix_match() {
        let score = fuzzy_match_score("pr", "println").unwrap();
        assert!(score >= 80, "prefix match should score >=80, got {}", score);
    }

    #[test]
    fn test_prefix_case_sensitive_bonus() {
        let exact_case = fuzzy_match_score("Hash", "HashMap").unwrap();
        let diff_case = fuzzy_match_score("hash", "HashMap").unwrap();
        assert!(
            exact_case > diff_case,
            "case-sensitive prefix ({}) should beat case-insensitive ({})",
            exact_case, diff_case
        );
    }

    #[test]
    fn test_fuzzy_subsequence() {
        // "hmap" should match "HashMap" (h, m, a, p all appear in order)
        let score = fuzzy_match_score("hmap", "HashMap");
        assert!(score.is_some(), "hmap should fuzzy-match HashMap");
    }

    #[test]
    fn test_fuzzy_camelcase_bonus() {
        // Matching word boundaries (capital H, M) should get bonuses
        let score = fuzzy_match_score("HM", "HashMap").unwrap();
        assert!(score > 60, "CamelCase boundary match should score well, got {}", score);
    }

    #[test]
    fn test_no_match_returns_none() {
        assert!(fuzzy_match_score("xyz", "let").is_none());
        assert!(fuzzy_match_score("abc", "HashMap").is_none());
    }

    #[test]
    fn test_empty_query_matches_everything() {
        assert_eq!(fuzzy_match_score("", "anything").unwrap(), 0);
    }

    #[test]
    fn test_prefix_beats_fuzzy() {
        let prefix = fuzzy_match_score("pr", "println").unwrap();
        let fuzzy = fuzzy_match_score("pl", "println").unwrap();
        assert!(
            prefix >= fuzzy,
            "prefix match ({}) should beat or equal fuzzy ({})",
            prefix, fuzzy
        );
    }

    #[test]
    fn test_longer_prefix_scores_same_base() {
        let short = fuzzy_match_score("p", "println").unwrap();
        let long = fuzzy_match_score("print", "println").unwrap();
        // Both are prefix matches, should both be >= 80
        assert!(short >= 80);
        assert!(long >= 80);
    }

    // ── analyse_cursor_context tests ─────────────────────────────

    #[test]
    fn test_context_type_position_colon() {
        assert_eq!(analyse_cursor_context("let x: "), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_type_position_arrow() {
        assert_eq!(analyse_cursor_context("fn foo() -> "), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_type_position_generic() {
        assert_eq!(analyse_cursor_context("Vec<"), CursorContext::TypePosition);
    }

    #[test]
    fn test_context_statement_start_empty() {
        assert_eq!(analyse_cursor_context(""), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_statement_start_brace() {
        assert_eq!(analyse_cursor_context("fn main() {"), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_statement_start_semicolon() {
        assert_eq!(analyse_cursor_context("let x = 5;"), CursorContext::StatementStart);
    }

    #[test]
    fn test_context_expression_equals() {
        assert_eq!(analyse_cursor_context("let x = "), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_expression_paren() {
        assert_eq!(analyse_cursor_context("foo("), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_expression_comma() {
        assert_eq!(analyse_cursor_context("foo(a, "), CursorContext::ExpressionPosition);
    }

    #[test]
    fn test_context_dot_access() {
        assert_eq!(analyse_cursor_context("my_var."), CursorContext::DotAccess);
    }

    #[test]
    fn test_context_newline_is_statement_start() {
        assert_eq!(analyse_cursor_context("let x = 5;\n    "), CursorContext::StatementStart);
    }

    // ── context_bonus tests ──────────────────────────────────────

    #[test]
    fn test_type_keyword_boosted_in_type_position() {
        let item = CompletionItem::Keyword("String".to_string());
        let bonus = context_bonus(&item, CursorContext::TypePosition, Some("type"), None);
        assert!(bonus > 0, "type keyword should get positive bonus in type position");
    }

    #[test]
    fn test_control_flow_demoted_in_type_position() {
        let item = CompletionItem::Keyword("for".to_string());
        let bonus = context_bonus(
            &item,
            CursorContext::TypePosition,
            Some("keyword"),
            Some("control_flow"),
        );
        assert!(bonus < 0, "control_flow keyword should be demoted in type position");
    }

    #[test]
    fn test_snippet_boosted_at_statement_start() {
        let item = CompletionItem::Snippet("fn".to_string(), "fn ${1:name}()".to_string());
        let bonus = context_bonus(&item, CursorContext::StatementStart, None, None);
        assert!(bonus > 0, "snippets should be boosted at statement start");
    }

    #[test]
    fn test_buffer_word_boosted_in_expression() {
        let item = CompletionItem::BufferWord("my_var".to_string());
        let bonus = context_bonus(&item, CursorContext::ExpressionPosition, None, None);
        assert!(bonus > 0, "buffer words should be boosted in expression position");
    }

    // ── completion_item_name tests ───────────────────────────────

    #[test]
    fn test_completion_item_name_keyword() {
        assert_eq!(completion_item_name(&CompletionItem::Keyword("let".to_string())), "let");
    }

    #[test]
    fn test_completion_item_name_snippet() {
        assert_eq!(
            completion_item_name(&CompletionItem::Snippet("fn".to_string(), "content".to_string())),
            "fn"
        );
    }

    #[test]
    fn test_completion_item_name_buffer_word() {
        assert_eq!(
            completion_item_name(&CompletionItem::BufferWord("my_var".to_string())),
            "my_var"
        );
    }
}
