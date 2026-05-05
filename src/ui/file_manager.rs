//! # File Manager — Clipboard & Drag-and-Drop Operations
//!
//! Provides cut/copy/paste for files in the file browser panel, plus
//! drag-and-drop reordering/moving of files between directories.
//!
//! ## File Clipboard
//!
//! The clipboard is a `thread_local!` holding an `Option<FileClipboard>` with
//! the path and operation type (copy vs cut). When a file is cut, the source
//! row in the file list gets a dimmed CSS class (`cut-file`) until the paste
//! completes or the clipboard is cleared.
//!
//! ## Drag-and-Drop
//!
//! Uses GTK4's `DragSource` / `DropTarget` API. Each file row registers as both
//! a drag source (providing its path as a string) and a drop target (accepting
//! file paths). A confirmation dialog is shown before moving files.
//!
//! ## OS Clipboard Sync
//!
//! Copy/cut operations are also synced to the OS clipboard (GNOME format:
//! `x-special/nautilus-clipboard`) so files can be pasted in Nautilus, and
//! files copied in Nautilus can be pasted in Dvop.
//!
//! See FEATURES.md: Feature #30 — File Operations (Copy, Cut, Paste)
//! See FEATURES.md: Feature #33 — File Type Filtering
//! See FEATURES.md: Feature #192 — Keyboard-Driven File Management

use gtk4::prelude::*;
use gtk4::{
    gdk,
    glib,
    // Drag and drop support
    DragSource,
    DropTarget,
    // Common UI elements
    ListBox,
};

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

/// The type of clipboard operation — determines visual styling and behavior.
///
/// `Copy` duplicates the file on paste; `Cut` moves it (deleting the original).
/// Cut files appear dimmed in the file list via the `cut-file` CSS class.
#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardOperation {
    Copy,
    Cut,
}

/// File clipboard data structure
#[derive(Clone, Debug)]
pub struct FileClipboard {
    pub file_path: PathBuf,
    pub operation: ClipboardOperation,
}

// Global file clipboard state - using thread-local storage for safety
thread_local! {
    // Option<T> is an enum that represents an optional value: either Some(T) or None.
    static FILE_CLIPBOARD: RefCell<Option<FileClipboard>> = const { RefCell::new(None) };
}

/// Copy a file to the clipboard
pub fn copy_file_to_clipboard(file_path: &PathBuf) {
    // Store in internal clipboard
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = Some(FileClipboard {
            file_path: file_path.clone(),
            operation: ClipboardOperation::Copy,
        });
    });

    // Also sync with OS clipboard
    sync_to_os_clipboard(file_path, ClipboardOperation::Copy);

    let filename = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    crate::status_log::log_info(&format!("Copied {} to clipboard", filename));
}

/// Cut a file to the clipboard
pub fn cut_file_to_clipboard(file_path: &PathBuf) {
    // Store in internal clipboard
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = Some(FileClipboard {
            file_path: file_path.clone(),
            operation: ClipboardOperation::Cut,
        });
    });

    // Also sync with OS clipboard
    sync_to_os_clipboard(file_path, ClipboardOperation::Cut);

    let filename = file_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    crate::status_log::log_info(&format!("Cut {} to clipboard", filename));
}

/// Check if a specific file is currently cut (not copied) in the clipboard
pub fn is_file_cut(file_path: &PathBuf) -> bool {
    FILE_CLIPBOARD.with(|clipboard| {
        // borrow() gets read-only access to the data inside a RefCell.
        if let Some(ref clipboard_content) = *clipboard.borrow() {
            clipboard_content.operation == ClipboardOperation::Cut
                && clipboard_content.file_path == *file_path
        } else {
            false
        }
    })
}

