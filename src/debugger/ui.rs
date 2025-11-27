// UI components for the Rust debugger

use gtk4::prelude::*;
use sourceview5::prelude::ViewExt;
use gtk4::{
    Box as GtkBox, Button, Label, ListBox, ListBoxRow, Orientation, ScrolledWindow,
};
use std::cell::RefCell;
use std::rc::Rc;
use std::path::PathBuf;
use glib;

#[derive(Debug, Clone, PartialEq)]
enum DebugState {
    NotRunning,
    Running,
    Paused,
}

#[derive(Debug, Clone)]
pub enum DebugEvent {
    Stopped { reason: String, line: Option<u32>, file: Option<String> },
    Running,
    Exited,
    StackFrame { frames: Vec<super::StackFrame> },
    Variables { vars: Vec<super::Variable> },
    /// Configuration used to start GDB (command + args)
    GdbConfig { config: String },
    /// Program output (stdout/stderr from the debugged program)
    ProgramOutput { text: String },
}

// Global debugger instance
thread_local! {
    static DEBUGGER: RefCell<super::Debugger> = RefCell::new(super::Debugger::new());
    static CURRENT_DIR: RefCell<Option<PathBuf>> = RefCell::new(None);
    static BREAKPOINT_LIST: RefCell<Option<ListBox>> = RefCell::new(None);
    static VARIABLES_LIST: RefCell<Option<ListBox>> = RefCell::new(None);
    static CALLSTACK_LIST: RefCell<Option<ListBox>> = RefCell::new(None);
    static DEBUG_STATE_REF: RefCell<Option<Rc<RefCell<DebugState>>>> = RefCell::new(None);
    static GDB_CONFIG_LABEL: RefCell<Option<Label>> = RefCell::new(None);
    static TERMINAL_NOTEBOOK: RefCell<Option<gtk4::Notebook>> = RefCell::new(None);
    static DEBUG_TERMINAL_PAGE: RefCell<Option<u32>> = RefCell::new(None);
    static DEBUG_OUTPUT_SENDER: RefCell<Option<std::sync::mpsc::Sender<String>>> = RefCell::new(None);
}

/// Set the terminal notebook reference for debug output
pub fn set_terminal_notebook(notebook: gtk4::Notebook) {
    TERMINAL_NOTEBOOK.with(|nb| {
        *nb.borrow_mut() = Some(notebook);
    });
    
    // Set up the channel for debug output from background thread using standard mpsc
    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    DEBUG_OUTPUT_SENDER.with(|s| {
        *s.borrow_mut() = Some(sender);
    });
    
    // Poll the receiver on the GTK main thread using a timeout
    glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
        // Try to receive all pending messages
        while let Ok(text) = receiver.try_recv() {
            write_to_debug_terminal(&text);
        }
        glib::ControlFlow::Continue
    });
}

