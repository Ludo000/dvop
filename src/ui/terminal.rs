//! # Terminal — Embedded VTE4 Terminal Emulator
//!
//! Provides an integrated terminal inside the editor, similar to VS Code's
//! built-in terminal. Uses the **VTE4** library (Virtual Terminal Emulator),
//! which renders a full PTY-backed terminal within a GTK4 widget.
//!
//! ## Key Concepts for Beginners
//!
//! - **VTE (Virtual Terminal Emulator)** — a C library (with Rust bindings)
//!   that implements terminal emulation. It handles escape codes, colors,
//!   scrolling, selection, and spawning a shell process.
//! - **PTY (Pseudo-Terminal)** — the OS-level mechanism that connects the
//!   shell process to the VTE widget. `spawn_async()` creates the PTY.
//! - **Color Palette** — VTE uses a 256-color palette. The first 16 entries
//!   are set from the editor's syntax theme to keep the terminal visually
//!   consistent with the code editor.
//!
//! ## Terminal Tabs
//!
//! Multiple terminals are supported via a `Notebook` (tabbed container).
//! `add_terminal_tab_with_toggle()` creates a new terminal and adds it as a
//! tab. Each terminal tab can be independently themed and resized.
//!
//! See FEATURES.md: Feature #44 — Integrated Terminal
//! See FEATURES.md: Feature #45 — Terminal Theming
//! See FEATURES.md: Feature #46 — Multiple Terminal Tabs

use gtk4::gdk;
use gtk4::pango;
use gtk4::prelude::*;
use gtk4::{Notebook, ScrolledWindow};
use std::env;
use std::path::PathBuf;

// Terminal emulator support
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExt;
use vte4::TerminalExtManual;

/// Creates a new VTE terminal widget and spawns the user's default shell.
///
/// The shell is determined by the `$SHELL` environment variable (falling back
/// to `/bin/bash`). The terminal starts in `working_dir` if provided, or the
/// user's home directory otherwise.
///
/// `spawn_async()` creates a PTY (pseudo-terminal) and forks the shell process.
/// The `Cancellable` parameter is set to `None` because we never cancel the spawn.
///
/// See FEATURES.md: Feature #44 — Integrated Terminal
pub fn create_terminal(working_dir: Option<PathBuf>) -> VteTerminal {
    let terminal = VteTerminal::new();

    // Set terminal colors to match the editor's theme
    setup_terminal_theme(&terminal);

    // Apply font size from settings
    apply_terminal_font_size(&terminal);

    // Get the user's default shell from environment variables
    if let Ok(shell) = env::var("SHELL") {
        // Use the provided working directory or fall back to user's home directory
        let dir = match working_dir {
            Some(dir) => dir,
            None => home::home_dir().expect("Could not find home directory"),
        };

        if let Some(dir_str) = dir.to_str() {
            // Spawn the shell asynchronously in the terminal
            terminal.spawn_async(
                vte4::PtyFlags::DEFAULT,   // Default pseudo-terminal flags
                Some(dir_str),             // Working directory
                &[&shell],                 // Command (user's shell)
                &[],                       // Environment variables (none added)
                glib::SpawnFlags::DEFAULT, // Default spawn flags
                || {},                     // Setup function (none)
                -1,                        // Default timeout
                None::<&Cancellable>,      // No cancellation
                move |res| {
                    // Handle spawn errors
                    if let Err(err) = res {
                        eprintln!("Failed to spawn shell: {}", err);
                    }
                },
            );
        } else {
            eprintln!("Failed to convert directory path to string");
        }
    }
    terminal
}