/// Check if there's something in the file clipboard
pub fn has_clipboard_content() -> bool {
    // First check internal clipboard (fast)
    let has_internal = FILE_CLIPBOARD.with(|clipboard| clipboard.borrow().is_some());

    if has_internal {
        return true;
    }

    // Quick check of OS clipboard without blocking
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();

        // Non-blocking check - just see if there's any text content
        // We'll do proper parsing only when actually getting the content
        return clipboard.formats().contain_mime_type("text/plain")
            || clipboard.formats().contain_mime_type("text/uri-list")
            || clipboard
                .formats()
                .contain_mime_type("x-special/gnome-copied-files");
    }

    false
}

/// Get the current clipboard content (if any)
pub fn get_clipboard_content() -> Option<FileClipboard> {
    // First try internal clipboard (fast)
    let internal_content = FILE_CLIPBOARD.with(|clipboard| clipboard.borrow().clone());

    if internal_content.is_some() {
        return internal_content;
    }

    // Only try OS clipboard if internal is empty
    try_get_os_clipboard_file()
}

/// Clear the file clipboard
pub fn clear_clipboard() {
    // Clear internal clipboard
    FILE_CLIPBOARD.with(|clipboard| {
        *clipboard.borrow_mut() = None;
    });

    // Also clear OS clipboard if it contains our file data
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();
        clipboard.set_text("");
    }

    crate::status_log::log_info("Clipboard cleared");
}

/// Paste a file from the clipboard to the target directory
pub fn paste_file_from_clipboard(
    target_dir: &PathBuf,
    window: &gtk4::ApplicationWindow,
    file_list_box: &ListBox,
    // Rc<RefCell<T>> is a common Rust pattern for single-threaded shared mutable state. Rc allows multiple owners, and RefCell allows runtime mutation.
    current_dir: &Rc<RefCell<PathBuf>>,
    // Rc<RefCell<T>> is a common Rust pattern for single-threaded shared mutable state. Rc allows multiple owners, and RefCell allows runtime mutation.
    active_tab_path: &Rc<RefCell<Option<PathBuf>>>,
) {
    let clipboard_content = get_clipboard_content();

    if let Some(clipboard) = clipboard_content {
        let source_path = clipboard.file_path;
        let filename = source_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "file".to_string());

        // Check if source file still exists
        if !source_path.exists() {
            crate::status_log::log_error(&format!("Source file {} no longer exists", filename));
            clear_clipboard();
            return;
        }

        // Create target path
        let mut target_path = target_dir.clone();
        target_path.push(&filename);

        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        match clipboard.operation {
            ClipboardOperation::Copy => {
                // For copy operations, handle name conflicts by generating unique names
                let final_target_path = if target_path.exists() {
                    generate_unique_filename(&target_path)
                } else {
                    target_path
                };

                // Copy the file
                match std::fs::copy(&source_path, &final_target_path) {
                    Ok(_) => {
                        let final_filename = final_target_path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "file".to_string());
                        crate::status_log::log_success(&format!(
                            "Copied {} to {}",
                            filename, final_filename
                        ));

                        // Refresh file list
                        crate::utils::update_file_list(
                            file_list_box,
                            // borrow() gets read-only access to the data inside a RefCell.
                            &current_dir.borrow(),
                            // borrow() gets read-only access to the data inside a RefCell.
                            &active_tab_path.borrow(),
                            crate::utils::FileSelectionSource::TabSwitch,
                        );
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!(
                            "Failed to copy {}: {}",
                            filename, e
                        ));
                        show_error_dialog(window, &format!("Failed to copy file: {}", e));
                    }
                }
            }
            ClipboardOperation::Cut => {
                // For cut operations, check if we're moving to the same location
                if source_path == target_path {
                    // Same location - nothing to do, just clear clipboard and refresh to remove cut styling
                    crate::status_log::log_info(&format!(
                        "File {} is already in the target location",
                        filename
                    ));
                    clear_clipboard();

                    // Refresh file list to remove cut styling
                    crate::utils::update_file_list(
                        file_list_box,
                        // borrow() gets read-only access to the data inside a RefCell.
                        &current_dir.borrow(),
                        &active_tab_path.borrow(),
                        crate::utils::FileSelectionSource::TabSwitch,
                    );
                    return;
                }

                // For cut operations to different locations, handle conflicts by generating unique names
                let final_target_path = if target_path.exists() && source_path != target_path {
                    generate_unique_filename(&target_path)
                } else {
                    target_path
                };

                // Move the file
                match std::fs::rename(&source_path, &final_target_path) {
                    Ok(_) => {
                        let final_filename = final_target_path
                            .file_name()
                            .map(|name| name.to_string_lossy().to_string())
                            .unwrap_or_else(|| "file".to_string());
                        crate::status_log::log_success(&format!(
                            "Moved {} to {}",
                            filename, final_filename
                        ));

                        // Update any open tabs that had this file open
                        crate::utils::trigger_tab_path_update(&source_path, &final_target_path);

                        // Clear clipboard since cut operation is consumed
                        clear_clipboard();

                        // Refresh file list
                        crate::utils::update_file_list(
                            file_list_box,
                            &current_dir.borrow(),
                            &active_tab_path.borrow(),
                            crate::utils::FileSelectionSource::TabSwitch,
                        );
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!(
                            "Failed to move {}: {}",
                            filename, e
                        ));
                        show_error_dialog(window, &format!("Failed to move file: {}", e));
                    }
                }
            }
        }
    } else {
        crate::status_log::log_error("No file in clipboard to paste");
    }
}

