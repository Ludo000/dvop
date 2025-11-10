// Diagnostics panel UI for displaying LSP diagnostics
// This module creates a terminal-like view for showing linter diagnostics

use gtk4::{prelude::*, ScrolledWindow, Box as GtkBox, Orientation, Label, Button, ListBox, ListBoxRow, Expander};
use gtk4::glib;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;


// Channel for sending messages to the diagnostics panel
static DIAGNOSTICS_SENDER: Lazy<Arc<Mutex<Option<Sender<DiagnosticMessage>>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Debug, Clone)]
enum DiagnosticMessage {
    Clear,
    FileSection {
        file_path: String,
        diagnostics: Vec<DiagnosticItem>,
    },
}

#[derive(Debug, Clone)]
struct DiagnosticItem {
    line: usize,
    column: usize,
    message: String,
    severity: crate::linter::DiagnosticSeverity,
    rule: String,
}

/// Create a diagnostics panel that looks like a terminal
pub fn create_diagnostics_panel() -> GtkBox {
    let outer_container = GtkBox::new(Orientation::Vertical, 0);
    
    // Create a ListBox for clickable diagnostic items
    let list_box = ListBox::new();
    list_box.set_selection_mode(gtk4::SelectionMode::Single);
    list_box.add_css_class("monospace");
    
    // Setup channel communication
    let (tx, rx) = channel::<DiagnosticMessage>();
    
    // Store the sender globally
    if let Ok(mut guard) = DIAGNOSTICS_SENDER.lock() {
        *guard = Some(tx);
    }
    
    // Receive messages on the main thread and update the ListBox
    let list_box_for_rx = list_box.clone();
    glib::idle_add_local(move || {
        // Process all pending messages
        while let Ok(msg) = rx.try_recv() {
            match msg {
                DiagnosticMessage::Clear => {
                    // Remove all children
                    while let Some(child) = list_box_for_rx.first_child() {
                        list_box_for_rx.remove(&child);
                    }
                }
                DiagnosticMessage::FileSection { file_path, diagnostics } => {
                    // Create an Expander for this file (collapsible section)
                    let expander = Expander::new(None::<&str>);
                    expander.set_expanded(false); // Collapsed by default
                    
                    // Count diagnostics by severity
                    let errors = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Error)).count();
                    let warnings = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Warning)).count();
                    let infos = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Info)).count();
                    
                    // Build title with file name and counts
                    let file_name = std::path::Path::new(&file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&file_path);
                    
                    let mut title = format!("📄 {}", file_name);
                    let mut counts = Vec::new();
                    if errors > 0 {
                        counts.push(format!("{} ❌", errors));
                    }
                    if warnings > 0 {
                        counts.push(format!("{} ⚠️", warnings));
                    }
                    if infos > 0 {
                        counts.push(format!("{} ℹ️", infos));
                    }
                    if !counts.is_empty() {
                        title.push_str(&format!("  ({})", counts.join(", ")));
                    }
                    
                    expander.set_label(Some(&title));
                    
                    // Create a ListBox for diagnostics in this file
                    let file_list_box = ListBox::new();
                    file_list_box.set_selection_mode(gtk4::SelectionMode::Single);
                    
                    // Add each diagnostic as a clickable row
                    for diag in diagnostics {
                        let item_box = GtkBox::new(Orientation::Horizontal, 8);
                        item_box.set_margin_start(8);
                        item_box.set_margin_end(8);
                        item_box.set_margin_top(4);
                        item_box.set_margin_bottom(4);
                        
                        // Severity icon
                        let icon = match diag.severity {
                            crate::linter::DiagnosticSeverity::Error => "❌",
                            crate::linter::DiagnosticSeverity::Warning => "⚠️",
                            crate::linter::DiagnosticSeverity::Info => "ℹ️",
                        };
                        let icon_label = Label::new(Some(icon));
                        item_box.append(&icon_label);
                        
                        // Location (dimmed)
                        let location_text = format!("Line {}:{}", diag.line, diag.column);
                        let location_label = Label::new(Some(&location_text));
                        location_label.add_css_class("dim");
                        item_box.append(&location_label);
                        
                        // Separator
                        let separator = Label::new(Some("-"));
                        item_box.append(&separator);
                        
                        // Message
                        let message_label = Label::new(Some(&diag.message));
                        message_label.set_hexpand(true);
                        message_label.set_halign(gtk4::Align::Start);
                        message_label.set_wrap(true);
                        message_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                        message_label.set_xalign(0.0);
                        item_box.append(&message_label);
                        
                        // Rule (dimmed)
                        if !diag.rule.is_empty() {
                            let rule_label = Label::new(Some(&format!("[{}]", diag.rule)));
                            rule_label.add_css_class("dim");
                            item_box.append(&rule_label);
                        }
                        
                        let row = ListBoxRow::new();
                        row.set_child(Some(&item_box));
                        row.set_activatable(true);
                        
                        // Make it clickable with a gesture
                        let file_path_clone = file_path.clone();
                        let line_num = diag.line;
                        let col_num = diag.column;
                        
                        let gesture = gtk4::GestureClick::new();
                        gesture.set_button(1); // Left click only
                        gesture.set_propagation_phase(gtk4::PropagationPhase::Capture);
                        
                        let file_path_for_gesture = file_path_clone.clone();
                        gesture.connect_pressed(move |gesture, _, _, _| {
                            // Stop propagation to prevent expander from toggling
                            gesture.set_state(gtk4::EventSequenceState::Claimed);
                            
                            println!("Diagnostic clicked: {} at line {}, column {}", file_path_for_gesture, line_num, col_num);
                            
                            // Convert URI to file path
                            let path = if file_path_for_gesture.starts_with("file://") {
                                // Parse as URI and extract path
                                if let Ok(url) = url::Url::parse(&file_path_for_gesture) {
                                    if let Ok(path) = url.to_file_path() {
                                        path
                                    } else {
                                        println!("Failed to convert URI to file path: {}", file_path_for_gesture);
                                        return;
                                    }
                                } else {
                                    println!("Failed to parse URI: {}", file_path_for_gesture);
                                    return;
                                }
                            } else {
                                // Already a file path
                                std::path::PathBuf::from(&file_path_for_gesture)
                            };
                            
                            println!("Opening file: {:?} at line {}, column {}", path, line_num, col_num);
                            crate::handlers::open_file_and_jump_to_location(path, line_num, col_num);
                        });
                        row.add_controller(gesture);
                        
                        // Also keep the activate signal for keyboard navigation
                        row.connect_activate(move |_| {
                            println!("Diagnostic clicked: {} at line {}, column {}", file_path_clone, line_num, col_num);
                            
                            // Convert URI to file path
                            let path = if file_path_clone.starts_with("file://") {
                                // Parse as URI and extract path
                                if let Ok(url) = url::Url::parse(&file_path_clone) {
                                    if let Ok(path) = url.to_file_path() {
                                        path
                                    } else {
                                        println!("Failed to convert URI to file path: {}", file_path_clone);
                                        return;
                                    }
                                } else {
                                    println!("Failed to parse URI: {}", file_path_clone);
                                    return;
                                }
                            } else {
                                // Already a file path
                                std::path::PathBuf::from(&file_path_clone)
                            };
                            
                            println!("Opening file: {:?} at line {}, column {}", path, line_num, col_num);
                            crate::handlers::open_file_and_jump_to_location(path, line_num, col_num);
                        });
                        
                        file_list_box.append(&row);
                    }
                    
                    expander.set_child(Some(&file_list_box));
                    
                    // Wrap expander in a ListBoxRow
                    let expander_row = ListBoxRow::new();
                    expander_row.set_activatable(false);
                    expander_row.set_selectable(false);
                    expander_row.set_child(Some(&expander));
                    
                    list_box_for_rx.append(&expander_row);
                }
            }
        }
        glib::ControlFlow::Continue
    });
    
    // Create scrolled window
    let scrolled = ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hexpand(true);
    scrolled.set_child(Some(&list_box));
    
    // Add header with clear button
    let header = GtkBox::new(Orientation::Horizontal, 4);
    header.set_margin_start(4);
    header.set_margin_end(4);
    header.set_margin_top(4);
    header.set_margin_bottom(4);
    
    let title = Label::new(Some("Diagnostics"));
    title.set_halign(gtk4::Align::Start);
    title.set_hexpand(true);
    header.append(&title);
    
    let clear_button = Button::with_label("Clear");
    let list_box_for_clear = list_box.clone();
    clear_button.connect_clicked(move |_| {
        while let Some(child) = list_box_for_clear.first_child() {
            list_box_for_clear.remove(&child);
        }
    });
    header.append(&clear_button);
    
    outer_container.append(&header);
    outer_container.append(&scrolled);
    
    outer_container
}

