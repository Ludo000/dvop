// Quick integration tests for Dvop
// Run with: cargo test --test quick_tests

use gtk4::prelude::*;
use gtk4::Notebook;
use sourceview5::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

#[test]
fn test_all_features() {
    gtk4::init().expect("Failed to initialize GTK");
    
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
            let (view, _) = dvop::syntax::create_source_view();
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
    
    println!("✓ All 7 tests passed!");
}