/// Generate a unique filename by appending a number if the file already exists
fn generate_unique_filename(original_path: &PathBuf) -> PathBuf {
    let parent_dir = original_path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let file_stem = original_path
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let extension = original_path
        .extension()
        .map(|ext| format!(".{}", ext.to_string_lossy()))
        .unwrap_or_default();

    let mut counter = 1;
    loop {
        let new_filename = format!("{} ({}){}", file_stem, counter, extension);
        let mut new_path = parent_dir.to_path_buf();
        new_path.push(&new_filename);

        if !new_path.exists() {
            return new_path;
        }

        counter += 1;

        // Prevent infinite loops
        if counter > 1000 {
            break;
        }
    }

    // Fallback with timestamp
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let fallback_filename = format!("{}-{}{}", file_stem, timestamp, extension);
    let mut fallback_path = parent_dir.to_path_buf();
    fallback_path.push(&fallback_filename);
    fallback_path
}

/// Synchronize file clipboard operation to OS clipboard
fn sync_to_os_clipboard(file_path: &PathBuf, operation: ClipboardOperation) {
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();

        if file_path.exists() {
            // Convert to absolute path and create proper file URI
            match file_path.canonicalize() {
                Ok(absolute_path) => {
                    let file_uri = format!("file://{}", absolute_path.to_string_lossy());

                    // Simplified approach: Just set the most important formats
                    // 1. URI list format (most compatible with file managers)
                    let uri_list_content = format!("{}\n", file_uri);

                    // 2. GNOME format for proper cut/copy indication
                    let gnome_content = match operation {
                        ClipboardOperation::Cut => format!("cut\n{}", file_uri),
                        ClipboardOperation::Copy => format!("copy\n{}", file_uri),
                    };

                    // Create content providers - focus on the most important ones
                    let uri_provider = gdk::ContentProvider::for_bytes(
                        "text/uri-list",
                        &glib::Bytes::from(uri_list_content.as_bytes()),
                    );

                    let gnome_provider = gdk::ContentProvider::for_bytes(
                        "x-special/gnome-copied-files",
                        &glib::Bytes::from(gnome_content.as_bytes()),
                    );

                    // Simple text fallback
                    let text_content = match operation {
                        ClipboardOperation::Copy => {
                            format!("DVOP_COPY:{}", absolute_path.to_string_lossy())
                        }
                        ClipboardOperation::Cut => {
                            format!("DVOP_CUT:{}", absolute_path.to_string_lossy())
                        }
                    };
                    let text_provider = gdk::ContentProvider::for_bytes(
                        "text/plain",
                        &glib::Bytes::from(text_content.as_bytes()),
                    );

                    // Combine providers efficiently
                    let combined_provider = gdk::ContentProvider::new_union(&[
                        uri_provider,
                        gnome_provider,
                        text_provider,
                    ]);
                    let _ = clipboard.set_content(Some(&combined_provider));
                }
                Err(_) => {
                    // If canonicalization fails, just set a simple text representation
                    let simple_text = match operation {
                        ClipboardOperation::Copy => {
                            format!("DVOP_COPY:{}", file_path.to_string_lossy())
                        }
                        ClipboardOperation::Cut => {
                            format!("DVOP_CUT:{}", file_path.to_string_lossy())
                        }
                    };
                    clipboard.set_text(&simple_text);
                }
            }
        }
    }
}

