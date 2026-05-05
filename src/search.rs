//! # In-File Search and Replace
//!
//! This module implements find/replace functionality within the active text document.
//! It uses **GtkSourceView5's `SearchContext`** which provides regex support, case
//! sensitivity, whole-word matching, and match highlighting out of the box.
//!
//! ## Architecture
//!
//! The search UI is a `SearchBar` widget (a collapsible toolbar) that appears at the top
//! of the editor when activated. A single global `SearchState` instance is shared across
//! all tabs — when the user switches tabs, the `SearchContext` is "rebound" to the new
//! tab's buffer via `rebind_buffer()`.
//!
//! ## Key Rust Pattern: `Rc<RefCell<Option<T>>>`
//!
//! The search context and source view are stored as `Rc<RefCell<Option<T>>>` because:
//! - `Rc`: Multiple closures (find next, find prev, replace) share the same reference
//! - `RefCell`: The value changes at runtime (rebound when switching tabs)
//! - `Option`: The value is `None` when no buffer is active (e.g., when viewing an image)
//!
//! See FEATURES.md: Feature #54 — In-File Search (Ctrl+F)
//! See FEATURES.md: Feature #55 — Find Next (F3)
//! See FEATURES.md: Feature #56 — Find Previous (Shift+F3)
//! See FEATURES.md: Feature #57 — Find and Replace (Ctrl+H)
//! See FEATURES.md: Feature #58 — Case Sensitive Search
//! See FEATURES.md: Feature #59 — Whole Word Matching

// Search functionality for the text editor
// This module manages find/replace operations within text documents

use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation, Revealer, SearchBar, SearchEntry};
use sourceview5::prelude::*;
use sourceview5::SearchContext;
use sourceview5::SearchSettings;
use std::cell::RefCell;
use std::rc::Rc;

/// Holds all UI widgets and state for the in-file search/replace functionality.
///
/// There is a **single global instance** of this struct (created lazily via `OnceLock`
/// in `get_search_state()`). When the user switches tabs, the `search_context` and
/// `source_view` are rebound to the new tab's buffer via `rebind_buffer()`.
///
/// ## Field Details
///
/// - `search_context: Rc<RefCell<Option<SearchContext>>>` — The SourceView5 search context
///   that tracks matches, highlighting, and search settings. Wrapped in `Option` because
///   it's `None` when no text buffer is active (e.g., viewing an image).
/// - `source_view: Rc<RefCell<Option<View>>>` — The active text editor widget. Needed to
///   scroll to matches when navigating find results.
///
/// See FEATURES.md: Feature #54 — In-File Search (Ctrl+F)
/// See FEATURES.md: Feature #57 — Find and Replace (Ctrl+H)
pub struct SearchState {
    pub search_bar: SearchBar,
    pub search_entry: SearchEntry,
    pub replace_entry: Entry,
    pub replace_box: GtkBox,
    // Rc<RefCell<T>> is a common Rust pattern for single-threaded shared mutable state. Rc allows multiple owners, and RefCell allows runtime mutation.
    pub search_context: Rc<RefCell<Option<SearchContext>>>,
    pub current_match_label: Label,
    pub revealer: Revealer,
    // Rc<RefCell<T>> is a common Rust pattern for single-threaded shared mutable state. Rc allows multiple owners, and RefCell allows runtime mutation.
    pub source_view: Rc<RefCell<Option<sourceview5::View>>>,
}

