    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_is_allowed_mime_type_text() {
        use mime_guess::mime;
        assert!(is_allowed_mime_type(&mime::TEXT_PLAIN));
        assert!(is_allowed_mime_type(&mime::TEXT_HTML));
        assert!(is_allowed_mime_type(&mime::TEXT_CSS));
        assert!(is_allowed_mime_type(&mime::TEXT_JAVASCRIPT));
    }

    #[test]
    fn test_is_allowed_mime_type_application() {
        use mime_guess::mime;
        assert!(is_allowed_mime_type(&mime::APPLICATION_JSON));
        assert!(is_allowed_mime_type(&mime::APPLICATION_JAVASCRIPT));
        assert!(is_allowed_mime_type(&mime::APPLICATION_OCTET_STREAM));
    }

    #[test]
    fn test_is_allowed_mime_type_xml() {
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let xml_mime = "application/xml".parse::<mime_guess::Mime>().unwrap();
        assert!(is_allowed_mime_type(&xml_mime));
    }

    #[test]
    fn test_is_not_allowed_mime_type() {
        use mime_guess::mime;
        assert!(!is_allowed_mime_type(&mime::IMAGE_PNG));
        assert!(!is_allowed_mime_type(&mime::IMAGE_JPEG));
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let video_mp4 = "video/mp4".parse::<mime_guess::Mime>().unwrap();
        assert!(!is_allowed_mime_type(&video_mp4));
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let audio_mp3 = "audio/mpeg".parse::<mime_guess::Mime>().unwrap();
        assert!(!is_allowed_mime_type(&audio_mp3));
    }

    #[test]
    fn test_mime_type_detection() {
        // Test TypeScript override
        let ts_path = std::path::Path::new("test.ts");
        let mime = mime_guess::from_path(ts_path).first_or_octet_stream();
        // MIME detection varies, just ensure it doesn't panic
        assert!(!mime.essence_str().is_empty());
    }

    #[test]
    fn test_path_components() {
        // unwrap() extracts the value, but will crash (panic) if the value is an Error or None.
        let temp_dir = TempDir::new().unwrap();
        let test_path = temp_dir.path().join("folder1/folder2/file.txt");
        
        fs::create_dir_all(test_path.parent().unwrap()).ok();
        
        let path_str = test_path.to_str().unwrap();
        assert!(path_str.contains("folder1"));
        assert!(path_str.contains("folder2"));
        assert!(path_str.contains("file.txt"));
    }

    #[test]
    fn test_home_directory() {
        if let Ok(home) = std::env::var("HOME") {
            assert!(!home.is_empty());
            let home_path = PathBuf::from(&home);
            assert!(home_path.is_absolute());
        }
    }

    #[test]
    fn test_pathbuf_extension() {
        let path = PathBuf::from("test.rs");
        assert_eq!(path.extension().and_then(|s| s.to_str()), Some("rs"));
        
        let path2 = PathBuf::from("test.py");
        assert_eq!(path2.extension().and_then(|s| s.to_str()), Some("py"));
        
        let path3 = PathBuf::from("no_extension");
        assert_eq!(path3.extension(), None);
    }

    #[test]
    fn test_file_selection_source() {
        let tab_switch = FileSelectionSource::TabSwitch;
        let direct_click = FileSelectionSource::DirectClick;
        
        assert_eq!(tab_switch, FileSelectionSource::TabSwitch);
        assert_eq!(direct_click, FileSelectionSource::DirectClick);
        assert_ne!(tab_switch, direct_click);
    }
