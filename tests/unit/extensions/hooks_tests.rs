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
