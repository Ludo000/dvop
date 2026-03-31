use super::runner;
use gtk4::prelude::*;
use std::path::Path;

// ── Lifecycle hooks ──────────────────────────────────────────────

/// Fire all enabled extensions' on_app_start hooks (fire-and-forget).
pub fn fire_on_app_start() {
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if !ext.manifest.enabled {
            continue;
        }
        if let Some(ref hooks) = ext.manifest.contributions.hooks {
            if let Some(ref script) = hooks.on_app_start {
                let script_path = ext.path.join(script);
                runner::run_script_fire_and_forget(&script_path, &[]);
            }
        }
    }
}

/// Fire all enabled extensions' on_file_open hooks (fire-and-forget).
pub fn fire_on_file_open(file_path: &Path) {
    let path_str = file_path.to_string_lossy().to_string();
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if !ext.manifest.enabled {
            continue;
        }
        if let Some(ref hooks) = ext.manifest.contributions.hooks {
            if let Some(ref script) = hooks.on_file_open {
                let script_path = ext.path.join(script);
                runner::run_script_fire_and_forget(&script_path, &[&path_str]);
            }
        }
    }
}

/// Fire all enabled extensions' on_file_save hooks (fire-and-forget).
pub fn fire_on_file_save(file_path: &Path) {
    let path_str = file_path.to_string_lossy().to_string();
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if !ext.manifest.enabled {
            continue;
        }
        if let Some(ref hooks) = ext.manifest.contributions.hooks {
            if let Some(ref script) = hooks.on_file_save {
                let script_path = ext.path.join(script);
                runner::run_script_fire_and_forget(&script_path, &[&path_str]);
            }
        }
    }
}

/// Fire all enabled extensions' on_file_close hooks (fire-and-forget).
pub fn fire_on_file_close(file_path: &Path) {
    let path_str = file_path.to_string_lossy().to_string();
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if !ext.manifest.enabled {
            continue;
        }
        if let Some(ref hooks) = ext.manifest.contributions.hooks {
            if let Some(ref script) = hooks.on_file_close {
                let script_path = ext.path.join(script);
                runner::run_script_fire_and_forget(&script_path, &[&path_str]);
            }
        }
    }
}

// ── Live refresh on enable/disable ───────────────────────────────

