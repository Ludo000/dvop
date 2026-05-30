    use super::*;
    use serial_test::serial;
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::path::Path;
    use std::rc::Rc;

    #[test]
    fn search_in_content_finds_case_insensitive_matches() {
        let path = Path::new("/tmp/search.rs");
        let results = search_in_content(
            path,
            "Hello World\nhello again",
            "hello",
            false,
            false,
        );
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].line, 1);
        assert_eq!(results[1].line, 2);
    }

    #[test]
    fn search_in_content_respects_case_sensitive_mode() {
        let path = Path::new("/tmp/search.rs");
        let results = search_in_content(path, "Hello hello", "hello", true, false);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 1);
    }

    #[test]
    fn search_in_content_respects_whole_word_option() {
        let path = Path::new("/tmp/search.rs");
        let results = search_in_content(path, "cat category", "cat", true, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].col, 1);
    }

    #[test]
    fn search_in_content_returns_empty_for_empty_query() {
        let path = Path::new("/tmp/search.rs");
        let results = search_in_content(path, "some text", "", true, false);
        assert!(results.is_empty());
    }

    #[test]
    fn search_file_reads_matches_from_disk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("sample.rs");
        std::fs::write(&file_path, "fn main() {\n    println!(\"find me\");\n}\n").unwrap();

        let results = search_file(&file_path, "find me", true, false, 1024 * 1024);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 2);
        assert!(results[0].preview.contains("find me"));
    }

    #[test]
    fn search_file_skips_binary_files() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("photo.png");
        std::fs::write(&file_path, b"\x89PNG\r\n\x1a\n").unwrap();

        let results = search_file(&file_path, "PNG", true, false, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn is_text_file_allows_rust_sources() {
        assert!(is_text_file(Path::new("main.rs")));
        assert!(!is_text_file(Path::new("image.png")));
    }

    #[test]
    fn search_file_respects_whole_word_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("words.txt");
        std::fs::write(&file_path, "cat category\n").unwrap();

        let results = search_file(&file_path, "cat", true, true, 1024 * 1024);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].col, 1);
    }

    #[test]
    fn search_file_skips_files_larger_than_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("large.txt");
        std::fs::write(&file_path, "needle in haystack").unwrap();

        let results = search_file(&file_path, "needle", true, false, 4);
        assert!(results.is_empty());
    }

    #[test]
    fn search_in_content_finds_multiple_matches_on_one_line() {
        let path = Path::new("/tmp/search.rs");
        let results = search_in_content(path, "foo foo foo", "foo", true, false);
        assert_eq!(results.len(), 3);
        assert!(results.iter().all(|r| r.line == 1));
    }

    #[test]
    fn search_file_finds_case_insensitive_matches_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("mixed.txt");
        std::fs::write(&file_path, "Hello HELLO hello\n").unwrap();

        let results = search_file(&file_path, "hello", false, false, 1024 * 1024);
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn search_file_returns_empty_for_empty_needle() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("empty-query.txt");
        std::fs::write(&file_path, "content").unwrap();

        let results = search_file(&file_path, "", true, false, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn walk_dir_recursive_skips_hidden_target_and_node_modules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("visible.txt"), "ok").unwrap();

        std::fs::create_dir_all(dir.path().join(".git")).unwrap();
        std::fs::write(dir.path().join(".git/secret"), "hidden").unwrap();

        let target = dir.path().join("target");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("artifact"), "skip").unwrap();

        let node_modules = dir.path().join("node_modules");
        std::fs::create_dir_all(&node_modules).unwrap();
        std::fs::write(node_modules.join("pkg.js"), "skip").unwrap();

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 100);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("visible.txt"));
    }

    #[test]
    fn search_file_finds_multiline_needle_in_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("multiline.txt");
        std::fs::write(&file_path, "begin\nmiddle\nline\nend\n").unwrap();

        let results = search_file(&file_path, "middle\nline", true, false, 1024 * 1024);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn walk_dir_recursive_stops_at_max_files_limit() {
        let dir = tempfile::tempdir().unwrap();
        for index in 0..5 {
            std::fs::write(dir.path().join(format!("file{index}.txt")), "x").unwrap();
        }

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 3);
        assert_eq!(files.len(), 3);
    }

    #[test]
    fn walk_dir_recursive_collects_nested_text_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src/deep")).unwrap();
        std::fs::write(dir.path().join("root.txt"), "root").unwrap();
        std::fs::write(dir.path().join("src/mod.rs"), "mod").unwrap();
        std::fs::write(dir.path().join("src/deep/lib.rs"), "lib").unwrap();

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 100);

        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("root.txt")));
        assert!(files.iter().any(|p| p.ends_with("mod.rs")));
        assert!(files.iter().any(|p| p.ends_with("lib.rs")));
    }

    #[test]
    fn search_in_content_reports_one_based_column_for_second_match() {
        let path = Path::new("/tmp/columns.txt");
        let results = search_in_content(path, "ab ab ab", "ab", true, false);
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].col, 1);
        assert_eq!(results[1].col, 4);
        assert_eq!(results[2].col, 7);
    }

    #[test]
    fn search_file_returns_empty_for_directory_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("child.txt"), "needle").unwrap();

        let results = search_file(dir.path(), "needle", true, false, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn walk_dir_recursive_skips_symlink_loops_and_hidden_files() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("visible.txt"), "ok").unwrap();
        std::fs::write(dir.path().join(".hidden.txt"), "skip").unwrap();

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 100);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("visible.txt"));
    }

    #[test]
    fn search_file_returns_empty_for_missing_path() {
        let missing = Path::new("/tmp/dvop/does-not-exist-search.txt");
        let results = search_file(missing, "needle", true, false, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn search_in_content_whole_word_does_not_match_prefixes() {
        let path = Path::new("/tmp/prefix.txt");
        let results = search_in_content(path, "testing tester", "test", true, true);
        assert!(results.is_empty());
    }

    #[test]
    fn search_in_content_case_sensitive_skips_wrong_case() {
        let path = Path::new("/tmp/case.txt");
        let results = search_in_content(path, "Hello world", "hello", true, false);
        assert!(results.is_empty());
    }

    #[test]
    fn search_file_whole_word_does_not_match_substrings_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("category.txt");
        std::fs::write(&file_path, "category catalog\n").unwrap();

        let results = search_file(&file_path, "cat", true, true, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn walk_dir_recursive_skips_empty_directories() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("empty/nested")).unwrap();
        std::fs::write(dir.path().join("top.txt"), "ok").unwrap();

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 100);

        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("top.txt"));
    }

    #[test]
    fn search_in_content_includes_needle_and_preview_in_results() {
        let path = Path::new("/tmp/preview.txt");
        let results = search_in_content(path, "find me here", "find", true, false);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].needle, "find");
        assert_eq!(results[0].preview, "find me here");
    }

    #[test]
    fn is_text_file_allows_markdown_and_json_extensions() {
        assert!(is_text_file(Path::new("README.md")));
        assert!(is_text_file(Path::new("package.json")));
    }

    #[test]
    fn search_in_content_uses_unicode_char_columns() {
        let path = Path::new("/tmp/unicode.txt");
        let results = search_in_content(path, "αβ find", "find", true, false);

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].col, 4);
    }

    #[test]
    fn search_in_content_whole_word_skips_ascii_substrings_in_memory() {
        let path = Path::new("/tmp/whole.txt");
        let results = search_in_content(path, "category catalog", "cat", true, true);
        assert!(results.is_empty());
    }

    #[test]
    fn search_in_content_whole_word_allows_underscore_prefixed_matches() {
        let path = Path::new("/tmp/underscore.txt");
        let results = search_in_content(path, "foo_bar baz", "bar", true, true);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].col, 5);
    }

    #[test]
    fn is_text_file_rejects_audio_and_video_extensions() {
        assert!(!is_text_file(Path::new("song.mp3")));
        assert!(!is_text_file(Path::new("clip.mp4")));
    }

    #[test]
    fn is_text_file_allows_toml_and_yaml_sources() {
        assert!(is_text_file(Path::new("Cargo.toml")));
        assert!(is_text_file(Path::new("config.yaml")));
    }

    #[test]
    fn search_file_finds_case_insensitive_multiline_needle() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("folded.txt");
        std::fs::write(&file_path, "BEGIN\nMiDdLe\nLiNe\nEND\n").unwrap();

        let results = search_file(&file_path, "middle\nline", false, false, 1024 * 1024);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].line, 2);
    }

    #[test]
    fn search_file_whole_word_treats_underscore_as_word_character_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("underscore.txt");
        std::fs::write(&file_path, "foo_bar baz\n").unwrap();

        let results = search_file(&file_path, "bar", true, true, 1024 * 1024);
        assert!(results.is_empty());
    }

    #[test]
    fn search_file_searches_file_at_exact_size_limit() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("limit.txt");
        std::fs::write(&file_path, "needle here").unwrap();
        let size = std::fs::metadata(&file_path).unwrap().len();

        let results = search_file(&file_path, "needle", true, false, size);
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn search_file_skips_files_larger_than_max_size() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("big.txt");
        std::fs::write(&file_path, "needle in a big file").unwrap();

        let results = search_file(&file_path, "needle", true, false, 4);
        assert!(results.is_empty());
    }

    #[test]
    fn walk_dir_recursive_collects_non_text_files_alongside_text() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("notes.txt"), "text").unwrap();
        std::fs::write(dir.path().join("photo.png"), "png").unwrap();

        let mut files = Vec::new();
        walk_dir_recursive(dir.path(), &mut files, 100);

        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("photo.png")));
    }

    #[test]
    #[serial]
    fn replace_in_buffer_replaces_needle_in_open_source_tab() {
        gtk4::test_synced(|| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("open.rs");
            std::fs::write(&path, "hello world\n").unwrap();

            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("hello world\n");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            notebook.append_page(&scrolled, None::<&gtk4::Widget>);

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            file_path_manager.borrow_mut().insert(0, path.clone());

            replace_in_buffer(
                &notebook,
                &file_path_manager,
                &path,
                1,
                1,
                "hello",
                "goodbye",
                true,
            )
            .expect("replace should succeed");

            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "goodbye world\n");
        });
    }

    #[test]
    #[serial]
    fn replace_in_buffer_errors_when_file_is_not_open() {
        gtk4::test_synced(|| {
            let notebook = gtk4::Notebook::new();
            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            let path = Path::new("/tmp/dvop-not-open.rs");

            let err = replace_in_buffer(
                &notebook,
                &file_path_manager,
                path,
                1,
                1,
                "hello",
                "goodbye",
                true,
            )
            .unwrap_err();

            assert!(err.contains("File not open"));
        });
    }

    #[test]
    #[serial]
    fn replace_in_buffer_supports_case_insensitive_matches() {
        gtk4::test_synced(|| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("fold.rs");
            std::fs::write(&path, "Hello\n").unwrap();

            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("Hello\n");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            notebook.append_page(&scrolled, None::<&gtk4::Widget>);

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            file_path_manager.borrow_mut().insert(0, path.clone());

            replace_in_buffer(
                &notebook,
                &file_path_manager,
                &path,
                1,
                1,
                "hello",
                "Hi",
                false,
            )
            .expect("case-insensitive replace should succeed");

            let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), false);
            assert_eq!(text.as_str(), "Hi\n");
        });
    }

    #[test]
    #[serial]
    fn replace_in_buffer_errors_when_column_is_out_of_range() {
        gtk4::test_synced(|| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("cols.rs");
            std::fs::write(&path, "short\n").unwrap();

            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("short\n");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            notebook.append_page(&scrolled, None::<&gtk4::Widget>);

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            file_path_manager.borrow_mut().insert(0, path.clone());

            let err = replace_in_buffer(
                &notebook,
                &file_path_manager,
                &path,
                1,
                99,
                "short",
                "long",
                true,
            )
            .unwrap_err();
            assert!(err.contains("beyond line length"));
        });
    }

    #[test]
    #[serial]
    fn replace_in_buffer_errors_when_needle_does_not_match_at_column() {
        gtk4::test_synced(|| {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("mismatch.rs");
            std::fs::write(&path, "alpha beta\n").unwrap();

            let notebook = gtk4::Notebook::new();
            let (view, buffer) = crate::syntax::create_source_view();
            buffer.set_text("alpha beta\n");
            let scrolled = crate::syntax::create_source_view_scrolled(&view);
            notebook.append_page(&scrolled, None::<&gtk4::Widget>);

            let file_path_manager = Rc::new(RefCell::new(HashMap::new()));
            file_path_manager.borrow_mut().insert(0, path.clone());

            let err = replace_in_buffer(
                &notebook,
                &file_path_manager,
                &path,
                1,
                1,
                "beta",
                "gamma",
                true,
            )
            .unwrap_err();
            assert!(err.contains("not found at exact position"));
        });
    }
