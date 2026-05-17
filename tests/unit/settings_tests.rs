    use super::*;
    use tempfile::TempDir;

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
    fn test_config_dir_creation() {
        let config_dir = get_config_dir_public();
        assert!(config_dir.is_absolute());
    }

    #[test]
    fn test_default_themes_detection() {
        let (light, dark) = detect_os_default_themes();
        assert!(!light.is_empty());
        assert!(!dark.is_empty());
    }