/// Refresh all runtime contributions for a single extension after enable/disable toggle.
/// This only updates what the extension actually contributes.
pub fn refresh_extension(ext_id: &str, enabled: bool) {
    let mgr = super::manager::get_manager();
    // Handle native extensions separately
    if super::native::is_native_extension(ext_id) {
        super::native::set_native_enabled(ext_id, enabled);
        return;
    }

    let ext = match mgr.get_extensions().iter().find(|e| e.manifest.id == ext_id) {
        Some(e) => e.clone(),
        None => return,
    };
    drop(mgr);
    let contribs = &ext.manifest.contributions;

    // CSS: re-apply all CSS if this extension has a CSS contribution
    if contribs.css.is_some() {
        crate::ui::css::apply_custom_css();
    }

    // Status bar: force refresh the cached text and re-render the label
    if contribs.status_bar.is_some() {
        ACTIVE_FILE_PATH.with(|fp| {
            if let Some(ref path) = *fp.borrow() {
                super::manager::update_status_bar_text(path);
            }
        });
        force_status_label_refresh();
    }

    // Keybindings: toggle action enabled state
    if !contribs.keybindings.is_empty() {
        ACTIVE_NOTEBOOK.with(|nb_cell| {
            let nb_opt = nb_cell.borrow();
            if let Some(ref notebook) = *nb_opt {
                if let Some(window) = notebook.root().and_then(|r| r.downcast::<gtk4::ApplicationWindow>().ok()) {
                    refresh_extension_keybindings(&window, ext_id, enabled);
                }
            }
        });
    }

    // Editor context menus: toggle action enabled state + rebuild menus on open tabs
    if contribs.context_menus.as_ref().map_or(false, |c| !c.editor.is_empty()) {
        ACTIVE_NOTEBOOK.with(|nb_cell| {
            let nb_opt = nb_cell.borrow();
            if let Some(ref notebook) = *nb_opt {
                if let Some(window) = notebook.root().and_then(|r| r.downcast::<gtk4::ApplicationWindow>().ok()) {
                    // Toggle action enabled states
                    if let Some(ref ctx) = contribs.context_menus {
                        for entry in &ctx.editor {
                            let action_name = format!(
                                "ext-editor-ctx-{}",
                                entry.label.to_lowercase().replace(' ', "-")
                            );
                            if let Some(action) = window.lookup_action(&action_name) {
                                if let Some(simple) = action.downcast_ref::<gtk4::gio::SimpleAction>() {
                                    simple.set_enabled(enabled);
                                }
                            }
                        }
                    }
                    // Rebuild the extra_menu on all open source views
                    let n_pages = notebook.n_pages();
                    for i in 0..n_pages {
                        if let Some((text_view, _)) = crate::handlers::get_text_view_and_buffer_for_page(notebook, i) {
                            if let Some(source_view) = text_view.downcast_ref::<sourceview5::View>() {
                                crate::handlers::setup_extension_editor_context_menu(source_view, &window);
                            }
                        }
                    }
                }
            }
        });
    }

    // Sidebar panels: toggle visibility of the activity bar button
    if !contribs.sidebar_panels.is_empty() {
        ACTIVE_NOTEBOOK.with(|nb_cell| {
            let nb_opt = nb_cell.borrow();
            if let Some(ref notebook) = *nb_opt {
                if let Some(root) = notebook.root() {
                    if let Some(window) = root.downcast_ref::<crate::ui::DvopWindow>() {
                        let activity_bar = window.imp().activity_bar.get();
                        // Iterate activity bar children to find extension panel buttons
                        let mut child = activity_bar.first_child();
                        while let Some(widget) = child {
                            if let Some(btn) = widget.downcast_ref::<gtk4::ToggleButton>() {
                                for panel in &contribs.sidebar_panels {
                                    if btn.tooltip_text().as_deref() == Some(&panel.title) {
                                        btn.set_visible(enabled);
                                        if !enabled && btn.is_active() {
                                            btn.set_active(false);
                                        }
                                    }
                                }
                            }
                            child = widget.next_sibling();
                        }
                    }
                }
            }
        });
    }
}

// ── Extension linters ────────────────────────────────────────────

/// Run all extension linters that match the given file extension.
/// Returns a vec of diagnostics from all matching linter scripts.
pub fn run_extension_linters(file_path: &Path) -> Vec<crate::linter::Diagnostic> {
    let file_ext = file_path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut all_diagnostics = Vec::new();
    let path_str = file_path.to_string_lossy().to_string();

    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if !ext.manifest.enabled || ext.manifest.is_native {
            continue;
        }
        for linter in &ext.manifest.contributions.linters {
            let matches = linter
                .languages
                .iter()
                .any(|lang| lang.to_lowercase() == file_ext);
            if !matches {
                continue;
            }

            let script_path = ext.path.join(&linter.script);
            match runner::run_script_json::<Vec<ExtLintDiagnostic>>(&script_path, &[&path_str]) {
                Ok(diags) => {
                    for d in diags {
                        all_diagnostics.push(crate::linter::Diagnostic {
                            severity: match d.severity.as_str() {
                                "error" => crate::linter::DiagnosticSeverity::Error,
                                "warning" => crate::linter::DiagnosticSeverity::Warning,
                                _ => crate::linter::DiagnosticSeverity::Info,
                            },
                            message: d.message,
                            line: d.line,
                            column: d.column,
                            end_line: d.end_line,
                            end_column: d.end_column,
                            rule: d.rule.unwrap_or_default(),
                        });
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Extension linter '{}' failed for {:?}: {}",
                        ext.manifest.name,
                        file_path.file_name().unwrap_or_default(),
                        e
                    );
                }
            }
        }
    }
    all_diagnostics
}

/// JSON format expected from extension linter scripts
#[derive(serde::Deserialize)]
struct ExtLintDiagnostic {
    severity: String,
    message: String,
    line: usize,
    column: usize,
    #[serde(default)]
    end_line: Option<usize>,
    #[serde(default)]
    end_column: Option<usize>,
    #[serde(default)]
    rule: Option<String>,
}