/// Set the current directory for the debugger
pub fn set_debugger_current_dir(dir: PathBuf) {
    CURRENT_DIR.with(|d| {
        *d.borrow_mut() = Some(dir.clone());
    });
    
    // Try to find and set the binary
    if let Some(binary) = super::find_rust_binary(&dir) {
        DEBUGGER.with(|dbg| {
            dbg.borrow_mut().set_binary(binary.clone());
            crate::status_log::log_success(&format!("Debug binary found: {}", 
                binary.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")));
        });
    } else {
        crate::status_log::log_error("No debug binary found. Build your project with 'cargo build' first.");
    }
}

/// Add a breakpoint at the given file and line
pub fn add_breakpoint(file: String, line: u32) {
    DEBUGGER.with(|dbg| {
        dbg.borrow_mut().add_breakpoint(file.clone(), line);
        crate::status_log::log_success(&format!("Breakpoint added: {}:{}", file, line));
    });
}

/// Remove a breakpoint at the given file and line  
pub fn remove_breakpoint(file: &str, line: u32) {
    DEBUGGER.with(|dbg| {
        dbg.borrow_mut().remove_breakpoint(file, line);
        crate::status_log::log_info(&format!("Breakpoint removed: {}:{}", file, line));
    });
}

/// Refresh the breakpoint list UI
pub fn refresh_breakpoint_list_ui() {
    BREAKPOINT_LIST.with(|list_ref| {
        if let Some(list) = list_ref.borrow().as_ref() {
            refresh_breakpoints_list(list);
        }
    });
}

/// Handle debug events from GDB thread
pub fn handle_debug_event(event: DebugEvent) {
    // Schedule GTK updates on the main thread
    match event {
        DebugEvent::Stopped { reason: _, line, file } => {
            glib::idle_add_local_once(move || {
                DEBUG_STATE_REF.with(|state_ref| {
                    if let Some(state) = state_ref.borrow().as_ref() {
                        *state.borrow_mut() = DebugState::Paused;
                    }
                });
                
                let location = if let (Some(f), Some(l)) = (file, line) {
                    format!(" at {}:{}", f.split('/').last().unwrap_or(&f), l)
                } else {
                    String::new()
                };
                println!("[UI] Breakpoint hit{}", location);
                crate::status_log::log_info(&format!("⏸️  Breakpoint hit{}", location));
            });
        }
        DebugEvent::Running => {
            glib::idle_add_local_once(|| {
                DEBUG_STATE_REF.with(|state_ref| {
                    if let Some(state) = state_ref.borrow().as_ref() {
                        *state.borrow_mut() = DebugState::Running;
                    }
                });
                println!("[UI] Program running");
                crate::status_log::log_success("▶️  Program is running");
            });
        }
        DebugEvent::Exited => {
            glib::idle_add_local_once(|| {
                DEBUG_STATE_REF.with(|state_ref| {
                    if let Some(state) = state_ref.borrow().as_ref() {
                        *state.borrow_mut() = DebugState::NotRunning;
                    }
                });
                println!("[UI] Program exited");
                crate::status_log::log_success("✓ Program exited");
            });
        }
        DebugEvent::GdbConfig { config } => {
            // Update the visible label with the config string
            glib::idle_add_local_once(move || {
                GDB_CONFIG_LABEL.with(|lbl| {
                    if let Some(label) = lbl.borrow().as_ref() {
                        label.set_text(&format!("GDB: {}", config));
                    }
                });
            });
        }
        DebugEvent::ProgramOutput { text } => {
            // Send output via channel from background thread to GTK thread
            DEBUG_OUTPUT_SENDER.with(|s| {
                if let Some(sender) = s.borrow().as_ref() {
                    let _ = sender.send(text);
                }
            });
        }
        DebugEvent::StackFrame { frames } => {
            glib::idle_add_local_once(move || {
                CALLSTACK_LIST.with(|list_ref| {
                    if let Some(list) = list_ref.borrow().as_ref() {
                        clear_list(list);
                        for frame in frames {
                            let location = format!("{}:{}", frame.file, frame.line);
                            add_stackframe_to_list(list, &frame.function, &location);
                        }
                    }
                });
            });
        }
        DebugEvent::Variables { vars } => {
            glib::idle_add_local_once(move || {
                VARIABLES_LIST.with(|list_ref| {
                    if let Some(list) = list_ref.borrow().as_ref() {
                        clear_list(list);
                        for var in vars {
                            add_variable_to_list(list, &var.name, &var.var_type, &var.value);
                        }
                    }
                });
            });
        }
    }
}

/// Write text to the debug output terminal
fn write_to_debug_terminal(text: &str) {
    use vte4::TerminalExt;
    println!("[DEBUG-UI] write_to_debug_terminal called with: {}", text.trim());
    TERMINAL_NOTEBOOK.with(|nb_ref| {
        if let Some(notebook) = nb_ref.borrow().as_ref() {
            println!("[DEBUG-UI] Terminal notebook found");
            DEBUG_TERMINAL_PAGE.with(|page_ref| {
                // Get or create the debug terminal page
                let page_num = if let Some(page) = *page_ref.borrow() {
                    println!("[DEBUG-UI] Using existing debug terminal page: {}", page);
                    page
                } else {
                    println!("[DEBUG-UI] Creating new debug output tab");
                    // Create new debug output tab
                    let terminal = crate::ui::terminal::create_read_only_terminal();
                    let terminal_box = crate::ui::terminal::create_terminal_box(&terminal);
                    
                    let (tab_widget, _tab_label, _tab_close_button) = crate::ui::create_tab_widget("Debug Output");
                    
                    let page = notebook.append_page(&terminal_box, Some(&tab_widget));
                    *page_ref.borrow_mut() = Some(page);
                    
                    // Switch to the debug output tab
                    notebook.set_current_page(Some(page));
                    
                    println!("[DEBUG-UI] Created debug output tab at page: {}", page);
                    page
                };
                
                // Write to the terminal
                if let Some(page_widget) = notebook.nth_page(Some(page_num)) {
                    if let Ok(scrolled) = page_widget.downcast::<ScrolledWindow>() {
                        if let Some(terminal) = scrolled.child().and_then(|w| w.downcast::<vte4::Terminal>().ok()) {
                            println!("[DEBUG-UI] Writing to terminal: {} bytes", text.len());
                            terminal.feed(text.as_bytes());
                        } else {
                            println!("[DEBUG-UI] Failed to get terminal widget");
                        }
                    } else {
                        println!("[DEBUG-UI] Failed to downcast to ScrolledWindow");
                    }
                } else {
                    println!("[DEBUG-UI] Failed to get page widget");
                }
            });
        } else {
            println!("[DEBUG-UI] Terminal notebook NOT found!");
        }
    });
}

/// Create the debugger panel UI
pub fn create_debugger_panel() -> GtkBox {
    let main_box = GtkBox::new(Orientation::Vertical, 0);
    main_box.add_css_class("debugger-panel");
    main_box.set_hexpand(true);
    main_box.set_vexpand(true);

    // Shared debug state
    let debug_state = Rc::new(RefCell::new(DebugState::NotRunning));

    // Header section
    let header_box = GtkBox::new(Orientation::Horizontal, 8);
    header_box.set_margin_start(12);
    header_box.set_margin_end(12);
    header_box.set_margin_top(8);
    header_box.set_margin_bottom(8);

    let title_label = Label::new(Some("RUST DEBUGGER"));
    title_label.add_css_class("sidebar-title");
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_hexpand(true);
    header_box.append(&title_label);

    main_box.append(&header_box);

    // Separator
    let separator = gtk4::Separator::new(Orientation::Horizontal);
    main_box.append(&separator);

    // Control buttons section
    let controls_box = GtkBox::new(Orientation::Horizontal, 4);
    controls_box.set_margin_start(12);
    controls_box.set_margin_end(12);
    controls_box.set_margin_top(8);
    controls_box.set_margin_bottom(8);
    controls_box.set_halign(gtk4::Align::Center);

    let start_button = Button::new();
    start_button.set_icon_name("media-playback-start-symbolic");
    start_button.set_tooltip_text(Some("Start/Continue"));
    start_button.add_css_class("flat");
    controls_box.append(&start_button);

    let pause_button = Button::new();
    pause_button.set_icon_name("media-playback-pause-symbolic");
    pause_button.set_tooltip_text(Some("Pause"));
    pause_button.add_css_class("flat");
    controls_box.append(&pause_button);

    let stop_button = Button::new();
    stop_button.set_icon_name("media-playback-stop-symbolic");
    stop_button.set_tooltip_text(Some("Stop"));
    stop_button.add_css_class("flat");
    controls_box.append(&stop_button);

    let step_over_button = Button::new();
    step_over_button.set_icon_name("go-next-symbolic");
    step_over_button.set_tooltip_text(Some("Step Over"));
    step_over_button.add_css_class("flat");
    controls_box.append(&step_over_button);

    let step_into_button = Button::new();
    step_into_button.set_icon_name("go-down-symbolic");
    step_into_button.set_tooltip_text(Some("Step Into"));
    step_into_button.add_css_class("flat");
    controls_box.append(&step_into_button);

    let step_out_button = Button::new();
    step_out_button.set_icon_name("go-up-symbolic");
    step_out_button.set_tooltip_text(Some("Step Out"));
    step_out_button.add_css_class("flat");
    controls_box.append(&step_out_button);

    main_box.append(&controls_box);

    // Separator
    let separator2 = gtk4::Separator::new(Orientation::Horizontal);
    main_box.append(&separator2);

    // Breakpoints section
    let breakpoints_header = GtkBox::new(Orientation::Horizontal, 8);
    breakpoints_header.set_margin_start(12);
    breakpoints_header.set_margin_top(8);
    breakpoints_header.set_margin_bottom(4);
    
    let breakpoints_label = Label::new(Some("Breakpoints"));
    breakpoints_label.add_css_class("sidebar-section-title");
    breakpoints_label.set_halign(gtk4::Align::Start);
    breakpoints_label.set_hexpand(true);
    breakpoints_header.append(&breakpoints_label);
    
    main_box.append(&breakpoints_header);

    let breakpoints_scrolled = ScrolledWindow::new();
    breakpoints_scrolled.set_vexpand(true);
    breakpoints_scrolled.set_margin_start(8);
    breakpoints_scrolled.set_margin_end(8);
    breakpoints_scrolled.set_margin_bottom(8);
    breakpoints_scrolled.set_min_content_height(100);

    let breakpoints_list = ListBox::new();
    breakpoints_list.add_css_class("navigation-sidebar");
    breakpoints_scrolled.set_child(Some(&breakpoints_list));
    main_box.append(&breakpoints_scrolled);

    // Store the breakpoint list globally so we can refresh it from anywhere
    BREAKPOINT_LIST.with(|list| {
        *list.borrow_mut() = Some(breakpoints_list.clone());
    });

    // Placeholder for breakpoints
    let bp_placeholder_label = Label::new(Some("No breakpoints set"));
    bp_placeholder_label.add_css_class("dim-label");
    bp_placeholder_label.set_margin_top(20);
    bp_placeholder_label.set_margin_bottom(20);
    breakpoints_list.set_placeholder(Some(&bp_placeholder_label));
    
    // Refresh breakpoints list
    refresh_breakpoints_list(&breakpoints_list);

    // Variables section
    let variables_label = Label::new(Some("Variables"));
    variables_label.add_css_class("sidebar-section-title");
    variables_label.set_margin_start(12);
    variables_label.set_margin_top(8);
    variables_label.set_margin_bottom(4);
    variables_label.set_halign(gtk4::Align::Start);
    main_box.append(&variables_label);

    let variables_scrolled = ScrolledWindow::new();
    variables_scrolled.set_vexpand(true);
    variables_scrolled.set_margin_start(8);
    variables_scrolled.set_margin_end(8);
    variables_scrolled.set_margin_bottom(8);
    variables_scrolled.set_min_content_height(150);

    let variables_list = ListBox::new();
    variables_list.add_css_class("navigation-sidebar");
    variables_scrolled.set_child(Some(&variables_list));
    main_box.append(&variables_scrolled);

    // Store variables list globally
    VARIABLES_LIST.with(|list| {
        *list.borrow_mut() = Some(variables_list.clone());
    });

    // Placeholder for variables
    let placeholder_label = Label::new(Some("No debug session active"));
    placeholder_label.add_css_class("dim-label");
    placeholder_label.set_margin_top(20);
    placeholder_label.set_margin_bottom(20);
    variables_list.set_placeholder(Some(&placeholder_label));

    // GDB config display (debug only)
    let gdb_config_label = Label::new(Some("GDB: (not started)"));
    gdb_config_label.add_css_class("dim-label");
    gdb_config_label.set_margin_start(12);
    gdb_config_label.set_margin_top(6);
    gdb_config_label.set_halign(gtk4::Align::Start);
    gdb_config_label.set_selectable(true);
    gdb_config_label.set_wrap(true);
    main_box.append(&gdb_config_label);

    // Store the config label globally so we can update it from events
    GDB_CONFIG_LABEL.with(|lbl| {
        *lbl.borrow_mut() = Some(gdb_config_label.clone());
    });

    // Call stack section
    let callstack_label = Label::new(Some("Call Stack"));
    callstack_label.add_css_class("sidebar-section-title");
    callstack_label.set_margin_start(12);
    callstack_label.set_margin_top(8);
    callstack_label.set_margin_bottom(4);
    callstack_label.set_halign(gtk4::Align::Start);
    main_box.append(&callstack_label);

    let callstack_scrolled = ScrolledWindow::new();
    callstack_scrolled.set_vexpand(true);
    callstack_scrolled.set_margin_start(8);
    callstack_scrolled.set_margin_end(8);
    callstack_scrolled.set_margin_bottom(12);
    callstack_scrolled.set_min_content_height(100);

    let callstack_list = ListBox::new();
    callstack_list.add_css_class("navigation-sidebar");
    callstack_scrolled.set_child(Some(&callstack_list));
    main_box.append(&callstack_scrolled);

    // Store callstack list globally
    CALLSTACK_LIST.with(|list| {
        *list.borrow_mut() = Some(callstack_list.clone());
    });

    // Placeholder for call stack
    let cs_placeholder_label = Label::new(Some("No debug session active"));
    cs_placeholder_label.add_css_class("dim-label");
    cs_placeholder_label.set_margin_top(20);
    cs_placeholder_label.set_margin_bottom(20);
    callstack_list.set_placeholder(Some(&cs_placeholder_label));

    // Set up button handlers with state management
    let debug_state_for_start = debug_state.clone();
    
    // Store debug state globally for event handler
    DEBUG_STATE_REF.with(|global_state| {
        *global_state.borrow_mut() = Some(debug_state.clone());
    });
    
    let variables_list_for_start = variables_list.clone();
    let callstack_list_for_start = callstack_list.clone();
    start_button.connect_clicked(move |btn| {
        let mut state = debug_state_for_start.borrow_mut();
        match *state {
            DebugState::NotRunning => {
                // Check if binary is set
                let binary_ready = DEBUGGER.with(|dbg| {
                    let debugger = dbg.borrow();
                    debugger.get_binary().is_some()
                });
                
                if !binary_ready {
                    crate::status_log::log_error("No debug binary found. Run 'cargo build' to create one.");
                    
                    // Try to find the binary again
                    CURRENT_DIR.with(|dir| {
                        if let Some(ref current_dir) = *dir.borrow() {
                            if let Some(binary) = super::find_rust_binary(current_dir) {
                                DEBUGGER.with(|dbg| {
                                    dbg.borrow_mut().set_binary(binary.clone());
                                    crate::status_log::log_success(&format!("Found binary: {}", 
                                        binary.file_name().and_then(|n| n.to_str()).unwrap_or("unknown")));
                                });
                            }
                        }
                    });
                    return;
                }
                
                // Try to start the debugger
                println!("[UI] Starting debugger...");
                DEBUGGER.with(|dbg| {
                    // Get binary name before starting (to avoid borrow conflicts)
                    let binary_name = dbg.borrow()
                        .get_binary()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    println!("[UI] Binary: {}", binary_name);
                    println!("[UI] Breakpoints: {}", dbg.borrow().get_breakpoints().len());
                    
                    match dbg.borrow_mut().start() {
                        Ok(_) => {
                            println!("[UI] Debugger started successfully!");
                            *state = DebugState::Running;
                            btn.set_sensitive(false);
                            
                            crate::status_log::log_success(&format!(
                                "Debug session started for '{}'. Program is running under GDB.", 
                                binary_name
                            ));
                            
                            // Clear previous data
                            clear_list(&variables_list_for_start);
                            clear_list(&callstack_list_for_start);
                        }
                        Err(e) => {
                            println!("[UI] Failed to start debugger: {}", e);
                            crate::status_log::log_error(&format!("Failed to start debugger: {}", e));
                        }
                    }
                });
            }
            DebugState::Paused => {
                // When paused, continue execution
                DEBUGGER.with(|dbg| {
                    match dbg.borrow().continue_execution() {
                        Ok(_) => {
                            *state = DebugState::Running;
                            crate::status_log::log_info("Debug session continued");
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!("Failed to continue: {}", e));
                        }
                    }
                });
            }
            _ => {}
        }
    });

    let debug_state_for_pause = debug_state.clone();
    let start_button_for_pause = start_button.clone();
    pause_button.connect_clicked(move |_| {
        let mut state = debug_state_for_pause.borrow_mut();
        if *state == DebugState::Running {
            *state = DebugState::Paused;
            start_button_for_pause.set_sensitive(true);
            crate::status_log::log_info("Debug session paused");
        }
    });

    let debug_state_for_stop = debug_state.clone();
    let start_button_for_stop = start_button.clone();
    let variables_list_for_stop = variables_list.clone();
    let callstack_list_for_stop = callstack_list.clone();
    stop_button.connect_clicked(move |_| {
        let mut state = debug_state_for_stop.borrow_mut();
        if *state != DebugState::NotRunning {
            DEBUGGER.with(|dbg| {
                dbg.borrow_mut().stop();
            });
            
            *state = DebugState::NotRunning;
            start_button_for_stop.set_sensitive(true);
            crate::status_log::log_info("Debug session stopped");
            
            // Clear all debug info
            clear_list(&variables_list_for_stop);
            clear_list(&callstack_list_for_stop);
        }
    });

    let debug_state_for_step_over = debug_state.clone();
    step_over_button.connect_clicked(move |_| {
        let state = debug_state_for_step_over.borrow();
        if *state == DebugState::Paused {
            DEBUGGER.with(|dbg| {
                match dbg.borrow().step_over() {
                    Ok(_) => crate::status_log::log_info("Step over"),
                    Err(e) => crate::status_log::log_error(&format!("Step failed: {}", e)),
                }
            });
        } else {
            crate::status_log::log_error("Cannot step: debugger not paused");
        }
    });

    let debug_state_for_step_into = debug_state.clone();
    step_into_button.connect_clicked(move |_| {
        let state = debug_state_for_step_into.borrow();
        if *state == DebugState::Paused {
            DEBUGGER.with(|dbg| {
                match dbg.borrow().step_into() {
                    Ok(_) => crate::status_log::log_info("Step into"),
                    Err(e) => crate::status_log::log_error(&format!("Step failed: {}", e)),
                }
            });
        } else {
            crate::status_log::log_error("Cannot step: debugger not paused");
        }
    });

    let debug_state_for_step_out = debug_state.clone();
    step_out_button.connect_clicked(move |_| {
        let state = debug_state_for_step_out.borrow();
        if *state == DebugState::Paused {
            crate::status_log::log_info("Step out");
            // TODO: Implement actual step out command
        } else {
            crate::status_log::log_error("Cannot step: debugger not paused");
        }
    });

    main_box
}

/// Refresh the breakpoints list from the debugger state
fn refresh_breakpoints_list(list: &ListBox) {
    clear_list(list);
    
    DEBUGGER.with(|dbg| {
        for bp in dbg.borrow().get_breakpoints() {
            add_breakpoint_to_list(list, &bp.file, bp.line);
        }
    });
}

/// Add a breakpoint to the visual list
fn add_breakpoint_to_list(list: &ListBox, file: &str, line: u32) {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);
    
    // Show just the filename for readability, but store full path
    let display_name = std::path::Path::new(file)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(file);
    let label = Label::new(Some(&format!("{}:{}", display_name, line)));
    label.set_xalign(0.0);
    label.set_hexpand(true);
    label.set_tooltip_text(Some(file)); // Full path in tooltip
    hbox.append(&label);
    
    // Add remove button
    let remove_btn = Button::from_icon_name("user-trash-symbolic");
    remove_btn.add_css_class("flat");
    remove_btn.set_tooltip_text(Some("Remove breakpoint"));
    
    let file_owned = file.to_string();
    remove_btn.connect_clicked(move |_| {
        remove_breakpoint(&file_owned, line);
    });
    hbox.append(&remove_btn);
    
    row.set_child(Some(&hbox));
    list.append(&row);
}

