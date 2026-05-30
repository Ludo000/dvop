    use super::*;
    use gtk4::prelude::*;
    use serial_test::serial;

    #[test]
    fn test_get_preferred_style_scheme() {
        let scheme = get_preferred_style_scheme();
        assert!(!scheme.is_empty());
    }

    #[test]
    fn test_large_file_threshold_is_ten_megabytes() {
        assert_eq!(LARGE_FILE_THRESHOLD, 10_485_760);
    }

    #[test]
    fn test_is_dark_mode_enabled_returns_boolean_without_panicking() {
        let _ = is_dark_mode_enabled();
    }

    #[test]
    fn test_preferred_style_scheme_matches_dark_mode_flag() {
        let scheme = get_preferred_style_scheme().to_lowercase();
        let dark = is_dark_mode_enabled();

        if dark {
            assert!(
                scheme.contains("dark"),
                "expected dark scheme, got {scheme}"
            );
        } else {
            assert!(
                !scheme.contains("dark") || scheme.contains("classic"),
                "expected light scheme, got {scheme}"
            );
        }
    }

    #[test]
    #[serial]
    fn set_language_for_file_assigns_rust_and_python_grammars() {
        gtk4::test_synced(|| {
            let (view, buffer) = create_source_view();
            assert!(set_language_for_file(&buffer, std::path::Path::new("main.rs")));
            assert!(buffer.language().is_some());

            assert!(set_language_for_file(&buffer, std::path::Path::new("script.py")));
            let lang_id = buffer
                .language()
                .map(|l| l.id().to_string())
                .unwrap_or_default();
            assert!(lang_id.starts_with("python"));

            let _ = view;
        });
    }

    #[test]
    #[serial]
    fn create_source_view_for_large_file_disables_expensive_features() {
        gtk4::test_synced(|| {
            let (view, _buffer) = create_source_view_for_large_file();
            assert!(!view.is_highlight_current_line());
            assert!(!view.is_auto_indent());
        });
    }

    #[test]
    #[serial]
    fn set_language_for_file_assigns_json_toml_and_ui_grammars() {
        gtk4::test_synced(|| {
            let (_view, buffer) = create_source_view();

            assert!(set_language_for_file(&buffer, std::path::Path::new("data.json")));
            assert!(set_language_for_file(&buffer, std::path::Path::new("Cargo.toml")));
            assert!(set_language_for_file(
                &buffer,
                std::path::Path::new("widget.ui")
            ));
            assert!(!set_language_for_file(
                &buffer,
                std::path::Path::new("unknown.xyz123")
            ));
        });
    }

    #[test]
    #[serial]
    fn set_language_for_file_assigns_markdown_and_go_grammars() {
        gtk4::test_synced(|| {
            let (_view, buffer) = create_source_view();
            assert!(set_language_for_file(&buffer, std::path::Path::new("README.md")));
            assert!(set_language_for_file(&buffer, std::path::Path::new("main.go")));
        });
    }

    #[test]
    #[serial]
    fn create_source_view_scrolled_wraps_source_view_in_scrolled_window() {
        gtk4::test_synced(|| {
            let (view, _buffer) = create_source_view();
            let scrolled = create_source_view_scrolled(&view);

            assert!(scrolled.child().is_some());
            let (h_policy, v_policy) = scrolled.policy();
            assert_eq!(h_policy, gtk4::PolicyType::Automatic);
            assert_eq!(v_policy, gtk4::PolicyType::Automatic);
        });
    }

    #[test]
    #[serial]
    fn increase_and_decrease_font_size_restore_original_setting() {
        gtk4::test_synced(|| {
            let before = crate::settings::get_settings().get_font_size();
            if before < 72 {
                increase_font_size();
                assert_eq!(crate::settings::get_settings().get_font_size(), before + 1);
            }
            if before > 6 {
                decrease_font_size();
                assert_eq!(crate::settings::get_settings().get_font_size(), before);
            }
        });
    }