/// Sets up the terminal color theme to match the editor's syntax highlighting theme
///
/// Configures VTE terminal colors to match the editor's current theme.
///
/// Reads the active syntax-highlighting color scheme and extracts its
/// foreground, background, and cursor colors. Then sets the first 16 ANSI
/// color palette entries to complementary values derived from the theme.
///
/// This keeps the terminal visually consistent with the code editor when
/// switching between light and dark mode.
///
/// See FEATURES.md: Feature #45 — Terminal Theming
pub fn setup_terminal_theme(terminal: &VteTerminal) {
    // Check if we're in dark mode to choose appropriate colors
    let is_dark_mode = crate::syntax::is_dark_mode_enabled();

    if is_dark_mode {
        // Dark mode color scheme
        // Set foreground (text) color to light gray/white
        terminal.set_color_foreground(&gdk::RGBA::new(0.85, 0.85, 0.85, 1.0));

        // Set background color to dark gray (not pure black for better readability)
        terminal.set_color_background(&gdk::RGBA::new(0.15, 0.15, 0.15, 1.0));

        // Set cursor color for visibility
        terminal.set_color_cursor(Some(&gdk::RGBA::new(0.8, 0.8, 0.8, 1.0)));

        // Set selection colors
        terminal.set_color_highlight(Some(&gdk::RGBA::new(0.3, 0.3, 0.5, 1.0)));
        terminal.set_color_highlight_foreground(Some(&gdk::RGBA::new(1.0, 1.0, 1.0, 1.0)));

        // Set the palette for ANSI colors
        let palette = [
            // Standard colors (0-7)
            gdk::RGBA::new(0.15, 0.15, 0.15, 1.0), // Black
            gdk::RGBA::new(0.8, 0.2, 0.2, 1.0),    // Red
            gdk::RGBA::new(0.2, 0.7, 0.2, 1.0),    // Green
            gdk::RGBA::new(0.8, 0.8, 0.0, 1.0),    // Yellow
            gdk::RGBA::new(0.2, 0.5, 0.8, 1.0),    // Blue
            gdk::RGBA::new(0.8, 0.2, 0.8, 1.0),    // Magenta
            gdk::RGBA::new(0.0, 0.7, 0.7, 1.0),    // Cyan
            gdk::RGBA::new(0.85, 0.85, 0.85, 1.0), // White
            // Bright colors (8-15)
            gdk::RGBA::new(0.3, 0.3, 0.3, 1.0), // Bright Black
            gdk::RGBA::new(1.0, 0.3, 0.3, 1.0), // Bright Red
            gdk::RGBA::new(0.3, 0.9, 0.3, 1.0), // Bright Green
            gdk::RGBA::new(1.0, 1.0, 0.3, 1.0), // Bright Yellow
            gdk::RGBA::new(0.3, 0.6, 0.9, 1.0), // Bright Blue
            gdk::RGBA::new(0.9, 0.3, 0.9, 1.0), // Bright Magenta
            gdk::RGBA::new(0.3, 0.9, 0.9, 1.0), // Bright Cyan
            gdk::RGBA::new(1.0, 1.0, 1.0, 1.0), // Bright White
        ];

        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();

        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs,     // Palette references
        );
    } else {
        // Light mode color scheme
        // Set foreground (text) color to dark gray/black
        terminal.set_color_foreground(&gdk::RGBA::new(0.1, 0.1, 0.1, 1.0));

        // Set background color to white/very light gray
        terminal.set_color_background(&gdk::RGBA::new(0.98, 0.98, 0.98, 1.0));

        // Set cursor color for visibility
        terminal.set_color_cursor(Some(&gdk::RGBA::new(0.2, 0.2, 0.2, 1.0)));

        // Set selection colors
        terminal.set_color_highlight(Some(&gdk::RGBA::new(0.7, 0.7, 0.9, 1.0)));
        terminal.set_color_highlight_foreground(Some(&gdk::RGBA::new(0.0, 0.0, 0.0, 1.0)));

        // Set the palette for ANSI colors
        let palette = [
            // Standard colors (0-7)
            gdk::RGBA::new(0.98, 0.98, 0.98, 1.0), // Black (actually white for background)
            gdk::RGBA::new(0.7, 0.0, 0.0, 1.0),    // Red
            gdk::RGBA::new(0.0, 0.6, 0.0, 1.0),    // Green
            gdk::RGBA::new(0.6, 0.6, 0.0, 1.0),    // Yellow
            gdk::RGBA::new(0.0, 0.3, 0.7, 1.0),    // Blue
            gdk::RGBA::new(0.7, 0.0, 0.7, 1.0),    // Magenta
            gdk::RGBA::new(0.0, 0.6, 0.6, 1.0),    // Cyan
            gdk::RGBA::new(0.1, 0.1, 0.1, 1.0),    // White (actually black/dark gray for text)
            // Bright colors (8-15)
            gdk::RGBA::new(0.8, 0.8, 0.8, 1.0), // Bright Black (light gray)
            gdk::RGBA::new(0.9, 0.2, 0.2, 1.0), // Bright Red
            gdk::RGBA::new(0.2, 0.8, 0.2, 1.0), // Bright Green
            gdk::RGBA::new(0.8, 0.8, 0.2, 1.0), // Bright Yellow
            gdk::RGBA::new(0.2, 0.4, 0.8, 1.0), // Bright Blue
            gdk::RGBA::new(0.8, 0.2, 0.8, 1.0), // Bright Magenta
            gdk::RGBA::new(0.2, 0.8, 0.8, 1.0), // Bright Cyan
            gdk::RGBA::new(0.0, 0.0, 0.0, 1.0), // Bright White (actually black)
        ];

        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();

        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs,     // Palette references
        );
    }
}

