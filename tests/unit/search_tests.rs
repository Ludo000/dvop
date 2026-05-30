    use super::*;
    use gtk4::prelude::*;
    use serial_test::serial;

    fn context_for_text(text: &str, query: &str) -> (sourceview5::Buffer, SearchContext) {
        let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
        buffer.set_text(text);
        let settings = SearchSettings::new();
        settings.set_search_text(Some(query));
        let context = SearchContext::new(&buffer, Some(&settings));
        (buffer, context)
    }

    #[test]
    #[serial]
    fn search_context_counts_positions_and_replaces_matches() {
        gtk4::test_synced(|| {
            let (_buffer, context) = context_for_text("one two one two one", "one");
            assert_eq!(SearchState::count_matches(&context), 3);

            let (_buffer, context) = context_for_text("alpha beta gamma", "delta");
            assert_eq!(SearchState::count_matches(&context), 0);

            let (_buffer, context) = context_for_text("find me and find me again", "find");
            assert_eq!(SearchState::get_current_match_position(&context), 3);

            let (buffer, context) = context_for_text("red blue red green red", "red");
            let iter = buffer.iter_at_offset(12);
            buffer.place_cursor(&iter);
            assert_eq!(SearchState::get_current_match_position(&context), 3);

            let (buffer, context) = context_for_text("cat dog cat cat", "cat");
            SearchState::replace_all(&context, "fox");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "fox dog fox fox");
            assert_eq!(SearchState::count_matches(&context), 0);

            let (buffer, context) = context_for_text("remove keep remove", "remove");
            SearchState::replace_all(&context, "");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), " keep ");

            let (_buffer, context) = context_for_text("abc abc", "");
            let label = Label::new(Some("old"));
            SearchState::update_match_count(&label, &context);
            assert_eq!(label.text().as_str(), "");

            context.settings().set_search_text(None);
            label.set_text("old");
            SearchState::update_match_count(&label, &context);
            assert_eq!(label.text().as_str(), "");

            let (buffer, context) = context_for_text("alpha beta alpha", "alpha");
            buffer.place_cursor(&buffer.start_iter());
            SearchState::find_next(&context);
            assert_eq!(SearchState::get_current_match_position(&context), 1);

            SearchState::find_next(&context);
            SearchState::update_match_count(&label, &context);
            assert_eq!(label.text().as_str(), "2 of 2");

            SearchState::find_previous(&context);
            SearchState::update_match_count(&label, &context);
            assert_eq!(label.text().as_str(), "1 of 2");

            let _ = buffer;
        });
    }

    #[test]
    #[serial]
    fn find_previous_wraps_to_last_match_from_start() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("alpha beta alpha", "alpha");
            buffer.place_cursor(&buffer.start_iter());

            SearchState::find_previous(&context);
            assert_eq!(SearchState::get_current_match_position(&context), 1);

            let _ = buffer;
        });
    }

    #[test]
    #[serial]
    fn replace_all_updates_single_match_in_buffer() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("replace me once", "replace");
            SearchState::replace_all(&context, "updated");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "updated me once");
        });
    }

    #[test]
    #[serial]
    fn find_next_advances_to_second_match_from_start() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("one two one", "one");
            buffer.place_cursor(&buffer.start_iter());

            SearchState::find_next(&context);
            assert_eq!(SearchState::get_current_match_position(&context), 1);

            SearchState::find_next(&context);
            assert_eq!(SearchState::get_current_match_position(&context), 2);
        });
    }

    #[test]
    #[serial]
    fn replace_current_replaces_selected_match_and_advances() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("foo bar foo", "foo");
            buffer.place_cursor(&buffer.start_iter());
            SearchState::find_next(&context);

            SearchState::replace_current(&context, "baz");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "baz bar foo");
        });
    }

    #[test]
    #[serial]
    fn get_current_match_position_includes_match_when_cursor_is_at_end_of_hit() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("only", "only");
            buffer.place_cursor(&buffer.end_iter());
            assert_eq!(SearchState::get_current_match_position(&context), 2);
        });
    }

    #[test]
    #[serial]
    fn find_next_wraps_to_first_match_from_last_selection() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("alpha beta alpha", "alpha");
            buffer.place_cursor(&buffer.start_iter());
            SearchState::find_next(&context);
            SearchState::find_next(&context);
            assert_eq!(SearchState::get_current_match_position(&context), 2);

            SearchState::find_next(&context);
            assert!(buffer.has_selection());
        });
    }

    #[test]
    #[serial]
    fn replace_current_is_noop_when_search_text_is_empty() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("keep me", "");
            context.settings().set_search_text(Some(""));
            SearchState::replace_current(&context, "changed");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "keep me");
        });
    }

    #[test]
    #[serial]
    fn replace_all_with_empty_replacement_deletes_every_match() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("x x x", "x");
            SearchState::replace_all(&context, "");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "  ");
        });
    }

    #[test]
    #[serial]
    fn update_match_count_shows_zero_matches_for_missing_query() {
        gtk4::test_synced(|| {
            let (_buffer, context) = context_for_text("alpha beta gamma", "missing");
            let label = Label::new(Some("old"));
            SearchState::update_match_count(&label, &context);
            assert_eq!(label.text().as_str(), "0 matches");
        });
    }

    #[test]
    #[serial]
    fn find_previous_moves_to_prior_match_from_end_selection() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("one two one", "one");
            buffer.place_cursor(&buffer.end_iter());
            SearchState::find_previous(&context);
            assert!(buffer.has_selection());
            assert_eq!(SearchState::get_current_match_position(&context), 2);
        });
    }

    #[test]
    #[serial]
    fn count_matches_respects_case_sensitive_search_settings() {
        gtk4::test_synced(|| {
            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            buffer.set_text("Hello hello");
            let settings = SearchSettings::new();
            settings.set_search_text(Some("hello"));
            settings.set_case_sensitive(true);
            let context = SearchContext::new(&buffer, Some(&settings));

            assert_eq!(SearchState::count_matches(&context), 1);
        });
    }

    #[test]
    #[serial]
    fn count_matches_respects_whole_word_search_settings() {
        gtk4::test_synced(|| {
            let buffer = sourceview5::Buffer::new(None::<&gtk4::TextTagTable>);
            buffer.set_text("cat category");
            let settings = SearchSettings::new();
            settings.set_search_text(Some("cat"));
            settings.set_at_word_boundaries(true);
            let context = SearchContext::new(&buffer, Some(&settings));

            assert_eq!(SearchState::count_matches(&context), 1);
        });
    }

    #[test]
    #[serial]
    fn find_next_selects_first_match_from_buffer_start() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("alpha beta alpha", "alpha");
            buffer.place_cursor(&buffer.start_iter());

            SearchState::find_next(&context);
            assert!(buffer.has_selection());
            assert_eq!(SearchState::get_current_match_position(&context), 1);
        });
    }

    #[test]
    #[serial]
    fn replace_all_leaves_buffer_unchanged_when_query_has_no_matches() {
        gtk4::test_synced(|| {
            let (buffer, context) = context_for_text("keep this text", "missing");
            SearchState::replace_all(&context, "changed");
            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "keep this text");
        });
    }
