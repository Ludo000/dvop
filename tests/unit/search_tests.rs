    use super::*;
    use serial_test::serial;
    use std::sync::Once;

    static GTK_INIT: Once = Once::new();

    fn ensure_gtk_initialized() {
        GTK_INIT.call_once(|| {
            gtk4::init().expect("GTK should initialize for search tests");
        });
    }

    fn context_for_text(text: &str, query: &str) -> (sourceview5::Buffer, SearchContext) {
        ensure_gtk_initialized();
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
    }
