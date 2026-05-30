    use super::*;
    use gtk4::prelude::*;
    use serial_test::serial;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    #[test]
    fn bulk_session_restore_flag_tracks_state() {
        set_bulk_session_restore(false);
        assert!(!bulk_session_restore_active());

        set_bulk_session_restore(true);
        assert!(bulk_session_restore_active());

        set_bulk_session_restore(false);
        assert!(!bulk_session_restore_active());
    }

    #[test]
    fn bulk_session_restore_defaults_to_inactive() {
        set_bulk_session_restore(false);
        assert!(!bulk_session_restore_active());
    }

    #[test]
    fn open_file_callback_invokes_registered_handler() {
        use std::sync::{Arc, Mutex};

        let captured = Arc::new(Mutex::new(None::<(PathBuf, usize, usize)>));
        let captured_for_callback = captured.clone();

        if let Ok(mut guard) = OPEN_FILE_CALLBACK.lock() {
            *guard = Some(Box::new(move |path, line, column| {
                *captured_for_callback.lock().unwrap() = Some((path, line, column));
            }));
        }

        let target = PathBuf::from("/tmp/dvop-open-callback.rs");
        open_file_and_jump_to_location(target.clone(), 12, 4);

        let (path, line, column) = captured
            .lock()
            .unwrap()
            .clone()
            .expect("callback should run");
        assert_eq!(path, target);
        assert_eq!(line, 12);
        assert_eq!(column, 4);

        if let Ok(mut guard) = OPEN_FILE_CALLBACK.lock() {
            *guard = None;
        }
    }

    #[test]
    #[serial]
    fn jump_to_line_and_column_moves_cursor_to_requested_position() {
        gtk4::test_synced(|| {
            let (view, _buffer) = crate::syntax::create_source_view();
            let buffer = view.buffer();
            buffer.set_text("line one\nline two\nline three\n");

            jump_to_line_and_column(&view, 2, 3);

            let cursor = buffer.cursor_position();
            let mut iter = buffer.iter_at_offset(cursor);
            assert_eq!(iter.line(), 1);
            assert_eq!(iter.line_offset(), 2);
        });
    }

    #[test]
    #[serial]
    fn jump_to_line_and_column_clamps_zero_line_to_start() {
        gtk4::test_synced(|| {
            let (view, _buffer) = crate::syntax::create_source_view();
            let buffer = view.buffer();
            buffer.set_text("alpha\nbeta\n");

            jump_to_line_and_column(&view, 0, 0);

            let cursor = buffer.cursor_position();
            let iter = buffer.iter_at_offset(cursor);
            assert_eq!(iter.line(), 0);
            assert_eq!(iter.line_offset(), 0);
        });
    }

    #[test]
    fn open_file_and_jump_without_callback_does_not_panic() {
        open_file_and_jump_to_location(PathBuf::from("/tmp/no-callback.rs"), 1, 1);
    }

    #[test]
    fn bulk_session_restore_can_be_toggled_multiple_times() {
        set_bulk_session_restore(true);
        assert!(bulk_session_restore_active());
        set_bulk_session_restore(false);
        assert!(!bulk_session_restore_active());
        set_bulk_session_restore(true);
        assert!(bulk_session_restore_active());
        set_bulk_session_restore(false);
    }

    #[test]
    #[serial]
    fn jump_to_line_and_column_honors_unicode_columns() {
        gtk4::test_synced(|| {
            let (view, _buffer) = crate::syntax::create_source_view();
            let buffer = view.buffer();
            buffer.set_text("αβγ\n");

            jump_to_line_and_column(&view, 1, 3);
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            assert_eq!(cursor.line_offset(), 2);
        });
    }

    #[test]
    #[serial]
    fn jump_to_line_and_column_clamps_line_past_eof() {
        gtk4::test_synced(|| {
            let (view, _buffer) = crate::syntax::create_source_view();
            let buffer = view.buffer();
            buffer.set_text("only line\n");

            jump_to_line_and_column(&view, 99, 1);
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            assert_eq!(cursor.line(), 0);
        });
    }

    #[test]
    #[serial]
    fn jump_to_line_and_column_clamps_column_on_short_lines() {
        gtk4::test_synced(|| {
            let (view, _buffer) = crate::syntax::create_source_view();
            let buffer = view.buffer();
            buffer.set_text("hi");

            jump_to_line_and_column(&view, 1, 2);
            let cursor = buffer.iter_at_mark(&buffer.get_insert());
            assert_eq!(cursor.line_offset(), 1);
        });
    }

    #[test]
    #[serial]
    fn close_empty_untitled_tabs_removes_blank_untitled_tab() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = crate::ui::create_tab_widget("Untitled");
            notebook.append_page(&scrolled, Some(&tab_widget));

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            close_empty_untitled_tabs(&notebook, &file_path_manager);

            assert_eq!(notebook.n_pages(), 0);
        });
    }

    #[test]
    #[serial]
    fn close_empty_untitled_tabs_keeps_untitled_tab_with_content() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("not empty");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = crate::ui::create_tab_widget("Untitled");
            notebook.append_page(&scrolled, Some(&tab_widget));

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            close_empty_untitled_tabs(&notebook, &file_path_manager);

            assert_eq!(notebook.n_pages(), 1);
        });
    }

    #[test]
    #[serial]
    fn get_text_view_and_buffer_for_page_returns_source_tab_widgets() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("page content");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            notebook.append_page(&scrolled, None::<&gtk4::Widget>);

            let (text_view, text_buffer) =
                get_text_view_and_buffer_for_page(&notebook, 0).expect("page should expose buffer");
            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
            assert_eq!(text.as_str(), "page content");
            let active_buffer = text_view.buffer();
            let active_text = active_buffer.text(
                &active_buffer.start_iter(),
                &active_buffer.end_iter(),
                false,
            );
            assert_eq!(active_text.as_str(), "page content");
        });
    }

    #[test]
    #[serial]
    fn get_active_text_view_and_buffer_returns_current_tab() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("active tab");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            let page = notebook.append_page(&scrolled, None::<&gtk4::Widget>);
            notebook.set_current_page(Some(page));

            let (text_view, text_buffer) =
                get_active_text_view_and_buffer(&notebook).expect("active tab should expose buffer");
            let text = text_buffer.text(&text_buffer.start_iter(), &text_buffer.end_iter(), false);
            assert_eq!(text.as_str(), "active tab");
            let active_buffer = text_view.buffer();
            let active_text = active_buffer.text(
                &active_buffer.start_iter(),
                &active_buffer.end_iter(),
                false,
            );
            assert_eq!(active_text.as_str(), "active tab");
        });
    }

    #[test]
    #[serial]
    fn close_empty_untitled_tabs_removes_star_untitled_with_empty_buffer() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            let (tab_widget, _, _) = crate::ui::create_tab_widget("*Untitled");
            notebook.append_page(&scrolled, Some(&tab_widget));

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            close_empty_untitled_tabs(&notebook, &file_path_manager);

            assert_eq!(notebook.n_pages(), 0);
        });
    }