/// Clear all diagnostics from the panel
pub fn clear_diagnostics() {
    if let Ok(guard) = DIAGNOSTICS_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let _ = sender.send(DiagnosticMessage::Clear);
        }
    }
}

/// Format and display diagnostics for a file
pub fn display_file_diagnostics(file_uri: &str, diagnostics: &[crate::linter::Diagnostic]) {
    println!("display_file_diagnostics called with {} diagnostics for {}", diagnostics.len(), file_uri);
    
    // Don't show anything if there are no diagnostics
    if diagnostics.is_empty() {
        return;
    }
    
    // Extract file path from URI (remove file:// prefix)
    let file_path = file_uri.strip_prefix("file://").unwrap_or(file_uri);
    
    // Convert diagnostics to DiagnosticItem format
    let diagnostic_items: Vec<DiagnosticItem> = diagnostics.iter().map(|d| DiagnosticItem {
        line: d.line,
        column: d.column,
        message: d.message.clone(),
        severity: d.severity.clone(),
        rule: d.rule.clone(),
    }).collect();
    
    // Send as a file section
    if let Ok(guard) = DIAGNOSTICS_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let msg = DiagnosticMessage::FileSection {
                file_path: file_path.to_string(),
                diagnostics: diagnostic_items,
            };
            let _ = sender.send(msg);
        }
    }
}
