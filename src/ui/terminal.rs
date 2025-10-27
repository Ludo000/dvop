// Terminal module for Dvop
// Contains terminal creation and management functions

use gtk4::prelude::*;
use gtk4::{
    Box as GtkBox, Notebook, ScrolledWindow, Button,
    Orientation, PackType
};
use gtk4::gdk;
use std::env;
use std::path::PathBuf;

// Terminal emulator support
use gtk4::gio::Cancellable;
use vte4::Terminal as VteTerminal;
use vte4::TerminalExtManual;
use vte4::TerminalExt;

/// Creates and initializes a terminal emulator
/// 
/// This function creates a VTE terminal widget and spawns the user's default shell in it
/// 
/// Parameters:
/// - working_dir: Optional working directory to start the terminal in. If None, uses the user's home directory
pub fn create_terminal(working_dir: Option<PathBuf>) -> VteTerminal {
    let terminal = VteTerminal::new();
    
    // Set terminal colors to match the editor's theme
    setup_terminal_theme(&terminal);
    
    // Get the user's default shell from environment variables
    if let Ok(shell) = env::var("SHELL") {
        // Use the provided working directory or fall back to user's home directory
        let dir = match working_dir {
            Some(dir) => dir,
            None => home::home_dir().expect("Could not find home directory")
        };
        
        if let Some(dir_str) = dir.to_str() {
            // Spawn the shell asynchronously in the terminal
            terminal.spawn_async(
                vte4::PtyFlags::DEFAULT,          // Default pseudo-terminal flags
                Some(dir_str),                    // Working directory
                &[&shell],                        // Command (user's shell)
                &[],                              // Environment variables (none added)
                glib::SpawnFlags::DEFAULT,        // Default spawn flags
                || {},                            // Setup function (none)
                -1,                               // Default timeout
                None::<&Cancellable>,             // No cancellation
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
/// This function configures the VTE terminal colors to match the editor's color scheme
/// based on whether the application is in dark mode or light mode. It sets:
/// - Foreground (text) color
/// - Background color
/// - Cursor color
/// - Selection colors
/// - A 16-color palette (standard ANSI colors and bright variants)
/// 
/// The color scheme is designed to be readable and consistent with the editor's appearance.
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
            gdk::RGBA::new(0.3, 0.3, 0.3, 1.0),    // Bright Black
            gdk::RGBA::new(1.0, 0.3, 0.3, 1.0),    // Bright Red
            gdk::RGBA::new(0.3, 0.9, 0.3, 1.0),    // Bright Green
            gdk::RGBA::new(1.0, 1.0, 0.3, 1.0),    // Bright Yellow
            gdk::RGBA::new(0.3, 0.6, 0.9, 1.0),    // Bright Blue
            gdk::RGBA::new(0.9, 0.3, 0.9, 1.0),    // Bright Magenta
            gdk::RGBA::new(0.3, 0.9, 0.9, 1.0),    // Bright Cyan
            gdk::RGBA::new(1.0, 1.0, 1.0, 1.0),    // Bright White
        ];
        
        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();
        
        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs      // Palette references
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
            gdk::RGBA::new(0.8, 0.8, 0.8, 1.0),    // Bright Black (light gray)
            gdk::RGBA::new(0.9, 0.2, 0.2, 1.0),    // Bright Red
            gdk::RGBA::new(0.2, 0.8, 0.2, 1.0),    // Bright Green
            gdk::RGBA::new(0.8, 0.8, 0.2, 1.0),    // Bright Yellow
            gdk::RGBA::new(0.2, 0.4, 0.8, 1.0),    // Bright Blue
            gdk::RGBA::new(0.8, 0.2, 0.8, 1.0),    // Bright Magenta
            gdk::RGBA::new(0.2, 0.8, 0.8, 1.0),    // Bright Cyan
            gdk::RGBA::new(0.0, 0.0, 0.0, 1.0),    // Bright White (actually black)
        ];
        
        // Create a vector of references to the RGBA values in the palette
        let palette_refs: Vec<&gdk::RGBA> = palette.iter().collect();
        
        terminal.set_colors(
            Some(&palette[7]), // Foreground
            Some(&palette[0]), // Background
            &palette_refs      // Palette references
        );
    }
}