/// Try to get file clipboard content from OS clipboard
fn try_get_os_clipboard_file() -> Option<FileClipboard> {
    if let Some(display) = gdk::Display::default() {
        let clipboard = display.clipboard();

        // Quick non-blocking check of available formats
        let formats = clipboard.formats();

        // Try GNOME format first (most reliable for file operations)
        if formats.contain_mime_type("x-special/gnome-copied-files") {
            // Use a very short timeout for GNOME format
            if let Some(gnome_content) =
                get_clipboard_content_fast(&clipboard, "x-special/gnome-copied-files")
            {
                if let Some(clipboard_data) = parse_gnome_clipboard_format(&gnome_content) {
                    return Some(clipboard_data);
                }
            }
        }

        // Try URI list format with short timeout
        if formats.contain_mime_type("text/uri-list") {
            if let Some(uri_content) = get_clipboard_content_fast(&clipboard, "text/uri-list") {
                if let Some(clipboard_data) = parse_uri_list_format(&uri_content) {
                    return Some(clipboard_data);
                }
            }
        }

        // Finally try plain text with very short timeout
        if formats.contain_mime_type("text/plain") {
            if let Some(text) = get_clipboard_text_fast(&clipboard) {
                if let Some(clipboard_data) = parse_text_format(&text) {
                    return Some(clipboard_data);
                }
            }
        }
    }
    None
}

/// Parse GNOME clipboard format (x-special/gnome-copied-files)
fn parse_gnome_clipboard_format(content: &str) -> Option<FileClipboard> {
    let lines: Vec<&str> = content.trim().lines().collect();
    if lines.len() >= 2 {
        let operation_str = lines[0];
        let file_uri = lines[1];

        // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
        let operation = match operation_str {
            "copy" => ClipboardOperation::Copy,
            "cut" => ClipboardOperation::Cut,
            _ => return None,
        };

        if let Some(file_path) = uri_to_path(file_uri) {
            if file_path.exists() {
                return Some(FileClipboard {
                    file_path,
                    operation,
                });
            }
        }
    }
    None
}

/// Parse URI list format (text/uri-list)
fn parse_uri_list_format(content: &str) -> Option<FileClipboard> {
    for line in content.lines() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') {
            if let Some(file_path) = uri_to_path(line) {
                if file_path.exists() {
                    return Some(FileClipboard {
                        file_path,
                        operation: ClipboardOperation::Copy, // Default to copy for URI lists
                    });
                }
            }
        }
    }
    None
}

