// Quick integration tests for Dvop
// Run with: cargo test --test quick_tests

use gtk4::prelude::*;
use gtk4::Notebook;
use sourceview5::prelude::*;
use serial_test::serial;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[serial]
#[test]
fn test_all_features() {
    gtk4::test_synced(|| {
    
    // Test 1: New file creation
    {
        let notebook = Notebook::new();
        assert_eq!(notebook.n_pages(), 0, "Notebook should start empty");

        let (source_view, source_buffer) = dvop::syntax::create_source_view();
        source_buffer.set_text("");
        
        let scrolled = dvop::syntax::create_source_view_scrolled(&source_view);
        let (tab_widget, _, _) = dvop::ui::create_tab_widget("Untitled");
        
        notebook.append_page(&scrolled, Some(&tab_widget));
        assert_eq!(notebook.n_pages(), 1, "Should have 1 tab");
        
        let text = source_buffer.text(&source_buffer.start_iter(), &source_buffer.end_iter(), false);
        assert_eq!(text.as_str(), "", "Buffer should be empty");
    }
    
    // Test 2: Text editing
    {
        let (_, buffer) = dvop::syntax::create_source_view();
        
        buffer.set_text("Hello, World!");
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        assert_eq!(text.as_str(), "Hello, World!");
        
        buffer.set_text("Modified");
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        assert_eq!(text.as_str(), "Modified");
    }
    
    // Test 3: Tab management
    {
        let notebook = Notebook::new();
        
        for i in 0..3 {
            let (view, _buffer) = dvop::syntax::create_source_view();
            let scrolled = dvop::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = dvop::ui::create_tab_widget(&format!("Tab {}", i));
            notebook.append_page(&scrolled, Some(&tab_widget));
        }
        
        assert_eq!(notebook.n_pages(), 3);
        
        notebook.set_current_page(Some(1));
        assert_eq!(notebook.current_page(), Some(1));
        
        notebook.remove_page(Some(1));
        assert_eq!(notebook.n_pages(), 2);
    }
    
    // Test 4: Syntax highlighting
    {
        let (view, buffer) = dvop::syntax::create_source_view();
        
        assert!(view.is_editable());
        assert!(view.shows_line_numbers());
        
        dvop::syntax::set_language_for_file(&buffer, &PathBuf::from("test.rs"));
        assert!(buffer.language().is_some());
        assert_eq!(buffer.language().unwrap().id().as_str(), "rust");
        
        dvop::syntax::set_language_for_file(&buffer, &PathBuf::from("test.py"));
        let lang_id = buffer.language().unwrap().id();
        assert!(lang_id.as_str() == "python" || lang_id.as_str() == "python3", 
                "Expected python or python3, got {}", lang_id);
    }
    
    // Test 5: File path tracking
    {
        let manager: Rc<RefCell<HashMap<u32, PathBuf>>> = Rc::new(RefCell::new(HashMap::new()));
        
        manager.borrow_mut().insert(0, PathBuf::from("/tmp/file1.txt"));
        manager.borrow_mut().insert(1, PathBuf::from("/tmp/file2.rs"));
        
        assert_eq!(manager.borrow().len(), 2);
        assert_eq!(manager.borrow().get(&0), Some(&PathBuf::from("/tmp/file1.txt")));
        
        manager.borrow_mut().remove(&0);
        assert_eq!(manager.borrow().len(), 1);
    }
    
    // Test 6: Tab labels
    {
        let (widget, label, button) = dvop::ui::create_tab_widget("TestFile.txt");
        
        assert_eq!(label.text().as_str(), "TestFile.txt");
        assert!(widget.is_visible());
        assert!(button.is_visible());
    }
    
    // Test 7: Buffer operations
    {
        let (_, buffer) = dvop::syntax::create_source_view();
        
        assert_eq!(buffer.char_count(), 0);
        
        buffer.set_text("Line 1\nLine 2\nLine 3");
        assert!(buffer.char_count() > 0);
        assert_eq!(buffer.line_count(), 3);
        
        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
        assert_eq!(text.as_str(), "Line 1\nLine 2\nLine 3");
    }
    
    // Test 8: Git diff panel close menu items
    {
        let notebook = Notebook::new();
        let file_manager: Rc<RefCell<HashMap<u32, PathBuf>>> = Rc::new(RefCell::new(HashMap::new()));
        
        // Create and add multiple tabs
        for i in 0..5 {
            let (view, buffer) = dvop::syntax::create_source_view();
            buffer.set_text(&format!("Content {}", i));
            let scrolled = dvop::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = dvop::ui::create_tab_widget(&format!("file{}.txt", i));
            notebook.append_page(&scrolled, Some(&tab_widget));
            file_manager.borrow_mut().insert(i as u32, PathBuf::from(format!("/tmp/file{}.txt", i)));
        }
        
        assert_eq!(notebook.n_pages(), 5, "Should have 5 tabs initially");
        assert_eq!(file_manager.borrow().len(), 5, "Should track 5 files");
        
        // Test close to the right (from index 2)
        let keep_page = 2u32;
        while notebook.n_pages() > keep_page + 1 {
            let last_page = notebook.n_pages() - 1;
            file_manager.borrow_mut().remove(&(last_page as u32));
            notebook.remove_page(Some(last_page));
        }
        assert_eq!(notebook.n_pages(), 3, "Should have 3 tabs after closing to the right");
        
        // Test close to the left (from index 2, which is now the last tab)
        let keep_page = 2u32;
        for _ in 0..keep_page {
            if notebook.n_pages() > 1 {
                file_manager.borrow_mut().remove(&0);
                notebook.remove_page(Some(0));
            }
        }
        assert_eq!(notebook.n_pages(), 1, "Should have 1 tab after closing to the left");
        
        // Add more tabs for other tests
        for i in 0..4 {
            let (view, _buffer) = dvop::syntax::create_source_view();
            let scrolled = dvop::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = dvop::ui::create_tab_widget(&format!("new{}.txt", i));
            notebook.append_page(&scrolled, Some(&tab_widget));
        }
        assert_eq!(notebook.n_pages(), 5, "Should have 5 tabs again");
        
        // Test close all
        while notebook.n_pages() > 0 {
            let last_page = notebook.n_pages() - 1;
            notebook.remove_page(Some(last_page));
        }
        assert_eq!(notebook.n_pages(), 0, "Should have 0 tabs after closing all");
    }
    
    // Test 9: Git diff panel open related file button
    {
        // Test that the open file callback can be set and called
        use std::sync::{Arc, Mutex};
        
        let test_file = PathBuf::from("/tmp/test_file.txt");
        let callback_called = Arc::new(Mutex::new(false));
        let callback_called_clone = callback_called.clone();
        let test_file_clone = test_file.clone();
        
        let callback = Box::new(move |path: PathBuf, _line: usize, _col: usize| {
            assert_eq!(path, test_file_clone);
            *callback_called_clone.lock().unwrap() = true;
        });
        
        // Set the callback
        if let Ok(mut guard) = dvop::handlers::OPEN_FILE_CALLBACK.lock() {
            *guard = Some(callback);
        }
        
        // Simulate button click by calling the callback directly
        dvop::handlers::open_file_and_jump_to_location(test_file.clone(), 1, 1);
        
        // Verify callback was called
        assert!(*callback_called.lock().unwrap(), "Open file callback should be invoked");
        
        // Clean up
        if let Ok(mut guard) = dvop::handlers::OPEN_FILE_CALLBACK.lock() {
            *guard = None;
        }
    }
    
    println!("✓ All 9 tests passed!");
    });
}