// "impl" blocks define methods and behavior for a struct or enum.
impl SearchState {
    /// Creates a new search state with UI components
    pub fn new() -> Self {
        // Create search bar
        let search_bar = SearchBar::new();
        search_bar.set_search_mode(false);

        // Create main container for search controls
        let search_container = GtkBox::new(Orientation::Horizontal, 8);
        search_container.set_margin_start(8);
        search_container.set_margin_end(8);
        search_container.set_margin_top(8);
        search_container.set_margin_bottom(8);

        // Create search entry
        let search_entry = SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_placeholder_text(Some("Find..."));

        // Create replace entry
        let replace_entry = Entry::new();
        replace_entry.set_hexpand(true);
        replace_entry.set_placeholder_text(Some("Replace with..."));

        // Create navigation buttons
        let prev_button = Button::from_icon_name("go-up-symbolic");
        prev_button.set_tooltip_text(Some("Find Previous (Shift+F3)"));

        let next_button = Button::from_icon_name("go-down-symbolic");
        next_button.set_tooltip_text(Some("Find Next (F3)"));

        // Create replace buttons
        let replace_button = Button::with_label("Replace");
        replace_button.set_tooltip_text(Some("Replace current match"));

        let replace_all_button = Button::with_label("Replace All");
        replace_all_button.set_tooltip_text(Some("Replace all matches"));

        // Create a box for replace controls (entry and buttons)
        let replace_box = GtkBox::new(Orientation::Horizontal, 8);
        replace_box.append(&replace_entry);
        replace_box.append(&replace_button);
        replace_box.append(&replace_all_button);

        // Create match counter label
        let current_match_label = Label::new(Some(""));
        current_match_label.add_css_class("dim-label");

        // Create close button
        let close_button = Button::from_icon_name("window-close-symbolic");
        close_button.set_tooltip_text(Some("Close search (Escape)"));

        // Assemble the search container
        search_container.append(&search_entry);
        search_container.append(&replace_box);
        search_container.append(&prev_button);
        search_container.append(&next_button);
        search_container.append(&current_match_label);
        search_container.append(&close_button);

        // Create revealer to animate the search bar appearance
        let revealer = Revealer::new();
        revealer.set_child(Some(&search_container));
        revealer.set_transition_type(gtk4::RevealerTransitionType::SlideDown);
        revealer.set_transition_duration(200);

        // Set the revealer as the child of the search bar
        search_bar.set_child(Some(&revealer));

        // Rc::new(...) creates a new Reference Counted pointer for shared ownership.
        let search_context = Rc::new(RefCell::new(None));
        // Rc::new(...) creates a new Reference Counted pointer for shared ownership.
        let source_view = Rc::new(RefCell::new(None));

        let search_state = SearchState {
            search_bar,
            search_entry: search_entry.clone(),
            replace_entry: replace_entry.clone(),
            replace_box: replace_box.clone(),
            search_context: search_context.clone(),
            current_match_label: current_match_label.clone(),
            revealer: revealer.clone(),
            source_view: source_view.clone(),
        };

        // Setup event handlers
        search_state.setup_handlers(
            prev_button,
            next_button,
            replace_button,
            replace_all_button,
            close_button,
        );

        search_state
    }