/// Parse plain text format (our custom format or plain paths)
fn parse_text_format(text: &str) -> Option<FileClipboard> {
    let text = text.trim();

    // Check if it's our custom format
    if text.starts_with("DVOP_COPY:") {
        if let Some(path_str) = text.strip_prefix("DVOP_COPY:") {
            let file_path = PathBuf::from(path_str);
            if file_path.exists() {
                return Some(FileClipboard {
                    file_path,
                    operation: ClipboardOperation::Copy,
                });
            }
        }
    } else if text.starts_with("DVOP_CUT:") {
        if let Some(path_str) = text.strip_prefix("DVOP_CUT:") {
            let file_path = PathBuf::from(path_str);
            if file_path.exists() {
                return Some(FileClipboard {
                    file_path,
                    operation: ClipboardOperation::Cut,
                });
            }
        }
    }
    // Handle file:// URIs
    else if let Some(file_path) = uri_to_path(text) {
        if file_path.exists() {
            return Some(FileClipboard {
                file_path,
                operation: ClipboardOperation::Copy,
            });
        }
    }
    // Handle plain file paths
    else if let Ok(file_path) = PathBuf::from(text).canonicalize() {
        if file_path.exists() && file_path.is_absolute() {
            return Some(FileClipboard {
                file_path,
                operation: ClipboardOperation::Copy,
            });
        }
    }

    None
}

/// Convert file URI to PathBuf
fn uri_to_path(uri: &str) -> Option<PathBuf> {
    if let Some(path_str) = uri.strip_prefix("file://") {
        // Simple URL decoding for basic cases (spaces, etc.)
        let decoded = path_str
            .replace("%20", " ")
            .replace("%25", "%")
            .replace("%2F", "/")
            .replace("%5C", "\\");

        let path = PathBuf::from(decoded);
        return Some(path);
    }
    None
}

/// Fast synchronous get text from clipboard with short timeout
fn get_clipboard_text_fast(clipboard: &gdk::Clipboard) -> Option<String> {
    // mpsc::channel creates a multi-producer, single-consumer queue for passing messages between threads.
    let (sender, receiver) = std::sync::mpsc::channel();

    clipboard.read_text_async(
        None::<&gtk4::gio::Cancellable>,
        // The "move" keyword forces the closure to take ownership of the variables it uses.
        move |result: Result<Option<glib::GString>, glib::Error>| {
            // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
            let text_result = match result {
                Ok(text) => text.map(|s| s.to_string()),
                Err(_) => None,
            };
            let _ = sender.send(text_result);
        },
    );

    // Much shorter timeout - 20ms instead of 200ms
    match receiver.recv_timeout(std::time::Duration::from_millis(20)) {
        Ok(result) => result,
        Err(_) => None,
    }
}

/// Fast get clipboard content for specific MIME type
fn get_clipboard_content_fast(clipboard: &gdk::Clipboard, _mime_type: &str) -> Option<String> {
    // For now, fall back to text reading since GTK4's async MIME type reading is complex
    // This is a simplification - in practice most clipboard content we care about is also available as text
    get_clipboard_text_fast(clipboard)
}

/// Show an error dialog
fn show_error_dialog(window: &gtk4::ApplicationWindow, message: &str) {
    let dialog = gtk4::MessageDialog::new(
        Some(window),
        gtk4::DialogFlags::MODAL | gtk4::DialogFlags::DESTROY_WITH_PARENT,
        gtk4::MessageType::Error,
        gtk4::ButtonsType::Ok,
        message,
    );

    // The "move" keyword forces the closure to take ownership of the variables it uses.
    dialog.connect_response(move |d, _| {
        d.close();
    });

    dialog.show();
}