/// Helper function to add a variable to the variables list
fn add_variable_to_list(list: &ListBox, name: &str, var_type: &str, value: &str) {
    let row = ListBoxRow::new();
    let hbox = GtkBox::new(Orientation::Horizontal, 8);
    hbox.set_margin_start(8);
    hbox.set_margin_end(8);
    hbox.set_margin_top(4);
    hbox.set_margin_bottom(4);
    
    let name_label = Label::new(Some(name));
    name_label.set_width_chars(10);
    name_label.set_xalign(0.0);
    name_label.add_css_class("monospace");
    hbox.append(&name_label);
    
    let type_label = Label::new(Some(var_type));
    type_label.set_width_chars(8);
    type_label.set_xalign(0.0);
    type_label.add_css_class("dim-label");
    hbox.append(&type_label);
    
    let value_label = Label::new(Some(value));
    value_label.set_hexpand(true);
    value_label.set_xalign(0.0);
    value_label.add_css_class("monospace");
    hbox.append(&value_label);
    
    row.set_child(Some(&hbox));
    list.append(&row);
}

/// Helper function to add a stack frame to the call stack list
fn add_stackframe_to_list(list: &ListBox, function: &str, location: &str) {
    let row = ListBoxRow::new();
    let vbox = GtkBox::new(Orientation::Vertical, 2);
    vbox.set_margin_start(8);
    vbox.set_margin_end(8);
    vbox.set_margin_top(4);
    vbox.set_margin_bottom(4);
    
    let func_label = Label::new(Some(function));
    func_label.set_xalign(0.0);
    func_label.add_css_class("monospace");
    vbox.append(&func_label);
    
    let loc_label = Label::new(Some(location));
    loc_label.set_xalign(0.0);
    loc_label.add_css_class("dim-label");
    vbox.append(&loc_label);
    
    row.set_child(Some(&vbox));
    list.append(&row);
}