// ── Keybinding registration ─────────────────────────────────────

/// Register extension keybindings on the GTK window.
/// Registers ALL extensions (enabled and disabled) and controls activation via set_enabled().
pub fn register_extension_keybindings(window: &gtk4::ApplicationWindow, app: &gtk4::Application) {
    let mgr = super::manager::get_manager();
    let mut accels: Vec<(String, String)> = Vec::new();

    for ext in mgr.get_extensions() {
        for kb in &ext.manifest.contributions.keybindings {
            let action_name = format!("ext-{}-{}", ext.manifest.id, sanitize_action_name(&kb.title));
            let script_path = ext.path.join(&kb.script);

            let action = gtk4::gio::SimpleAction::new(&action_name, None);
            action.set_enabled(ext.manifest.enabled);
            let sp = script_path.clone();
            action.connect_activate(move |_, _| {
                run_extension_command_on_active_editor(&sp);
            });
            window.add_action(&action);

            // Convert key string like "Ctrl+Shift+L" to GTK accel "<Control><Shift>l"
            let gtk_accel = key_string_to_gtk_accel(&kb.key);
            let full_action = format!("win.{}", action_name);
            accels.push((full_action, gtk_accel));
        }
    }
    drop(mgr);

    for (action, accel) in &accels {
        app.set_accels_for_action(action, &[accel]);
    }
}

/// Update enabled state of keybinding actions for a specific extension.
pub fn refresh_extension_keybindings(window: &gtk4::ApplicationWindow, ext_id: &str, enabled: bool) {
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if ext.manifest.id != ext_id {
            continue;
        }
        for kb in &ext.manifest.contributions.keybindings {
            let action_name = format!("ext-{}-{}", ext.manifest.id, sanitize_action_name(&kb.title));
            if let Some(action) = window.lookup_action(&action_name) {
                if let Some(simple) = action.downcast_ref::<gtk4::gio::SimpleAction>() {
                    simple.set_enabled(enabled);
                }
            }
        }
        break;
    }
}

/// Convert a user-friendly key string (e.g. "Ctrl+Shift+L") to GTK accel format ("<Control><Shift>l")
fn key_string_to_gtk_accel(key: &str) -> String {
    let parts: Vec<&str> = key.split('+').collect();
    let mut accel = String::new();
    for part in &parts {
        let trimmed = part.trim();
        match trimmed.to_lowercase().as_str() {
            "ctrl" | "control" => accel.push_str("<Control>"),
            "shift" => accel.push_str("<Shift>"),
            "alt" => accel.push_str("<Alt>"),
            "super" | "meta" => accel.push_str("<Super>"),
            _ => accel.push_str(trimmed),
        }
    }
    accel
}

/// Sanitize a string for use as a GAction name (lowercase, alphanumeric + hyphens)
fn sanitize_action_name(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect()
}

// ── Command/transform execution on active editor ─────────────────

/// Run an extension command script on the active editor's selection.
/// If the script produces output, replace the selection with it.
pub fn run_extension_command_on_active_editor(script_path: &std::path::Path) {
    // Get the active editor buffer info
    let (file_path, selection) = get_active_editor_info();
    let file_str = file_path.unwrap_or_default();
    let sel_str = selection.unwrap_or_default();

    match runner::run_script(script_path, &[&file_str, &sel_str], None) {
        Ok(output) => {
            if !output.is_empty() {
                replace_active_editor_selection(&output);
            }
        }
        Err(e) => {
            crate::status_log::log_error(&format!("Extension script failed: {}", e));
        }
    }
}

/// Run a text transform script: passes selection on stdin, replaces with stdout.
pub fn run_text_transform_on_active_editor(script_path: &std::path::Path) {
    let (file_path, selection) = get_active_editor_info();
    let file_str = file_path.unwrap_or_default();
    let sel_str = selection.unwrap_or_default();

    match runner::run_script(script_path, &[&file_str], Some(&sel_str)) {
        Ok(output) => {
            if !output.is_empty() {
                replace_active_editor_selection(&output);
            }
        }
        Err(e) => {
            crate::status_log::log_error(&format!("Transform failed: {}", e));
        }
    }
}