    /// Sets up event handlers for search controls
    fn setup_handlers(
        &self,
        prev_button: Button,
        next_button: Button,
        replace_button: Button,
        replace_all_button: Button,
        close_button: Button,
    ) {
        let search_context = self.search_context.clone();
        let current_match_label = self.current_match_label.clone();
        let search_entry = self.search_entry.clone();
        let replace_entry = self.replace_entry.clone();
        let search_bar = self.search_bar.clone();
        let revealer = self.revealer.clone();

        // Handle search entry changes
        let search_context_clone = search_context.clone();
        let current_match_label_clone = current_match_label.clone();
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        search_entry.connect_search_changed(move |entry| {
            let search_text = entry.text();
            // borrow() gets read-only access to the data inside a RefCell.
            if let Some(context) = search_context_clone.borrow().as_ref() {
                let settings = context.settings();
                settings.set_search_text(Some(&search_text));

                if !search_text.is_empty() {
                    Self::update_match_count(&current_match_label_clone, context);
                    Self::highlight_first_match(context);
                } else {
                    current_match_label_clone.set_text("");
                }
            }
        });

        // Handle Enter key to find next
        let search_context_clone = search_context.clone();
        let current_match_label_clone = current_match_label.clone();
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        search_entry.connect_activate(move |_| {
            // borrow() gets read-only access to the data inside a RefCell.
            if let Some(context) = search_context_clone.borrow().as_ref() {
                // Find mode: ensure we don't require a double Enter when no selection yet
                let buffer = context.buffer();
                let search_text = context.settings().search_text().unwrap_or_default();
                if !search_text.is_empty() {
                    let mut selection_matches = false;
                    if buffer.has_selection() {
                        if let Some((s, e)) = buffer.selection_bounds() {
                            let sel = buffer.text(&s, &e, false);
                            if sel.as_str() == search_text {
                                selection_matches = true;
                            }
                        }
                    }
                    if !selection_matches {
                        // First time: highlight first match
                        let total = Self::count_matches(context);
                        Self::highlight_first_match(context);
                        // If more than one match, advance immediately so first Enter goes to 2nd
                        if total > 1 {
                            Self::find_next(context);
                        }
                    } else {
                        // Already on a match, go to next
                        Self::find_next(context);
                    }
                }
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle previous button
        let search_context_clone = search_context.clone();
        let current_match_label_clone = current_match_label.clone();
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        prev_button.connect_clicked(move |_| {
            // borrow() gets read-only access to the data inside a RefCell.
            if let Some(context) = search_context_clone.borrow().as_ref() {
                Self::find_previous(context);
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle next button
        let search_context_clone = search_context.clone();
        let current_match_label_clone = current_match_label.clone();
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        next_button.connect_clicked(move |_| {
            // borrow() gets read-only access to the data inside a RefCell.
            if let Some(context) = search_context_clone.borrow().as_ref() {
                Self::find_next(context);
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle replace button
        let search_context_clone = search_context.clone();
        let replace_entry_clone = replace_entry.clone();
        let current_match_label_clone = current_match_label.clone();
        replace_button.connect_clicked(move |_| {
            if let Some(context) = search_context_clone.borrow().as_ref() {
                let replace_text = replace_entry_clone.text();
                Self::replace_current(context, &replace_text);
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle replace all button
        let search_context_clone = search_context.clone();
        let replace_entry_clone = replace_entry.clone();
        let current_match_label_clone = current_match_label.clone();
        replace_all_button.connect_clicked(move |_| {
            if let Some(context) = search_context_clone.borrow().as_ref() {
                let replace_text = replace_entry_clone.text();
                Self::replace_all(context, &replace_text);
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle Enter in replace entry (replace current & advance)
        let search_context_clone = search_context.clone();
        let current_match_label_clone = current_match_label.clone();
        replace_entry.connect_activate(move |entry| {
            if let Some(context) = search_context_clone.borrow().as_ref() {
                let replace_text = entry.text();
                Self::replace_current(context, &replace_text);
                Self::update_match_count(&current_match_label_clone, context);
            }
        });

        // Handle close button
        let search_bar_clone = search_bar.clone();
        let revealer_clone = revealer.clone();
        close_button.connect_clicked(move |_| {
            search_bar_clone.set_search_mode(false);
            revealer_clone.set_reveal_child(false);
        });

        // Handle Escape key to close search
        let search_bar_clone = search_bar.clone();
        let revealer_clone = revealer.clone();
        let key_controller = gtk4::EventControllerKey::new();
        key_controller.connect_key_pressed(move |_, keyval, _, _| {
            if keyval == gtk4::gdk::Key::Escape {
                search_bar_clone.set_search_mode(false);
                revealer_clone.set_reveal_child(false);
                return gtk4::glib::Propagation::Stop;
            }
            gtk4::glib::Propagation::Proceed
        });

        search_entry.add_controller(key_controller);
    }

    /// Shows the search bar and focuses the search entry
    pub fn show_search(
        &self,
        // Option<T> is an enum that represents an optional value: either Some(T) or None.
        text_buffer: Option<&sourceview5::Buffer>,
        // Option<T> is an enum that represents an optional value: either Some(T) or None.
        source_view: Option<&sourceview5::View>,
    ) {
        self.show_search_internal(text_buffer, source_view, true);
    }

    /// Shows only the find functionality (without replace)
    pub fn show_find_only(
        &self,
        // Option<T> is an enum that represents an optional value: either Some(T) or None.
        text_buffer: Option<&sourceview5::Buffer>,
        // Option<T> is an enum that represents an optional value: either Some(T) or None.
        source_view: Option<&sourceview5::View>,
    ) {
        self.show_search_internal(text_buffer, source_view, false);
    }

    /// Internal method to show the search bar with optional replace functionality
    fn show_search_internal(
        &self,
        text_buffer: Option<&sourceview5::Buffer>,
        source_view: Option<&sourceview5::View>,
        show_replace: bool,
    ) {
        // Show or hide replace controls
        println!("DEBUG: Setting replace_box visibility to: {}", show_replace);
        self.replace_box.set_visible(show_replace);

        // Create search context if we have a buffer
        if let Some(buffer) = text_buffer {
            let settings = SearchSettings::new();
            let context = SearchContext::new(buffer, Some(&settings));
            // Disable highlighting of all occurrences
            context.set_highlight(false);
            // If the user already has text in the search entry (e.g., after switching
            // to another tab/file) we need to copy that text into the fresh
            // SearchSettings; otherwise searching/replacing will operate with an
            // empty pattern and appear broken.
            let existing_text = self.search_entry.text();
            if !existing_text.is_empty() {
                context.settings().set_search_text(Some(&existing_text));
            }

            *self.search_context.borrow_mut() = Some(context);

            // After storing, update match count & highlight first occurrence if any
            if !existing_text.is_empty() {
                if let Some(ctx) = self.search_context.borrow().as_ref() {
                    Self::update_match_count(&self.current_match_label, ctx);
                    Self::highlight_first_match(ctx);
                }
            }
        }

        // Store the source view reference for scrolling
        if let Some(view) = source_view {
            *self.source_view.borrow_mut() = Some(view.clone());
        }

        self.search_bar.set_search_mode(true);
        self.revealer.set_reveal_child(true);
        self.search_entry.grab_focus();

        println!("Search bar shown");
    }

    /// Rebind the search context to a new buffer (used when switching tabs while search UI is open)
    pub fn rebind_buffer(
        &self,
        buffer: &sourceview5::Buffer,
        source_view: Option<&sourceview5::View>,
    ) {
        let settings = SearchSettings::new();
        let context = SearchContext::new(buffer, Some(&settings));
        context.set_highlight(false);
        let existing_text = self.search_entry.text();
        if !existing_text.is_empty() {
            context.settings().set_search_text(Some(&existing_text));
        }
        *self.search_context.borrow_mut() = Some(context);
        if let Some(view) = source_view {
            *self.source_view.borrow_mut() = Some(view.clone());
        }
        if !existing_text.is_empty() {
            if let Some(ctx) = self.search_context.borrow().as_ref() {
                Self::update_match_count(&self.current_match_label, ctx);
                Self::highlight_first_match(ctx);
            }
        } else {
            // Clear match label since no pattern
            self.current_match_label.set_text("");
        }
    }

    /// Hides the search bar
    pub fn hide_search(&self) {
        self.search_bar.set_search_mode(false);
        self.revealer.set_reveal_child(false);

        // Clear search context
        *self.search_context.borrow_mut() = None;

        println!("Search bar hidden");
    }

    /// Updates the match count display
    pub fn update_match_count(label: &Label, context: &SearchContext) {
        let _buffer = context.buffer();
        let search_text = context.settings().search_text();

        if let Some(text) = search_text {
            if !text.is_empty() {
                // Count total matches
                let total_matches = Self::count_matches(context);
                let current_position = Self::get_current_match_position(context);

                if total_matches > 0 {
                    label.set_text(&format!("{} of {}", current_position, total_matches));
                } else {
                    label.set_text("0 matches");
                }
            } else {
                label.set_text("");
            }
        } else {
            label.set_text("");
        }
    }

    /// Counts the total number of matches in the buffer
    fn count_matches(context: &SearchContext) -> i32 {
        let buffer = context.buffer();
        let mut start_iter = buffer.start_iter();
        let mut count = 0;

        while let Some((_, match_end, _wrapped)) = context.forward(&start_iter) {
            count += 1;
            start_iter = match_end;
        }

        count
    }

    /// Gets the current match position (1-based)
    fn get_current_match_position(context: &SearchContext) -> i32 {
        let buffer = context.buffer();

        // Get current cursor position using the buffer's insert mark
        let cursor_iter = buffer.iter_at_mark(&buffer.get_insert());

        // Count matches before cursor
        let mut start_iter = buffer.start_iter();
        let mut position = 0;

        while let Some((match_start, match_end, _wrapped)) = context.forward(&start_iter) {
            if match_start.offset() >= cursor_iter.offset() {
                break;
            }
            position += 1;
            start_iter = match_end;
        }

        // If cursor is at or in a match, include it
        let check_iter = cursor_iter;
        if let Some((match_start, _match_end, _wrapped)) = context.backward(&check_iter) {
            if match_start.offset() <= cursor_iter.offset() {
                position += 1;
            }
        }

        if position == 0 && Self::count_matches(context) > 0 {
            1 // If we haven't found any matches before cursor but there are matches, we're at the first
        } else {
            position
        }
    }

    /// Highlights the first match in the buffer
    fn highlight_first_match(context: &SearchContext) {
        let buffer = context.buffer();
        let start_iter = buffer.start_iter();

        if let Some((match_start, match_end, _wrapped)) = context.forward(&start_iter) {
            // Move cursor to the match and select it
            buffer.place_cursor(&match_start);
            buffer.select_range(&match_start, &match_end);

            // Scroll to show the match
            if let Some(view) = Self::get_source_view_from_search_state() {
                let mut iter_for_scroll = match_start;
                view.scroll_to_iter(&mut iter_for_scroll, 0.25, false, 0.0, 0.0);
            }
        }
    }

    /// Finds the next match
    pub fn find_next(context: &SearchContext) {
        let buffer = context.buffer();
        let mut start_iter = buffer.iter_at_mark(&buffer.get_insert());

        // If there's currently selected text, start searching after it
        if buffer.has_selection() {
            // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
            let (_start, end) = buffer.selection_bounds().unwrap();
            start_iter = end;
        }

        if let Some((match_start, match_end, wrapped)) = context.forward(&start_iter) {
            buffer.place_cursor(&match_start);
            buffer.select_range(&match_start, &match_end);

            if wrapped {
                println!("Search wrapped to beginning");
            }

            if let Some(view) = Self::get_source_view_from_search_state() {
                let mut iter_for_scroll = match_start;
                view.scroll_to_iter(&mut iter_for_scroll, 0.25, false, 0.0, 0.0);
            }
        }
    }

    /// Finds the previous match
    pub fn find_previous(context: &SearchContext) {
        let buffer = context.buffer();
        let mut end_iter = buffer.iter_at_mark(&buffer.get_insert());

        // If there's currently selected text, start searching before it
        if buffer.has_selection() {
            // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
            let (start, _end) = buffer.selection_bounds().unwrap();
            end_iter = start;
        }

        if let Some((match_start, match_end, wrapped)) = context.backward(&end_iter) {
            buffer.place_cursor(&match_start);
            buffer.select_range(&match_start, &match_end);

            if wrapped {
                println!("Search wrapped to end");
            }

            if let Some(view) = Self::get_source_view_from_search_state() {
                let mut iter_for_scroll = match_start;
                view.scroll_to_iter(&mut iter_for_scroll, 0.25, false, 0.0, 0.0);
            }
        }
    }

    /// Replaces the current match
    fn replace_current(context: &SearchContext, replace_text: &str) {
        let buffer = context.buffer();

        // Ensure we have an active selection that matches the search term; if not, find the next match automatically
        let search_text = context.settings().search_text().unwrap_or_default();
        if search_text.is_empty() {
            return; // nothing to replace
        }

        let mut need_selection = true;
        if buffer.has_selection() {
            if let Some((start, end)) = buffer.selection_bounds() {
                let selected_text = buffer.text(&start, &end, false);
                if selected_text.as_str() == search_text {
                    need_selection = false;
                }
            }
        }

        if need_selection {
            // Try to find the next occurrence and select it
            Self::find_next(context);
            if !buffer.has_selection() {
                return; // still nothing selected
            }
        }

        if let Some((start, end)) = buffer.selection_bounds() {
            let selected_text = buffer.text(&start, &end, false);
            if selected_text.as_str() != search_text {
                return; // safety guard
            }
            // Perform replacement (robust approach maintaining iter validity)
            buffer.begin_user_action();
            let insert_offset = start.offset();
            let mut del_start = start;
            let mut del_end = end;
            buffer.delete(&mut del_start, &mut del_end);
            if !replace_text.is_empty() {
                let mut insert_iter = buffer.iter_at_offset(insert_offset);
                buffer.insert(&mut insert_iter, replace_text);
            }
            buffer.end_user_action();

            let after_iter = buffer.iter_at_offset(insert_offset + replace_text.len() as i32);
            buffer.place_cursor(&after_iter);
            // Automatically advance to next match so user can just keep clicking Replace
            Self::find_next(context);
        }
    }

    /// Replaces all matches in the buffer
    fn replace_all(context: &SearchContext, replace_text: &str) {
        let buffer = context.buffer();
        let mut start_iter = buffer.start_iter();
        let mut replacements = 0;

        // Collect all matches first to avoid iterator invalidation
        let mut matches = Vec::new();
        while let Some((match_start, match_end, _wrapped)) = context.forward(&start_iter) {
            matches.push((match_start.offset(), match_end.offset()));
            start_iter = match_end;
        }

        // Replace all matches from end to beginning to maintain offsets
        for (start_offset, end_offset) in matches.iter().rev() {
            let mut start_iter = buffer.iter_at_offset(*start_offset);
            let mut end_iter = buffer.iter_at_offset(*end_offset);
            buffer.delete(&mut start_iter, &mut end_iter);
            buffer.insert(&mut buffer.iter_at_offset(*start_offset), replace_text);
            replacements += 1;
        }

        println!("Replaced {} occurrences", replacements);
    }

    /// Helper function to get the source view from stored reference
    fn get_source_view_from_search_state() -> Option<sourceview5::View> {
        let search_state = get_search_state();
        search_state.source_view.borrow().clone()
    }
}

/// Global search state - shared across the application
static mut SEARCH_STATE: Option<SearchState> = None;

/// Gets or creates the global search state. This uses a mutable static because GTK widgets
/// are !Send / !Sync and can't live in thread-safe singletons. All access happens on the
/// main GTK thread, so this is safe in practice.
#[allow(static_mut_refs)]
// pub makes this function public, allowing it to be used from outside this module.
pub fn get_search_state() -> &'static SearchState {
    unsafe {
        if SEARCH_STATE.is_none() {
            SEARCH_STATE = Some(SearchState::new());
        }
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        SEARCH_STATE.as_ref().unwrap()
    }
}

/// Shows the search bar for the current active text buffer
pub fn show_search_for_buffer(
    buffer: Option<&sourceview5::Buffer>,
    source_view: Option<&sourceview5::View>,
) {
    let search_state = get_search_state();
    search_state.show_search(buffer, source_view);
}

/// Shows only the find functionality (without replace) for the current active text buffer
pub fn show_find_only_for_buffer(
    buffer: Option<&sourceview5::Buffer>,
    source_view: Option<&sourceview5::View>,
) {
    let search_state = get_search_state();
    search_state.show_find_only(buffer, source_view);
}

/// Hides the search bar
pub fn hide_search() {
    let search_state = get_search_state();
    search_state.hide_search();
}