/// Creates the file manager panel components with drag and drop support
///
/// Returns a tuple containing:
/// Sets up drag and drop functionality for a file list row
///
/// This function configures both drag source and drop target for a file or directory item
/// in the file manager. It enables users to drag files and folders to move them around.
pub fn setup_drag_drop_for_row(
    row: &gtk4::ListBoxRow,
    file_path: &std::path::Path,
    is_directory: bool,
) {
    let file_path_clone = file_path.to_path_buf();
    let _file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();

    // Set up drag source - what this item can be dragged as
    let drag_source = DragSource::new();
    drag_source.set_actions(gdk::DragAction::MOVE | gdk::DragAction::COPY);

    // Prepare drag data - we'll send the full file path as text
    let file_path_for_drag = file_path_clone.clone();
    // The "move" keyword forces the closure to take ownership of the variables it uses.
    drag_source.connect_prepare(move |_, _x, _y| {
        let file_path_str = file_path_for_drag.to_string_lossy().to_string();
        let content_provider = gdk::ContentProvider::for_value(&glib::Value::from(&file_path_str));
        Some(content_provider)
    });

    // Set up drag begin handler to show what's being dragged
    drag_source.connect_drag_begin(move |_source, _drag| {
        // For now, we'll skip setting a custom icon since it's complex in GTK4
        // The system will provide a default drag cursor
    });

    row.add_controller(drag_source);

    // Set up drop target - what can be dropped on this item (only for directories)
    if is_directory {
        let drop_target = DropTarget::new(
            glib::Type::STRING,
            gdk::DragAction::MOVE | gdk::DragAction::COPY,
        );

        // Visual feedback during drag over
        drop_target.connect_enter(move |target, _x, _y| {
            if let Some(widget) = target.widget() {
                widget.add_css_class("drop-target");
            }
            gdk::DragAction::MOVE
        });

        // The "move" keyword forces the closure to take ownership of the variables it uses.
        drop_target.connect_leave(move |target| {
            if let Some(widget) = target.widget() {
                widget.remove_css_class("drop-target");
            }
        });

        // Ensure drop target class is removed after drop operation
        let target_dir_for_cleanup = file_path_clone.clone();
        drop_target.connect_drop(move |target, value, _x, _y| {
            // Remove visual feedback immediately
            if let Some(widget) = target.widget() {
                widget.remove_css_class("drop-target");
                // Also try to get the parent list box and clean it up
                if let Some(parent) = widget.parent() {
                    if let Some(list_box) = parent.downcast_ref::<ListBox>() {
                        cleanup_drag_drop_styles(list_box);
                    }
                }
            }

            if let Ok(source_path_str) = value.get::<String>() {
                let source_path = std::path::PathBuf::from(&source_path_str);
                let target_path = target_dir_for_cleanup.clone();

                // Show confirmation modal for the move operation
                crate::ui::file_manager::show_move_confirmation_modal(&source_path, &target_path);
                return true;
            }
            false
        });

        row.add_controller(drop_target);
    }
}
/// Forces cleanup of all drag and drop CSS classes from the file list
///
/// This function ensures that no drag-drop visual feedback persists after operations complete
pub fn cleanup_drag_drop_styles(file_list_box: &ListBox) {
    // Iterate through all rows and remove any drag-drop related CSS classes
    let mut child = file_list_box.first_child();
    while let Some(current_child) = child {
        if let Some(row) = current_child.downcast_ref::<gtk4::ListBoxRow>() {
            row.remove_css_class("drop-target");
            row.remove_css_class("drop-target-background");
        }
        current_child.remove_css_class("drop-target");
        current_child.remove_css_class("drop-target-background");
        child = current_child.next_sibling();
    }

    // Also remove from the file list box itself
    file_list_box.remove_css_class("drop-target");
    file_list_box.remove_css_class("drop-target-background");
}

