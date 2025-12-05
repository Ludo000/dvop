use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Label, Orientation, TextView, ScrolledWindow, glib, Notebook};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};
use std::rc::Rc;
use std::cell::RefCell;
use async_channel;

use super::cargo;

pub fn create_debugger_panel(workspace_root: PathBuf, terminal_notebook: Notebook) -> GtkBox {
    let container = GtkBox::new(Orientation::Vertical, 5);
    container.set_margin_top(10);
    container.set_margin_bottom(10);
    container.set_margin_start(10);
    container.set_margin_end(10);
    
    // Title
    let title = Label::builder()
        .label("Rust Debugger")
        .css_classes(vec!["title-2"])
        .halign(gtk4::Align::Start)
        .build();
    container.append(&title);

    // Target info
    let target_box = GtkBox::new(Orientation::Horizontal, 5);
    let target_label = Label::new(Some("Target: Detecting..."));
    target_box.append(&target_label);
    container.append(&target_box);

    // Controls
    let controls_box = GtkBox::new(Orientation::Horizontal, 5);
    let run_button = Button::with_label("Run");
    run_button.set_sensitive(false);
    controls_box.append(&run_button);
    container.append(&controls_box);

    // Detect target
    let target = cargo::detect_cargo_target(&workspace_root);
    let target_name = Rc::new(RefCell::new(None));

    if let Some(t) = target {
        target_label.set_text(&format!("Target: {} ({})", t.name, t.kind));
        run_button.set_sensitive(true);
        *target_name.borrow_mut() = Some(t.name);
    } else {
        target_label.set_text("Target: None found");
    }

    // Run button handler
    let workspace_root_clone = workspace_root.clone();
    let target_name_clone = target_name.clone();
    let terminal_notebook_clone = terminal_notebook.clone();
    
    let output_tab_widget = Rc::new(RefCell::new(None::<gtk4::Widget>));
    let output_text_view = Rc::new(RefCell::new(None::<TextView>));

    run_button.connect_clicked(move |_| {
        // Ensure tab exists
        let mut tab_widget = output_tab_widget.borrow_mut();
        let mut text_view_ref = output_text_view.borrow_mut();
        
        let needs_creation = if let Some(widget) = tab_widget.as_ref() {
            terminal_notebook_clone.page_num(widget).is_none()
        } else {
            true
        };

        if needs_creation {
             let scrolled_window = ScrolledWindow::builder()
                .hscrollbar_policy(gtk4::PolicyType::Automatic)
                .vscrollbar_policy(gtk4::PolicyType::Automatic)
                .vexpand(true)
                .build();

             let text_view = TextView::builder()
                .editable(false)
                .monospace(true)
                .build();
             
             scrolled_window.set_child(Some(&text_view));
             
             let label = Label::new(Some("Debug Output"));
             terminal_notebook_clone.append_page(&scrolled_window, Some(&label));
             terminal_notebook_clone.set_tab_reorderable(&scrolled_window, true);
             terminal_notebook_clone.set_tab_detachable(&scrolled_window, true);
             
             *tab_widget = Some(scrolled_window.upcast::<gtk4::Widget>());
             *text_view_ref = Some(text_view);
        }
        
        // Switch to tab
        if let Some(widget) = tab_widget.as_ref() {
             if let Some(page) = terminal_notebook_clone.page_num(widget) {
                 terminal_notebook_clone.set_current_page(Some(page));
             }
             widget.set_visible(true);
        }

        let text_view = text_view_ref.as_ref().unwrap().clone();
        let buffer = text_view.buffer();
        buffer.set_text(""); // Clear previous output

        if let Some(name) = target_name_clone.borrow().as_ref() {
            let name = name.clone();
            buffer.insert_at_cursor(&format!("Running target: {}...\n", name));
            
            let workspace_root = workspace_root_clone.clone();
            let (sender, receiver) = async_channel::unbounded::<String>();
            
            // Spawn local future to handle results
            let text_view = text_view.clone();
            glib::MainContext::default().spawn_local(async move {
                while let Ok(msg) = receiver.recv().await {
                    let buffer = text_view.buffer();
                    buffer.insert_at_cursor(&msg);
                }
            });

            std::thread::spawn(move || {
                // Run cargo run
                let mut child = Command::new("cargo")
                    .arg("run")
                    .arg("--bin")
                    .arg(&name)
                    .current_dir(&workspace_root)
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn();

                match child {
                    Ok(mut child) => {
                        let stdout = child.stdout.take();
                        let stderr = child.stderr.take();

                        let sender_out = sender.clone();
                        let thread_out = std::thread::spawn(move || {
                            if let Some(out) = stdout {
                                let reader = BufReader::new(out);
                                for line in reader.lines() {
                                    if let Ok(l) = line {
                                        let _ = sender_out.send_blocking(format!("{}\n", l));
                                    }
                                }
                            }
                        });

                        let sender_err = sender.clone();
                        let thread_err = std::thread::spawn(move || {
                            if let Some(err) = stderr {
                                let reader = BufReader::new(err);
                                for line in reader.lines() {
                                    if let Ok(l) = line {
                                        let _ = sender_err.send_blocking(format!("{}\n", l));
                                    }
                                }
                            }
                        });

                        // Wait for process to finish
                        let status = child.wait();
                        
                        // Wait for readers to finish
                        let _ = thread_out.join();
                        let _ = thread_err.join();

                        match status {
                            Ok(s) => {
                                if s.success() {
                                    let _ = sender.send_blocking("\nProcess finished successfully.\n".to_string());
                                } else {
                                    let _ = sender.send_blocking(format!("\nProcess failed with code: {:?}\n", s.code()));
                                }
                            }
                            Err(e) => {
                                let _ = sender.send_blocking(format!("\nError waiting for process: {}\n", e));
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender.send_blocking(format!("Error starting cargo: {}\n", e));
                    }
                }
            });
        }
    });
    
    container
}