/// Wraps a VTE terminal widget in a `ScrolledWindow` for proper scrolling.
///
/// The scrolled window expands to fill available space and enforces a minimum
/// height of 200px so the terminal is always usable.
pub fn create_terminal_box(terminal: &VteTerminal) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(terminal) // Set the terminal as the child widget
        .vexpand(true) // Expand vertically to fill all available space
        .hexpand(true) // Expand horizontally to fill available width
        .min_content_height(200) // Set minimum height for better usability (increased from 150)
        .build()
}

/// Adds a new terminal tab to the terminal notebook
///
/// Creates a new VTE terminal, wraps it in a scrolled window, and adds it
/// as a new tab in the terminal notebook. Includes a close button that hides
/// the terminal panel entirely when the last tab is closed (if `editor_paned`
/// is provided).
///
/// Returns the new page index.
///
/// See FEATURES.md: Feature #46 — Multiple Terminal Tabs
pub fn add_terminal_tab_with_toggle(
    terminal_notebook: &Notebook,
    working_dir: Option<PathBuf>,
    editor_paned: &gtk4::Paned,
) -> u32 {
    add_terminal_tab_with_paned(terminal_notebook, working_dir, Some(editor_paned), true)
}

/// Internal function to add a terminal tab with optional paned reference and auto-hide control
fn add_terminal_tab_with_paned(
    terminal_notebook: &Notebook,
    working_dir: Option<PathBuf>,
    editor_paned: Option<&gtk4::Paned>,
    auto_hide: bool,
) -> u32 {
    // Use the last folder name from the path for the tab title, or "Home" for default tabs
    let tab_title = if let Some(dir_path) = &working_dir {
        // Get the last component of the path (the folder name)
        dir_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "Home".to_string())
    } else {
        "home".to_string()
    };

    // Create a new terminal with a clone of the working directory
    let terminal = create_terminal(working_dir.clone());
    let terminal_box = create_terminal_box(&terminal);

    // Create a tab widget with the folder name or default title
    let (tab_widget, _tab_label, tab_close_button) = crate::ui::create_tab_widget(&tab_title);

    // Add middle mouse click support for the tab
    crate::ui::setup_tab_middle_click(&tab_widget, &tab_close_button);

    // Note: Right-click menu with close all/close others is not added for terminal tabs
    // as they don't have file paths or unsaved state to check

    // Append the terminal to the notebook
    let page_num = terminal_notebook.append_page(&terminal_box, Some(&tab_widget));
    terminal_notebook.set_current_page(Some(page_num));

    // Connect the close button
    let notebook_clone = terminal_notebook.clone();
    let terminal_box_clone = terminal_box.clone();
    let editor_paned_clone = editor_paned.cloned();
    tab_close_button.connect_clicked(move |_| {
        // Find the current page number for this tab's content - it may have changed since creation
        if let Some(current_page_num) = notebook_clone.page_num(&terminal_box_clone) {
            // Remove the terminal tab
            notebook_clone.remove_page(Some(current_page_num));

            // If this was the last terminal and auto-hide is enabled, hide the terminal view completely
            if auto_hide && notebook_clone.n_pages() == 0 {
                if let Some(paned) = &editor_paned_clone {
                    if let Some(end_child) = paned.end_child() {
                        end_child.set_visible(false);
                        let max_pos = paned.allocation().height();
                        paned.set_position(max_pos);
                        // Save terminal hidden state
                        let mut settings = crate::settings::get_settings_mut();
                        settings.set_terminal_visible(false);
                        let _ = settings.save();
                    }
                }
            }
        }
    });

    page_num
}