/// Shows a confirmation modal for file/folder move operations
///
/// This function displays a dialog asking the user to confirm moving a file or folder
/// from the source location to the target directory.
pub fn show_move_confirmation_modal(source_path: &std::path::Path, target_dir: &std::path::Path) {
    use gtk4::{ButtonsType, DialogFlags, MessageDialog, MessageType, ResponseType};

    let source_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown item");

    let target_name = target_dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown folder");

    // Determine if we're moving a file or directory
    let item_type = if source_path.is_dir() {
        "folder"
    } else {
        "file"
    };

    // Build the target path where the item would be moved
    let mut final_target = target_dir.to_path_buf();
    final_target.push(source_name);

    // Check if target already exists
    let conflict_message = if final_target.exists() {
        format!("\n\nWarning: A {} with this name already exists in the destination. It will be replaced.", item_type)
    } else {
        String::new()
    };

    let message = format!(
        "Move {} \"{}\" to folder \"{}\"?{}",
        item_type, source_name, target_name, conflict_message
    );

    // Find the application window to use as parent
    if let Some(app) = gtk4::gio::Application::default() {
        if let Some(gtk_app) = app.downcast_ref::<gtk4::Application>() {
            if let Some(window) = gtk_app.active_window() {
                let dialog = MessageDialog::new(
                    Some(&window),
                    DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                    MessageType::Question,
                    ButtonsType::None,
                    &message,
                );

                dialog.add_buttons(&[
                    ("Cancel", ResponseType::Cancel),
                    ("Move", ResponseType::Accept),
                ]);

                dialog.set_default_response(ResponseType::Cancel);

                let source_path = source_path.to_path_buf();
                let final_target = final_target.clone();
                let window_clone = window.clone();

                dialog.connect_response(move |d, response| {
                    if response == ResponseType::Accept {
                        perform_file_move(&source_path, &final_target, &window_clone);
                    }
                    d.close();
                });

                dialog.show();
            }
        }
    }
}

/// Performs the actual file/folder move operation
///
/// This function handles the filesystem operation of moving a file or directory
/// and shows appropriate success or error messages.
fn perform_file_move(source: &std::path::Path, target: &std::path::Path, window: &gtk4::Window) {
    use gtk4::{ButtonsType, DialogFlags, MessageDialog, MessageType};

    let source_name = source
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Unknown item");

    // Attempt to move the file/directory
    match std::fs::rename(source, target) {
        Ok(()) => {
            // Success - log it
            crate::status_log::log_success(&format!("Moved {} successfully", source_name));

            // Update any open tabs that had this file open
            crate::utils::trigger_tab_path_update(&source.to_path_buf(), &target.to_path_buf());

            // Force cleanup of any lingering drag-drop styles
            if let Some(app) = gtk4::gio::Application::default() {
                if let Some(gtk_app) = app.downcast_ref::<gtk4::Application>() {
                    if let Some(active_window) = gtk_app.active_window() {
                        cleanup_all_drag_drop_styles(active_window.upcast_ref::<gtk4::Widget>());
                    }
                }
            }

            // Trigger a file list refresh by sending a custom event
            refresh_file_list_after_move();
        }
        Err(e) => {
            // Error - show error dialog
            let error_msg = format!("Failed to move {}: {}", source_name, e);
            crate::status_log::log_error(&error_msg);

            let error_dialog = MessageDialog::new(
                Some(window),
                DialogFlags::MODAL | DialogFlags::DESTROY_WITH_PARENT,
                MessageType::Error,
                ButtonsType::Ok,
                &error_msg,
            );

            error_dialog.show();
        }
    }
}

/// Triggers a refresh of the file list after a file move operation
/// This uses the callback system to refresh the file list
fn refresh_file_list_after_move() {
    // Trigger the refresh callback with a small delay to ensure the filesystem operation is complete
    glib::timeout_add_local_once(std::time::Duration::from_millis(50), || {
        crate::utils::trigger_file_list_refresh();
    });
}

/// Recursively finds and cleans up all drag-drop styles from a widget hierarchy
///
/// This function walks through the widget tree and removes any lingering drag-drop CSS classes
fn cleanup_all_drag_drop_styles(widget: &gtk4::Widget) {
    // Remove classes from this widget
    widget.remove_css_class("drop-target");
    widget.remove_css_class("drop-target-background");

    // If this is a ListBox, use the specialized cleanup
    if let Some(list_box) = widget.downcast_ref::<ListBox>() {
        cleanup_drag_drop_styles(list_box);
    }

    // Recursively clean up all children
    let mut child = widget.first_child();
    while let Some(current_child) = child {
        cleanup_all_drag_drop_styles(&current_child);
        child = current_child.next_sibling();
    }
}