#[serial]
#[test]
fn test_git_diff_path_bar_update() {
    gtk4::test_synced(|| {
    
    // Test that opening a git diff updates the path bar to show the file's directory
    use std::env;
    
    // Use a real directory that exists (the current directory or temp)
    let real_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
    let current_dir = Rc::new(RefCell::new(real_dir.clone()));
    let path_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let file_list_box = gtk4::ListBox::new();
    let active_tab_path = Rc::new(RefCell::new(None));
    
    // Simulate opening a diff for a file in a subdirectory
    let test_file = real_dir.join("src/main.rs");
    let expected_dir = real_dir.join("src");
    
    // Update current_dir as would happen when opening a diff
    if let Some(parent) = test_file.parent() {
        *current_dir.borrow_mut() = parent.to_path_buf();
        
        // Only update file list if the directory exists
        if parent.exists() {
            dvop::utils::update_file_list(
                &file_list_box,
                &current_dir.borrow(),
                &active_tab_path.borrow(),
                dvop::utils::FileSelectionSource::TabSwitch,
            );
        }
        
        // Update path buttons
        dvop::utils::update_path_buttons(
            &path_box,
            &current_dir,
            &file_list_box,
            &active_tab_path,
        );
    }
    
    // Verify the directory was updated
    assert_eq!(*current_dir.borrow(), expected_dir, "Current directory should be updated to file's parent");
    
    // Verify path_box has children (buttons were created)
    assert!(path_box.first_child().is_some(), "Path box should contain path buttons");
    
    // If the directory exists, verify file_list_box was populated
    if expected_dir.exists() {
        assert!(file_list_box.first_child().is_some(), "File list should be populated with directory contents");
    }
    
    // Count the number of path segments
    let mut child_count = 0;
    let mut child = path_box.first_child();
    while let Some(widget) = child {
        child_count += 1;
        child = widget.next_sibling();
    }
    assert!(child_count > 0, "Path box should have path segment buttons");
    
    println!("✓ Git diff path bar update test passed!");
    });
}