/// Helper function to clear a ListBox
fn clear_list(list: &ListBox) {
    while let Some(row) = list.first_child() {
        list.remove(&row);
    }
}

/// Update the debugger panel with current directory
pub fn update_debugger_visibility(debugger_panel: &GtkBox, current_dir: &std::path::Path) {
    let has_rust = super::has_rust_files(current_dir);
    debugger_panel.set_visible(has_rust);
    
    if has_rust {
        set_debugger_current_dir(current_dir.to_path_buf());
    }
}

/// Add breakpoint toggle functionality to a source view
/// Call this when creating a new source view for Rust files
pub fn setup_breakpoint_support(
    source_view: &sourceview5::View,
    file_path: Option<&std::path::Path>,
) {
    // Only add breakpoint support for Rust files
    if let Some(path) = file_path {
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            return;
        }
    } else {
        return;
    }

    // Convert to absolute path for GDB
    let file_path_str = file_path
        .and_then(|p| {
            if p.is_absolute() {
                p.to_str().map(|s| s.to_string())
            } else {
                std::env::current_dir()
                    .ok()
                    .and_then(|cwd| cwd.join(p).canonicalize().ok())
                    .and_then(|abs| abs.to_str().map(|s| s.to_string()))
            }
        })
        .unwrap_or_else(|| "".to_string());
    
    // Add right-click gesture for breakpoint menu
    let right_click = gtk4::GestureClick::new();
    right_click.set_button(3); // Right click
    
    let view_for_click = source_view.clone();
    let file_for_click = file_path_str.clone();
    right_click.connect_pressed(move |_, _, x, y| {
        // For now, just show a simple message
        let buffer_coords = view_for_click.window_to_buffer_coords(
            gtk4::TextWindowType::Widget,
            x as i32,
            y as i32,
        );
        
        if let Some(iter) = view_for_click.iter_at_location(buffer_coords.0, buffer_coords.1) {
            let line = iter.line() as u32 + 1; // Line numbers are 1-based for users
            
            // Toggle breakpoint directly on right-click
            DEBUGGER.with(|dbg| {
                let mut debugger = dbg.borrow_mut();
                let has_bp = debugger.get_breakpoints().iter().any(|bp| {
                    bp.file == file_for_click && bp.line == line
                });
                
                if has_bp {
                    debugger.remove_breakpoint(&file_for_click, line);
                    crate::status_log::log_info(&format!("Breakpoint removed: {}:{}", 
                        file_for_click.split('/').last().unwrap_or(&file_for_click), line));
                } else {
                    debugger.add_breakpoint(file_for_click.clone(), line);
                    crate::status_log::log_success(&format!("Breakpoint added: {}:{}", 
                        file_for_click.split('/').last().unwrap_or(&file_for_click), line));
                }
            });
            
            // Refresh the breakpoint list UI
            refresh_breakpoint_list_ui();
        }
    });
    
    source_view.add_controller(right_click);
    
    // Add visual indicator for breakpoints - enable line numbers
    if let Ok(sv) = source_view.clone().downcast::<sourceview5::View>() {
        sv.set_show_line_numbers(true);
    }
    
    // Add click handler for line numbers to toggle breakpoints  
    let click_line = gtk4::GestureClick::new();
    click_line.set_button(1); // Left click
    
    let view_for_line_click = source_view.clone();
    click_line.connect_pressed(move |gesture, _, x, y| {
        // Check if click is in the line numbers area (gutter) by checking x position
        // Gutter is usually on the left, so x < some threshold
        if x < 50.0 { // Approximate gutter width
            let buffer_coords = view_for_line_click.window_to_buffer_coords(
                gtk4::TextWindowType::Widget,
                x as i32,
                y as i32,
            );
            
            if let Some(iter) = view_for_line_click.iter_at_location(buffer_coords.0, buffer_coords.1) {
                let line = iter.line() as u32 + 1;
                
                // Toggle breakpoint
                DEBUGGER.with(|dbg| {
                    let mut debugger = dbg.borrow_mut();
                    let has_bp = debugger.get_breakpoints().iter().any(|bp| {
                        bp.file == file_path_str && bp.line == line
                    });
                    
                    if has_bp {
                        debugger.remove_breakpoint(&file_path_str, line);
                        crate::status_log::log_info(&format!("Breakpoint removed: {}:{}", 
                            file_path_str.split('/').last().unwrap_or(&file_path_str), line));
                    } else {
                        debugger.add_breakpoint(file_path_str.clone(), line);
                        crate::status_log::log_success(&format!("Breakpoint added: {}:{}", 
                            file_path_str.split('/').last().unwrap_or(&file_path_str), line));
                    }
                });
                
                // Refresh the breakpoint list UI
                refresh_breakpoint_list_ui();
                
                gesture.set_state(gtk4::EventSequenceState::Claimed);
            }
        }
    });
    
    source_view.add_controller(click_line);
}