/// Run an editor context menu script with full context.
pub fn run_editor_context_menu_script(
    script_path: &std::path::Path,
    file_path: &str,
    selection: &str,
    line: usize,
    col: usize,
) {
    let line_str = line.to_string();
    let col_str = col.to_string();

    match runner::run_script(script_path, &[file_path, selection, &line_str, &col_str], None) {
        Ok(output) => {
            if !output.is_empty() {
                replace_active_editor_selection(&output);
            }
        }
        Err(e) => {
            crate::status_log::log_error(&format!("Context menu script failed: {}", e));
        }
    }
}

/// Run a file explorer context menu script (side-effects only).
pub fn run_file_explorer_context_menu_script(script_path: &std::path::Path, file_path: &str) {
    runner::run_script_fire_and_forget(script_path, &[file_path]);
}

// ── Helpers to interact with the active editor ──────────────────

/// Get the file path and selection text of the active editor.
fn get_active_editor_info() -> (Option<String>, Option<String>) {
    ACTIVE_NOTEBOOK.with(|nb_cell| {
        let nb_opt = nb_cell.borrow();
        if let Some(ref notebook) = *nb_opt {
            if let Some(page_num) = notebook.current_page() {
                if let Some((text_view, _)) =
                    crate::handlers::get_text_view_and_buffer_for_page(notebook, page_num)
                {
                    let buffer = text_view.buffer();
                    let selection = if buffer.has_selection() {
                        let (start, end) = buffer.selection_bounds().unwrap_or_else(|| {
                            let iter = buffer.iter_at_offset(0);
                            (iter.clone(), iter)
                        });
                        Some(buffer.text(&start, &end, false).to_string())
                    } else {
                        None
                    };

                    // Try to get file path from the ACTIVE_FILE_PATH thread local
                    let file_path = ACTIVE_FILE_PATH.with(|fp| {
                        fp.borrow().as_ref().map(|p| p.to_string_lossy().to_string())
                    });

                    return (file_path, selection);
                }
            }
        }
        (None, None)
    })
}

/// Replace the current selection in the active editor, or insert at cursor if no selection.
fn replace_active_editor_selection(text: &str) {
    ACTIVE_NOTEBOOK.with(|nb_cell| {
        let nb_opt = nb_cell.borrow();
        if let Some(ref notebook) = *nb_opt {
            if let Some(page_num) = notebook.current_page() {
                if let Some((text_view, _)) =
                    crate::handlers::get_text_view_and_buffer_for_page(notebook, page_num)
                {
                    let buffer = text_view.buffer();
                    if buffer.has_selection() {
                        buffer.delete_selection(true, true);
                    }
                    buffer.insert_at_cursor(text);
                }
            }
        }
    });
}