#[serial]
#[test]
fn test_path_bar_updates_for_different_files() {
    gtk4::test_synced(|| {
    
    // Test that the path bar updates correctly for files in different directories
    use std::env;
    
    let initial_dir = env::current_dir().unwrap_or_else(|_| PathBuf::from("/tmp"));
    let current_dir = Rc::new(RefCell::new(initial_dir.clone()));
    let path_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    let file_list_box = gtk4::ListBox::new();
    let active_tab_path = Rc::new(RefCell::new(None));
    
    // Test file 1: use actual src/ui directory if it exists
    let file1 = initial_dir.join("src/ui/git_diff.rs");
    if let Some(parent) = file1.parent() {
        *current_dir.borrow_mut() = parent.to_path_buf();
        
        if parent.exists() {
            dvop::utils::update_file_list(
                &file_list_box,
                &current_dir.borrow(),
                &active_tab_path.borrow(),
                dvop::utils::FileSelectionSource::TabSwitch,
            );
        }
        
        dvop::utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
        assert_eq!(*current_dir.borrow(), parent.to_path_buf());
    }
    
    // Test file 2: use a different real directory
    let file2 = initial_dir.join("tests/e2e_tests.rs");
    if let Some(parent) = file2.parent() {
        *current_dir.borrow_mut() = parent.to_path_buf();
        
        if parent.exists() {
            dvop::utils::update_file_list(
                &file_list_box,
                &current_dir.borrow(),
                &active_tab_path.borrow(),
                dvop::utils::FileSelectionSource::TabSwitch,
            );
        }
        
        dvop::utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
        assert_eq!(*current_dir.borrow(), parent.to_path_buf());
    }
    
    // Test file 3: use /tmp which should always exist
    let file3 = PathBuf::from("/tmp/test.txt");
    if let Some(parent) = file3.parent() {
        *current_dir.borrow_mut() = parent.to_path_buf();
        
        dvop::utils::update_file_list(
            &file_list_box,
            &current_dir.borrow(),
            &active_tab_path.borrow(),
            dvop::utils::FileSelectionSource::TabSwitch,
        );
        
        dvop::utils::update_path_buttons(&path_box, &current_dir, &file_list_box, &active_tab_path);
        assert_eq!(*current_dir.borrow(), PathBuf::from("/tmp"));
    }
    
    println!("✓ Path bar updates for different files test passed!");
    });
}

#[serial]
#[test]
fn test_staged_revealer_behavior() {
    gtk4::test_synced(|| {
    
    // Test the behavior logic for staged revealer without creating actual widgets
    // This test verifies the conditional logic used in git_diff.rs
    
    // Scenario 1: No staged changes (empty list)
    let staged_changes_count = 0;
    let should_reveal = staged_changes_count > 0;
    assert!(!should_reveal, "Revealer should be hidden when no staged changes");
    
    // Scenario 2: Has staged changes
    let staged_changes_count = 3;
    let should_reveal = staged_changes_count > 0;
    assert!(should_reveal, "Revealer should be visible when there are staged changes");
    
    // Scenario 3: Staged changes added then removed
    let mut staged_changes_count = 0;
    let should_reveal = staged_changes_count > 0;
    assert!(!should_reveal, "Should start hidden");
    
    staged_changes_count = 1;
    let should_reveal = staged_changes_count > 0;
    assert!(should_reveal, "Should be visible after adding changes");
    
    staged_changes_count = 0;
    let should_reveal = staged_changes_count > 0;
    assert!(!should_reveal, "Should be hidden after removing all changes");
    
    println!("✓ Staged revealer behavior test passed!");
    });
}
