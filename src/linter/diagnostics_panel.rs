// Diagnostics panel UI for displaying LSP diagnostics
// This module creates a terminal-like view for showing linter diagnostics

use gtk4::{prelude::*, ScrolledWindow, Box as GtkBox, Orientation, Label, ListBox, ListBoxRow, Expander, Image};
use gtk4::glib;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};
use once_cell::sync::Lazy;
use std::collections::HashMap;


// Channel for sending messages to the diagnostics panel
static DIAGNOSTICS_SENDER: Lazy<Arc<Mutex<Option<Sender<DiagnosticMessage>>>>> =
    Lazy::new(|| Arc::new(Mutex::new(None)));

// Track expansion state per file so refreshes don't collapse everything
static EXPANSION_STATE: Lazy<Arc<Mutex<HashMap<String, bool>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[derive(Debug, Clone)]
enum DiagnosticMessage {
    Clear,
    FileSection {
        file_path: String,
        diagnostics: Vec<DiagnosticItem>,
    },
    UpdateSummary {
        total_errors: usize,
        total_warnings: usize,
        total_infos: usize,
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
    
    // Create summary box with icons
    let summary_box = GtkBox::new(Orientation::Horizontal, 8);
    summary_box.set_halign(gtk4::Align::Start);
    
    // Create summary label early so we can capture it in the closure
    let summary_label = Label::new(Some("No diagnostics"));
    summary_label.set_halign(gtk4::Align::Start);
    summary_label.set_hexpand(true);
    summary_box.append(&summary_label);
    
    // Setup channel communication
    let (tx, rx) = channel::<DiagnosticMessage>();
    
    // Store the sender globally
    if let Ok(mut guard) = DIAGNOSTICS_SENDER.lock() {
        *guard = Some(tx);
    }
    
    // Receive messages on the main thread and update the ListBox
    let list_box_for_rx = list_box.clone();
    let summary_box_for_rx = summary_box.clone();
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
                DiagnosticMessage::UpdateSummary { total_errors, total_warnings, total_infos } => {
                    // Clear the summary box
                    while let Some(child) = summary_box_for_rx.first_child() {
                        summary_box_for_rx.remove(&child);
                    }
                    
                    let total = total_errors + total_warnings + total_infos;
                    
                    if total == 0 {
                        let label = Label::new(Some("No diagnostics"));
                        summary_box_for_rx.append(&label);
                    } else {
                        // Add error count with icon
                        if total_errors > 0 {
                            let error_icon = Image::from_icon_name("dialog-error-symbolic");
                            error_icon.set_pixel_size(16);
                            summary_box_for_rx.append(&error_icon);
                            
                            let error_label = Label::new(Some(&format!("{} error{}", total_errors, if total_errors == 1 { "" } else { "s" })));
                            summary_box_for_rx.append(&error_label);
                        }
                        
                        // Add warning count with icon
                        if total_warnings > 0 {
                            let warning_icon = Image::from_icon_name("dialog-warning-symbolic");
                            warning_icon.set_pixel_size(16);
                            summary_box_for_rx.append(&warning_icon);
                            
                            let warning_label = Label::new(Some(&format!("{} warning{}", total_warnings, if total_warnings == 1 { "" } else { "s" })));
                            summary_box_for_rx.append(&warning_label);
                        }
                        
                        // Add info count with icon
                        if total_infos > 0 {
                            let info_icon = Image::from_icon_name("dialog-information-symbolic");
                            info_icon.set_pixel_size(16);
                            summary_box_for_rx.append(&info_icon);
                            
                            let info_label = Label::new(Some(&format!("{} info{}", total_infos, if total_infos == 1 { "" } else { "s" })));
                            summary_box_for_rx.append(&info_label);
                        }
                    }
                }
                DiagnosticMessage::FileSection { file_path, diagnostics } => {
                    // Create an Expander for this file (collapsible section)
                    let expander = Expander::new(None::<&str>);
                    // Restore previous expansion state if any
                    let expanded = EXPANSION_STATE
                        .lock()
                        .ok()
                        .and_then(|m| m.get(&file_path).cloned())
                        .unwrap_or(false);
                    expander.set_expanded(expanded);
                    
                    let errors = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Error)).count();
                    let warnings = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Warning)).count();
                    let infos = diagnostics.iter().filter(|d| matches!(d.severity, crate::linter::DiagnosticSeverity::Info)).count();
                    
                    let file_name = std::path::Path::new(&file_path)
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or(&file_path);
                    
                    let mut title = file_name.to_string();
                    let mut counts = Vec::new();
                    if errors > 0 {
                        counts.push(format!("{} errors", errors));
                    }
                    if warnings > 0 {
                        counts.push(format!("{} warnings", warnings));
                    }
                    if infos > 0 {
                        counts.push(format!("{} infos", infos));
                    }
                    if !counts.is_empty() {
                        title.push_str(&format!("  ({})", counts.join(", ")));
                    }
                    
                    let expander_box = GtkBox::new(Orientation::Horizontal, 8);
                    let file_icon = Image::from_icon_name("text-x-generic-symbolic");
                    file_icon.set_pixel_size(16);
                    expander_box.append(&file_icon);
                    
                    let title_label = Label::new(Some(&title));
                    expander_box.append(&title_label);
                    
                    expander.set_label_widget(Some(&expander_box));

                    // Listen for expansion changes to persist state
                    let file_path_for_notify = file_path.clone();
                    expander.connect_notify_local(Some("expanded"), move |expander, _| {
                        let is_expanded = expander.is_expanded();
                        if let Ok(mut map) = EXPANSION_STATE.lock() {
                            map.insert(file_path_for_notify.clone(), is_expanded);
                        }
                    });
                    
                    let file_list_box = ListBox::new();
                    file_list_box.set_selection_mode(gtk4::SelectionMode::Single);
                    
                    for diag in diagnostics {
                        let item_box = GtkBox::new(Orientation::Horizontal, 8);
                        
                        let icon_name = match diag.severity {
                            crate::linter::DiagnosticSeverity::Error => "dialog-error-symbolic",
                            crate::linter::DiagnosticSeverity::Warning => "dialog-warning-symbolic",
                            crate::linter::DiagnosticSeverity::Info => "dialog-information-symbolic",
                        };
                        let icon = Image::from_icon_name(icon_name);
                        icon.set_pixel_size(16);
                        item_box.append(&icon);
                        
                        // Add CSS class for background color based on severity
                        let css_class = match diag.severity {
                            crate::linter::DiagnosticSeverity::Error => "diagnostic-error",
                            crate::linter::DiagnosticSeverity::Warning => "diagnostic-warning",
                            crate::linter::DiagnosticSeverity::Info => "diagnostic-info",
                        };
                        item_box.add_css_class(css_class);
                        
                        let formatted_message = if !diag.rule.is_empty() {
                            format!("[{}:{}]: {} ({})", 
                                diag.line, 
                                diag.column, 
                                diag.message,
                                diag.rule)
                        } else {
                            format!("[{}:{}]: {}", 
                                diag.line, 
                                diag.column, 
                                diag.message)
                        };
                        
                        let message_label = Label::new(Some(&formatted_message));
                        message_label.set_hexpand(true);
                        message_label.set_halign(gtk4::Align::Start);
                        message_label.set_wrap(true);
                        message_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
                        message_label.set_xalign(0.0);
                        message_label.add_css_class("monospace");
                        item_box.append(&message_label);
                        
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
                            gesture.set_state(gtk4::EventSequenceState::Claimed);
                            
                            let path = if file_path_for_gesture.starts_with("file://") {
                                if let Ok(url) = url::Url::parse(&file_path_for_gesture) {
                                    if let Ok(path) = url.to_file_path() {
                                        path
                                    } else {
                                        return;
                                    }
                                } else {
                                    return;
                                }
                            } else {
                                std::path::PathBuf::from(&file_path_for_gesture)
                            };
                            
                            crate::handlers::open_file_and_jump_to_location(path, line_num, col_num);
                        });
                        row.add_controller(gesture);
                        
                        row.connect_activate(move |_| {
                            let path = if file_path_clone.starts_with("file://") {
                                if let Ok(url) = url::Url::parse(&file_path_clone) {
                                    if let Ok(path) = url.to_file_path() {
                                        path
                                    } else {
                                        return;
                                    }
                                } else {
                                    return;
                                }
                            } else {
                                std::path::PathBuf::from(&file_path_clone)
                            };
                            
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
    
    // Add header with summary
    let header = GtkBox::new(Orientation::Horizontal, 4);
    header.set_margin_start(4);
    header.set_margin_end(4);
    header.set_margin_top(4);
    header.set_margin_bottom(4);
    
    // Add the summary box to the header (replaces the "Diagnostics" title)
    summary_box.set_halign(gtk4::Align::Start);
    summary_box.set_hexpand(true);
    header.append(&summary_box);
    
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
    if diagnostics.is_empty() {
        return;
    }
    
    // Extract file path from URI (remove file:// prefix)
    let file_path = file_uri.strip_prefix("file://").unwrap_or(file_uri);
    
    // Store diagnostics for applying underlines when file is rendered
    crate::linter::store_file_diagnostics(file_path, diagnostics.to_vec());
    
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

/// Update the summary header with total diagnostic counts
pub fn update_summary(total_errors: usize, total_warnings: usize, total_infos: usize) {
    if let Ok(guard) = DIAGNOSTICS_SENDER.lock() {
        if let Some(sender) = guard.as_ref() {
            let msg = DiagnosticMessage::UpdateSummary {
                total_errors,
                total_warnings,
                total_infos,
            };
            let _ = sender.send(msg);
        }
    }
}