// Thread-local references set during app init for hook access
thread_local! {
    pub static ACTIVE_NOTEBOOK: std::cell::RefCell<Option<gtk4::Notebook>> =
        const { std::cell::RefCell::new(None) };
    pub static ACTIVE_FILE_PATH: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
    pub static STATUS_LABEL: std::cell::RefCell<Option<gtk4::Label>> =
        const { std::cell::RefCell::new(None) };
    /// Stores (ext_id, panel_id, TextView) for each sidebar panel so we can refresh them.
    static SIDEBAR_PANEL_VIEWS: std::cell::RefCell<Vec<(String, String, gtk4::TextView)>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

/// Set the notebook reference for hooks to access. Call once during init.
pub fn set_active_notebook(notebook: &gtk4::Notebook) {
    ACTIVE_NOTEBOOK.with(|nb| {
        *nb.borrow_mut() = Some(notebook.clone());
    });
}

/// Update the active file path (call on tab switch).
pub fn set_active_file_path(path: Option<std::path::PathBuf>) {
    ACTIVE_FILE_PATH.with(|fp| {
        *fp.borrow_mut() = path;
    });
}

/// Set the status label reference for live refresh. Call once during init.
pub fn set_status_label(label: &gtk4::Label) {
    STATUS_LABEL.with(|sl| {
        *sl.borrow_mut() = Some(label.clone());
    });
}

/// Register a sidebar panel's TextView for later refresh. Call during init for each panel.
pub fn register_sidebar_panel_view(ext_id: &str, panel_id: &str, text_view: &gtk4::TextView) {
    SIDEBAR_PANEL_VIEWS.with(|views| {
        views.borrow_mut().push((
            ext_id.to_string(),
            panel_id.to_string(),
            text_view.clone(),
        ));
    });
}

/// Refresh all registered sidebar panels with current file. Call on tab switch.
pub fn refresh_sidebar_panels() {
    SIDEBAR_PANEL_VIEWS.with(|views| {
        let views = views.borrow();
        for (ext_id, panel_id, text_view) in views.iter() {
            let content = get_sidebar_panel_content(ext_id, panel_id, "refresh");
            text_view.buffer().set_text(&content);
        }
    });
}

/// Force the status label to re-render with current cached data.
fn force_status_label_refresh() {
    STATUS_LABEL.with(|sl_cell| {
        let sl_opt = sl_cell.borrow();
        let Some(ref status_label) = *sl_opt else { return };

        ACTIVE_NOTEBOOK.with(|nb_cell| {
            let nb_opt = nb_cell.borrow();
            let Some(ref notebook) = *nb_opt else { return };
            let Some(page_num) = notebook.current_page() else { return };
            let Some((text_view, _)) =
                crate::handlers::get_text_view_and_buffer_for_page(notebook, page_num)
            else { return };
            let Some(source_view) = text_view.downcast_ref::<sourceview5::View>() else { return };

            let buffer = source_view.buffer();
            let cursor_mark = buffer.get_insert();
            let cursor_iter = buffer.iter_at_mark(&cursor_mark);
            let line = cursor_iter.line() + 1;
            let column = cursor_iter.line_offset() + 1;

            let (filename, size_part) = ACTIVE_FILE_PATH.with(|fp| {
                match *fp.borrow() {
                    Some(ref path) => {
                        let name = path.file_name()
                            .map(|n| n.to_string_lossy().into_owned())
                            .unwrap_or_else(|| "Unknown".to_string());
                        let size = std::fs::metadata(path).ok()
                            .map(|m| {
                                let bytes = m.len();
                                let formatted = if bytes < 1024 {
                                    format!("{} B", bytes)
                                } else if bytes < 1024 * 1024 {
                                    format!("{:.1} KB", bytes as f64 / 1024.0)
                                } else if bytes < 1024 * 1024 * 1024 {
                                    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
                                } else {
                                    format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
                                };
                                format!(" | {}", formatted)
                            })
                            .unwrap_or_default();
                        (name, size)
                    }
                    None => ("Untitled".to_string(), String::new()),
                }
            });

            let ext_text = super::manager::get_status_bar_text();
            let ext_part = if ext_text.is_empty() {
                String::new()
            } else {
                format!(" | {}", ext_text)
            };

            let status_text = if filename == "Untitled" {
                format!("{}:{}", line, column)
            } else {
                format!("{}:{} | {}{}{}", line, column, filename, size_part, ext_part)
            };

            status_label.set_text(&status_text);
        });
    });
}

// ── Sidebar panel content ────────────────────────────────────────

/// Get content for an extension sidebar panel by running its script.
pub fn get_sidebar_panel_content(ext_id: &str, panel_id: &str, action: &str) -> String {
    let mgr = super::manager::get_manager();
    for ext in mgr.get_extensions() {
        if ext.manifest.id != ext_id || !ext.manifest.enabled {
            continue;
        }
        for panel in &ext.manifest.contributions.sidebar_panels {
            if panel.id != panel_id {
                continue;
            }
            let script_path = ext.path.join(&panel.script);
            let file_path = ACTIVE_FILE_PATH.with(|fp| {
                fp.borrow().as_ref().map(|p| p.to_string_lossy().to_string())
            });
            let file_str = file_path.unwrap_or_default();

            match runner::run_script(&script_path, &[action, &file_str], None) {
                Ok(output) => return output,
                Err(e) => {
                    eprintln!("Sidebar panel script failed: {}", e);
                    return format!("Error: {}", e);
                }
            }
        }
    }
    String::new()
}