/// Creates a scrollable container for the terminal
/// 
/// The terminal is placed in a scrolled window with appropriate sizing constraints
pub fn create_terminal_box(terminal: &VteTerminal) -> ScrolledWindow {
    ScrolledWindow::builder()
        .child(terminal)           // Set the terminal as the child widget
        .vexpand(true)             // Expand vertically to fill all available space
        .hexpand(true)             // Expand horizontally to fill available width
        .min_content_height(200)   // Set minimum height for better usability (increased from 150)
        .build()
}

/// Creates a tabbed terminal interface with Add and Close buttons
/// 
/// This function creates a notebook container with an initial terminal tab,
/// plus an "Add" button to create new terminal tabs.
/// Each terminal tab has its own close button.
pub fn create_terminal_notebook() -> (Notebook, Button) {
    // Create a notebook for terminal tabs
    let terminal_notebook = Notebook::new();
    terminal_notebook.set_scrollable(true);
    terminal_notebook.set_show_border(true);
    
    // Add some CSS classes for better tab styling
    terminal_notebook.add_css_class("dvop-notebook");
    
    // Create an "Add Terminal" button
    let add_terminal_button = Button::from_icon_name("list-add-symbolic");
    add_terminal_button.set_tooltip_text(Some("Add a new terminal tab"));
    add_terminal_button.set_margin_end(8); // Add right padding
    
    // Create the first terminal tab
    add_terminal_tab(&terminal_notebook, None);
    
    // Connect the Add Terminal button click handler
    let terminal_notebook_clone = terminal_notebook.clone();
    add_terminal_button.connect_clicked(move |_| {
        add_terminal_tab(&terminal_notebook_clone, None);
    });
    
    (terminal_notebook, add_terminal_button)
}

/// Adds a new terminal tab to the terminal notebook
/// 
/// Creates a new terminal instance, places it in a tab, and adds it to the notebook
/// 
/// Parameters:
/// - terminal_notebook: The notebook to add the terminal tab to
/// - working_dir: Optional working directory to start the terminal in
///
/// Returns the page number of the new tab
pub fn add_terminal_tab(terminal_notebook: &Notebook, working_dir: Option<PathBuf>) -> u32 {
    // Use the last folder name from the path for the tab title, or "Home" for default tabs
    let tab_title = if let Some(dir_path) = &working_dir {
        // Get the last component of the path (the folder name)
        dir_path.file_name()
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
    
    // Append the terminal to the notebook
    let page_num = terminal_notebook.append_page(&terminal_box, Some(&tab_widget));
    terminal_notebook.set_current_page(Some(page_num));
    
    // Connect the close button
    let notebook_clone = terminal_notebook.clone();
    let terminal_box_clone = terminal_box.clone();
    tab_close_button.connect_clicked(move |_| {
        // Find the current page number for this tab's content - it may have changed since creation
        if let Some(current_page_num) = notebook_clone.page_num(&terminal_box_clone) {
            // Remove the terminal tab regardless of whether it's the last one
            notebook_clone.remove_page(Some(current_page_num));
        }
    });
    
    page_num
}

/// Creates a container box for the terminal notebook with the add button
/// 
/// The terminal notebook is placed in a box and the add button is placed as an action button
/// in the notebook's tab bar area using the notebook's action widget feature
pub fn create_terminal_notebook_box(terminal_notebook: &Notebook, add_terminal_button: &Button) -> GtkBox {
    let terminal_box = GtkBox::new(Orientation::Vertical, 0);
    
    // Add the add button to the tab bar via the action widget feature
    // This places the button in the same row as the tabs
    terminal_notebook.set_action_widget(add_terminal_button, PackType::End);
    
    // Set the terminal notebook to expand vertically
    terminal_notebook.set_vexpand(true);
    
    // Pack just the notebook into the container box
    terminal_box.append(terminal_notebook);
    
    // Make the entire container expand vertically
    terminal_box.set_vexpand(true);
    
    terminal_box
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
        println!("Terminal colors updated. Dark mode is now: {}", 
            if is_dark { "enabled" } else { "disabled" });
    }
}
