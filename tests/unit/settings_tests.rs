    use super::*;
    use serial_test::serial;
    use std::collections::HashMap;
    use tempfile::TempDir;

    fn settings_with_temp_path(temp_dir: &TempDir) -> EditorSettings {
        EditorSettings {
            values: HashMap::new(),
            config_path: temp_dir.path().join("settings.conf"),
        }
    }

    #[test]
    fn test_settings_creation() {
        let settings = EditorSettings::new();
        assert!(settings.get_font_size() >= 8);
        assert!(settings.get_window_width() > 0);
        assert!(settings.get_window_height() > 0);
    }

    #[test]
    fn test_font_size_validation() {
        let mut settings = EditorSettings::new();
        
        // Valid font size
        settings.set_font_size(14);
        assert_eq!(settings.get_font_size(), 14);
        
        // Can set any value
        settings.set_font_size(4);
        assert_eq!(settings.get_font_size(), 4);
        
        settings.set_font_size(100);
        assert_eq!(settings.get_font_size(), 100);
    }

    #[test]
    fn test_terminal_font_size() {
        let mut settings = EditorSettings::new();
        
        settings.set_terminal_font_size(12);
        assert_eq!(settings.get_terminal_font_size(), 12);
        
        settings.set_terminal_font_size(8);
        assert_eq!(settings.get_terminal_font_size(), 8);
    }

    #[test]
    fn test_audio_volume() {
        let mut settings = EditorSettings::new();
        
        settings.set_audio_volume(0.5);
        assert_eq!(settings.get_audio_volume(), 0.5);
        
        // Test clamping to 0.0-1.0 range
        settings.set_audio_volume(-0.5);
        assert_eq!(settings.get_audio_volume(), 0.0);
        
        settings.set_audio_volume(2.0);
        assert_eq!(settings.get_audio_volume(), 1.0);
    }

    #[test]
    fn test_window_dimensions() {
        let mut settings = EditorSettings::new();
        
        settings.set_window_size(1024, 768);
        assert_eq!(settings.get_window_width(), 1024);
        assert_eq!(settings.get_window_height(), 768);
        
        // Individual setters
        settings.set_window_width(1920);
        assert_eq!(settings.get_window_width(), 1920);
        
        settings.set_window_height(1080);
        assert_eq!(settings.get_window_height(), 1080);
    }

    #[test]
    fn test_pane_dimensions() {
        let mut settings = EditorSettings::new();
        
        settings.set_pane_dimensions(250, 200);
        assert_eq!(settings.get_file_panel_width(), 250);
        assert_eq!(settings.get_terminal_height(), 200);
        
        // Individual setters
        settings.set_file_panel_width(300);
        assert_eq!(settings.get_file_panel_width(), 300);
        
        settings.set_terminal_height(150);
        assert_eq!(settings.get_terminal_height(), 150);
    }

    #[test]
    fn test_theme_settings() {
        let mut settings = EditorSettings::new();
        
        settings.set_light_theme("solarized-light");
        assert_eq!(settings.get_light_theme(), "solarized-light");
        
        settings.set_dark_theme("solarized-dark");
        assert_eq!(settings.get_dark_theme(), "solarized-dark");
    }

    #[test]
    fn test_last_folder() {
        let mut settings = EditorSettings::new();
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let temp_dir = TempDir::new().unwrap();
        
        settings.set_last_folder(temp_dir.path());
        assert_eq!(settings.get_last_folder(), temp_dir.path());
    }

    #[test]
    fn test_opened_files_management() {
        let mut settings = EditorSettings::new();
        
        let file1 = PathBuf::from("/path/to/file1.rs");
        let file2 = PathBuf::from("/path/to/file2.rs");
        
        settings.set_opened_files(&[file1.clone(), file2.clone()]);
        
        let opened = settings.get_opened_files();
        assert_eq!(opened.len(), 2);
        assert!(opened.contains(&file1));
        assert!(opened.contains(&file2));
        
        // Remove one file
        settings.set_opened_files(&[file2.clone()]);
        let opened = settings.get_opened_files();
        assert_eq!(opened.len(), 1);
        assert!(!opened.contains(&file1));
        assert!(opened.contains(&file2));
    }

    #[test]
    fn test_empty_opened_files_returns_empty_list() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set_opened_files(&[]);
        assert!(settings.get_opened_files().is_empty());
    }

    #[test]
    fn test_boolean_sidebar_and_search_settings() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        assert!(settings.get_sidebar_visible());
        assert!(!settings.get_terminal_visible());
        assert!(!settings.get_search_case_sensitive());
        assert!(!settings.get_search_whole_word());

        settings.set_active_sidebar_tab("search");
        settings.set_sidebar_visible(false);
        settings.set_terminal_visible(true);
        settings.set_search_case_sensitive(true);
        settings.set_search_whole_word(true);
        settings.set_search_query("needle");

        assert_eq!(settings.get_active_sidebar_tab(), "search");
        assert!(!settings.get_sidebar_visible());
        assert!(settings.get_terminal_visible());
        assert!(settings.get_search_case_sensitive());
        assert!(settings.get_search_whole_word());
        assert_eq!(settings.get_search_query(), "needle");
    }

    #[test]
    fn test_dimension_getters_clamp_invalid_values() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set("window_width", "10");
        settings.set("window_height", "20");
        settings.set("file_panel_width", "30");
        settings.set("terminal_height", "40");

        assert_eq!(settings.get_window_width(), 400);
        assert_eq!(settings.get_window_height(), 300);
        assert_eq!(settings.get_file_panel_width(), 100);
        assert_eq!(settings.get_terminal_height(), 100);
    }

    #[test]
    fn test_invalid_numeric_values_fall_back_to_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set("font_size", "large");
        settings.set("terminal_font_size", "huge");
        settings.set("audio_volume", "loud");

        assert_eq!(settings.get_font_size(), DEFAULT_FONT_SIZE);
        assert_eq!(settings.get_terminal_font_size(), DEFAULT_TERMINAL_FONT_SIZE);
        assert_eq!(settings.get_audio_volume(), DEFAULT_AUDIO_VOLUME);
    }

    #[test]
    fn test_save_and_load_from_file_roundtrip() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set_font_size(18);
        settings.set_dark_theme("test-dark");
        settings.set_git_commit_message("coverage tests");
        settings.save().unwrap();

        let saved = std::fs::read_to_string(&settings.config_path).unwrap();
        assert!(saved.contains("# Text Editor Settings"));
        assert!(saved.contains("font_size=18"));

        let mut loaded = settings_with_temp_path(&temp_dir);
        loaded.load_from_file().unwrap();

        assert_eq!(loaded.get_font_size(), 18);
        assert_eq!(loaded.get_dark_theme(), "test-dark");
        assert_eq!(loaded.get_git_commit_message(), "coverage tests");
    }

    #[test]
    fn test_load_from_file_ignores_lines_without_separator() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        std::fs::write(
            &settings.config_path,
            "invalid line\nfont_size=16\nkey = trimmed value\n",
        )
        .unwrap();

        settings.load_from_file().unwrap();

        assert_eq!(settings.get_font_size(), 16);
        assert_eq!(settings.get("key"), Some(&"trimmed value".to_string()));
        assert!(settings.get("invalid line").is_none());
    }

    #[test]
    fn test_config_dir_creation() {
        let config_dir = get_config_dir_public();
        assert!(config_dir.is_absolute());
        assert!(
            config_dir.to_string_lossy().contains("dvop"),
            "config dir should live under dvop: {config_dir:?}"
        );
    }

    #[test]
    fn test_default_themes_detection() {
        let (light, dark) = detect_os_default_themes();
        assert!(!light.is_empty());
        assert!(!dark.is_empty());
    }

    #[test]
    fn test_git_commit_message_defaults_to_empty_string() {
        let settings = EditorSettings::new();
        assert!(settings.get_git_commit_message().is_empty());
    }

    #[test]
    fn test_set_opened_files_serializes_multiple_paths() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);
        let files = vec![
            temp_dir.path().join("one.rs"),
            temp_dir.path().join("two.py"),
        ];

        settings.set_opened_files(&files);
        assert_eq!(settings.get_opened_files(), files);
    }

    #[test]
    fn test_terminal_visible_defaults_to_false() {
        // Use a fresh settings instance with a temp path to avoid loading from disk
        let temp_dir = tempfile::TempDir::new().unwrap();
        let settings = settings_with_temp_path(&temp_dir);
        assert!(!settings.get_terminal_visible());
    }

    #[test]
    fn test_set_window_size_updates_both_dimensions() {
        let mut settings = EditorSettings::new();
        settings.set_window_size(1440, 900);
        assert_eq!(settings.get_window_width(), 1440);
        assert_eq!(settings.get_window_height(), 900);
    }

    #[test]
    fn test_sidebar_visible_defaults_to_true() {
        let settings = EditorSettings::new();
        assert!(settings.get_sidebar_visible());
    }

    #[test]
    fn test_search_query_defaults_to_empty_string() {
        let temp_dir = TempDir::new().unwrap();
        let settings = settings_with_temp_path(&temp_dir);
        assert!(settings.get_search_query().is_empty());
    }

    #[test]
    fn test_generic_get_and_set_round_trip_custom_key() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set("custom_feature_flag", "enabled");
        assert_eq!(
            settings.get("custom_feature_flag").map(String::as_str),
            Some("enabled")
        );
        assert!(settings.get("missing_custom_key").is_none());
    }

    #[test]
    #[serial]
    fn get_config_dir_public_honors_xdg_config_home() {
        let temp = tempfile::tempdir().unwrap();
        let previous = std::env::var("XDG_CONFIG_HOME").ok();
        std::env::set_var("XDG_CONFIG_HOME", temp.path());

        let config_dir = get_config_dir_public();
        assert_eq!(config_dir, temp.path().join("dvop"));

        match previous {
            Some(value) => std::env::set_var("XDG_CONFIG_HOME", value),
            None => std::env::remove_var("XDG_CONFIG_HOME"),
        }
    }

    #[test]
    fn test_git_commit_message_round_trips_through_settings() {
        let temp_dir = TempDir::new().unwrap();
        let mut settings = settings_with_temp_path(&temp_dir);

        settings.set_git_commit_message("fix: handle edge case");
        assert_eq!(
            settings.get_git_commit_message(),
            "fix: handle edge case"
        );
    }