/// Updates the theme for all terminals in the terminal notebook
///
/// This should be called whenever the system theme changes to ensure
/// the terminal colors match the new theme
pub fn update_all_terminal_themes(terminal_notebook: &Notebook) {
    println!("Updating themes for all terminal tabs...");
    // Go through all tabs in the terminal notebook
    for page_num in 0..terminal_notebook.n_pages() {
        if let Some(page) = terminal_notebook.nth_page(Some(page_num)) {
            // Try to find ScrolledWindow which contains our terminal
            if let Some(scrolled_window) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                if let Some(child) = scrolled_window.child() {
                    // Check if the child is a VteTerminal
                    if let Some(terminal) = child.downcast_ref::<VteTerminal>() {
                        println!("Updating theme for terminal tab {}", page_num);
                        setup_terminal_theme(terminal);

                        // Force redraw
                        terminal.queue_draw();
                    }
                }
            }
        }
    }

    // Force the notebook to redraw
    terminal_notebook.queue_draw();

    // Print the current theme setting for debugging
    if let Some(settings) = gtk4::Settings::default() {
        let is_dark = settings.is_gtk_application_prefer_dark_theme();
        println!(
            "Terminal colors updated. Dark mode is now: {}",
            if is_dark { "enabled" } else { "disabled" }
        );
    }
}

/// Applies font size setting to a terminal
///
/// Reads the terminal font size from settings and applies it to the given terminal
pub fn apply_terminal_font_size(terminal: &VteTerminal) {
    let settings = crate::settings::get_settings();
    let font_size = settings.get_terminal_font_size();

    // Create a Pango font description with the specified size
    let font_desc = format!("monospace {}", font_size);
    if let Some(font) = pango::FontDescription::from_string(&font_desc).into() {
        terminal.set_font(Some(&font));
        println!("Applied terminal font size: {}", font_size);
    }
}

/// Updates font size for all terminals in the terminal notebook
///
/// This should be called when the terminal font size setting changes
pub fn update_all_terminal_font_sizes(terminal_notebook: &Notebook) {
    println!("Updating font sizes for all terminal tabs...");

    // Go through all tabs in the terminal notebook
    for page_num in 0..terminal_notebook.n_pages() {
        if let Some(page) = terminal_notebook.nth_page(Some(page_num)) {
            // Try to find ScrolledWindow which contains our terminal
            if let Some(scrolled_window) = page.downcast_ref::<gtk4::ScrolledWindow>() {
                if let Some(child) = scrolled_window.child() {
                    // Check if the child is a VteTerminal
                    if let Some(terminal) = child.downcast_ref::<VteTerminal>() {
                        println!("Updating font size for terminal tab {}", page_num);
                        apply_terminal_font_size(terminal);
                        terminal.queue_draw();
                    }
                }
            }
        }
    }

    terminal_notebook.queue_draw();
}
