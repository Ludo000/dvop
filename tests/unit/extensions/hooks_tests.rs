    use super::*;

    #[test]
    fn key_string_to_gtk_accel_converts_common_modifiers() {
        assert_eq!(key_string_to_gtk_accel("Ctrl+Shift+L"), "<Control><Shift>L");
        assert_eq!(key_string_to_gtk_accel("Control+Alt+P"), "<Control><Alt>P");
        assert_eq!(key_string_to_gtk_accel("Super+Meta+K"), "<Super><Super>K");
        assert_eq!(key_string_to_gtk_accel(" F5 "), "F5");
    }

    #[test]
    fn sanitize_action_name_replaces_non_alphanumeric_characters() {
        assert_eq!(sanitize_action_name("Run Command"), "run-command");
        assert_eq!(sanitize_action_name("Format: Selection!"), "format--selection-");
        assert_eq!(sanitize_action_name("Already-OK_123"), "already-ok-123");
    }

    #[test]
    fn active_file_path_tracking_matches_exact_path() {
        let path = PathBuf::from("/tmp/dvop-hooks-test.rs");

        set_active_file_path(Some(path.clone()));
        assert!(active_file_path_is(&path));
        assert!(!active_file_path_is(Path::new("/tmp/other.rs")));

        set_active_file_path(None);
        assert!(!active_file_path_is(&path));
    }

    #[test]
    fn key_string_to_gtk_accel_handles_arrow_and_function_keys() {
        assert_eq!(key_string_to_gtk_accel("Ctrl+Return"), "<Control>Return");
        assert_eq!(key_string_to_gtk_accel("Alt+Left"), "<Alt>Left");
        assert_eq!(key_string_to_gtk_accel("Shift+F12"), "<Shift>F12");
    }

    #[test]
    fn sanitize_action_name_collapses_non_alphanumeric_runs_to_dashes() {
        assert_eq!(sanitize_action_name("  Hello   World  "), "--hello---world--");
        assert_eq!(sanitize_action_name("!!!"), "---");
    }

    #[test]
    fn key_string_to_gtk_accel_normalizes_mixed_modifier_names() {
        assert_eq!(key_string_to_gtk_accel("ctrl+shift+s"), "<Control><Shift>s");
        assert_eq!(key_string_to_gtk_accel("Alt+PageDown"), "<Alt>PageDown");
    }

    #[test]
    fn key_string_to_gtk_accel_handles_single_character_keys() {
        assert_eq!(key_string_to_gtk_accel("a"), "a");
        assert_eq!(key_string_to_gtk_accel("Escape"), "Escape");
    }

    #[test]
    fn key_string_to_gtk_accel_handles_tab_and_space_keys() {
        assert_eq!(key_string_to_gtk_accel("Tab"), "Tab");
        assert_eq!(key_string_to_gtk_accel("Ctrl+Space"), "<Control>Space");
    }

    #[test]
    fn run_extension_linters_returns_empty_for_unmatched_extension() {
        let diagnostics = run_extension_linters(Path::new("/tmp/example.txt"));
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn active_file_path_is_false_after_clearing_active_path() {
        let path = PathBuf::from("/tmp/dvop-active.rs");
        set_active_file_path(Some(path));
        assert!(active_file_path_is(Path::new("/tmp/dvop-active.rs")));

        set_active_file_path(None);
        assert!(!active_file_path_is(Path::new("/tmp/dvop-active.rs")));
    }

    #[test]
    fn get_sidebar_panel_content_returns_empty_for_unknown_panel() {
        assert!(get_sidebar_panel_content("missing-ext", "panel", "refresh").is_empty());
    }

    #[test]
    fn sanitize_action_name_strips_non_ascii_letters() {
        assert_eq!(sanitize_action_name("Formaté Selection"), "formaté-selection");
    }

    #[test]
    fn fire_on_file_close_without_hooks_does_not_panic() {
        fire_on_file_close(Path::new("/tmp/dvop-closed.rs"));
    }

    #[test]
    fn fire_on_file_save_without_hooks_does_not_panic() {
        fire_on_file_save(Path::new("/tmp/dvop-saved.rs"));
    }

    #[test]
    fn fire_on_app_start_without_hooks_does_not_panic() {
        fire_on_app_start();
    }

    #[test]
    fn refresh_extension_without_match_does_not_panic() {
        refresh_extension("missing-extension-id", true);
        refresh_extension("missing-extension-id", false);
    }

    #[test]
    fn sanitize_action_name_strips_leading_and_trailing_dashes() {
        assert_eq!(sanitize_action_name("  Format Selection  "), "--format-selection--");
    }
