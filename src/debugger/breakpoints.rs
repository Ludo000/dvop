use gtk4::prelude::*;
use gtk4::{gdk, glib};
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

// Global breakpoint manager
thread_local! {
    static BREAKPOINT_MANAGER: RefCell<BreakpointManager> = RefCell::new(BreakpointManager::new());
}

#[derive(Clone, Debug)]
pub struct Breakpoint {
    pub file_path: PathBuf,
    pub line: u32, // 1-based line number
    pub enabled: bool,
}

pub struct BreakpointManager {
    breakpoints: HashMap<PathBuf, Vec<Breakpoint>>,
    observers: Vec<Rc<dyn Fn()>>,
}

impl BreakpointManager {
    fn new() -> Self {
        Self {
            breakpoints: HashMap::new(),
            observers: Vec::new(),
        }
    }

    pub fn toggle_breakpoint(&mut self, file_path: PathBuf, line: u32) {
        let file_breakpoints = self.breakpoints.entry(file_path.clone()).or_insert(Vec::new());
        
        if let Some(index) = file_breakpoints.iter().position(|b| b.line == line) {
            file_breakpoints.remove(index);
        } else {
            file_breakpoints.push(Breakpoint {
                file_path,
                line,
                enabled: true,
            });
        }
        
        self.notify_observers();
    }

    pub fn get_breakpoints(&self) -> Vec<Breakpoint> {
        self.breakpoints.values().flatten().cloned().collect()
    }

    pub fn add_observer(&mut self, observer: Rc<dyn Fn()>) {
        self.observers.push(observer);
    }

    fn notify_observers(&self) {
        for observer in &self.observers {
            observer();
        }
    }
}

pub fn toggle_breakpoint(file_path: PathBuf, line: u32) {
    BREAKPOINT_MANAGER.with(|manager| {
        manager.borrow_mut().toggle_breakpoint(file_path, line);
    });
}

pub fn get_all_breakpoints() -> Vec<Breakpoint> {
    BREAKPOINT_MANAGER.with(|manager| {
        manager.borrow().get_breakpoints()
    })
}

pub fn add_observer(observer: Rc<dyn Fn()>) {
    BREAKPOINT_MANAGER.with(|manager| {
        manager.borrow_mut().add_observer(observer);
    });
}

pub fn setup_breakpoint_attributes(view: &sourceview5::View) {
    let attributes = sourceview5::MarkAttributes::new();
    attributes.set_icon_name("media-record-symbolic"); // Red dot-like icon
    
    let rgba = gdk::RGBA::new(0.8, 0.2, 0.2, 1.0); // Red color
    attributes.set_background(&rgba);
    
    view.set_mark_attributes("breakpoint", &attributes, 10);
}

pub fn setup_gutter_click_handler(
    view: &sourceview5::View, 
    notebook: gtk4::Notebook, 
    file_path_manager: Rc<RefCell<HashMap<u32, PathBuf>>>
) {
    let gutter_widget = sourceview5::prelude::ViewExt::gutter(view, gtk4::TextWindowType::Left);
    let gesture = gtk4::GestureClick::new();
    gesture.set_button(1); // Left click
    
    let view_clone = view.clone();
    let notebook_clone = notebook.clone();
    let file_path_manager = file_path_manager.clone();
    
    gesture.connect_pressed(move |_, _, _, y| {
        // y is in widget coordinates of the gutter
        // We need to convert it to buffer coordinates to find the line
        
        let (_, y_buffer) = view_clone.window_to_buffer_coords(gtk4::TextWindowType::Left, 0, y as i32);
        
        let (iter, _) = view_clone.line_at_y(y_buffer);
        let line = iter.line() + 1;
            
            // Find the page number for this view
            // The view is inside a ScrolledWindow, which is the page child.
            // We need to find the ScrolledWindow parent of the view.
            if let Some(parent) = view_clone.parent() {
                // parent is ScrolledWindow (usually)
                // Note: In some cases there might be intermediate widgets (like Viewport), 
                // but usually ScrolledWindow -> Viewport -> View.
                // notebook.page_num expects the direct child of the notebook.
                
                // Let's try to find the ancestor that is a child of the notebook.
                let mut current = parent;
                while let Some(p) = current.parent() {
                    if &p == notebook_clone.upcast_ref::<gtk4::Widget>() {
                        // current is the child of notebook
                        if let Some(page_num) = notebook_clone.page_num(&current) {
                            if let Some(path) = file_path_manager.borrow().get(&page_num) {
                                toggle_breakpoint(path.clone(), line as u32);
                                update_visual_breakpoints(&view_clone, path);
                            }
                        }
                        break;
                    }
                    current = p;
                }
            }
    });
    
    gutter_widget.add_controller(gesture);
}

pub fn update_visual_breakpoints(view: &sourceview5::View, file_path: &PathBuf) {
    let buffer = view.buffer().downcast::<sourceview5::Buffer>().unwrap();
    
    // Clear existing breakpoint marks
    let start = buffer.start_iter();
    let end = buffer.end_iter();
    buffer.remove_source_marks(&start, &end, Some("breakpoint"));
    
    // Add marks for current breakpoints
    let breakpoints = get_all_breakpoints();
    for bp in breakpoints {
        if &bp.file_path == file_path && bp.enabled {
            if let Some(iter) = buffer.iter_at_line((bp.line - 1) as i32) {
                buffer.create_source_mark(None, "breakpoint", &iter);
            }
        }
    }
}
