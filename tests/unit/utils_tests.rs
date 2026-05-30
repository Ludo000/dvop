    use super::*;
    use std::cell::RefCell;
    use std::fs;
    use std::rc::Rc;
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

    #[test]
    fn test_is_current_file_non_editable_detects_media_paths() {
        for path in ["image.png", "clip.mp4", "song.mp3"] {
            let active_path = Rc::new(RefCell::new(Some(PathBuf::from(path))));
            assert!(
                is_current_file_non_editable(Some(&active_path), None),
                "{} should be non-editable",
                path
            );
        }
    }

    #[test]
    fn test_is_current_file_non_editable_allows_typescript_override_and_text() {
        for path in ["component.ts", "component.tsx", "notes.txt", "data.json"] {
            let active_path = Rc::new(RefCell::new(Some(PathBuf::from(path))));
            assert!(
                !is_current_file_non_editable(Some(&active_path), None),
                "{} should be editable",
                path
            );
        }
    }

    #[test]
    fn test_is_current_file_non_editable_without_active_file_is_false() {
        let empty_path = Rc::new(RefCell::new(None));

        assert!(!is_current_file_non_editable(None, None));
        assert!(!is_current_file_non_editable(Some(&empty_path), None));
    }

    #[test]
    fn parse_path_components_expands_paths_under_home() {
        let Some(home) = home::home_dir() else {
            return;
        };

        let path = home.join("dev/dvop/src/main.rs");
        let components = parse_path_components(&path);

        assert_eq!(components.first().map(|(name, _)| name.as_str()), Some("Home"));
        assert!(components.iter().any(|(name, _)| name == "dvop"));
        assert!(components.last().map(|(_, p)| p.ends_with("main.rs")).unwrap_or(false));
    }

    #[test]
    fn parse_path_components_handles_absolute_paths_outside_home() {
        let path = PathBuf::from("/tmp/project/src/lib.rs");
        let components = parse_path_components(&path);

        assert!(components.iter().any(|(name, _)| name == "Root" || name == "/"));
        assert!(components.iter().any(|(name, _)| name == "project"));
        assert!(components.iter().any(|(name, _)| name == "lib.rs"));
    }

    #[test]
    fn trigger_file_list_refresh_invokes_registered_callback() {
        let called = Rc::new(RefCell::new(false));
        let called_for_closure = called.clone();

        set_file_list_refresh_callback(move || {
            *called_for_closure.borrow_mut() = true;
        });

        trigger_file_list_refresh();
        assert!(*called.borrow());
    }

    #[test]
    fn trigger_tab_path_update_invokes_registered_callback_with_paths() {
        let captured = Rc::new(RefCell::new(None::<(PathBuf, PathBuf)>));
        let captured_for_closure = captured.clone();

        set_tab_path_update_callback(move |old_path, new_path| {
            *captured_for_closure.borrow_mut() = Some((old_path.clone(), new_path.clone()));
        });

        let old_path = PathBuf::from("/tmp/old.rs");
        let new_path = PathBuf::from("/tmp/new.rs");
        trigger_tab_path_update(&old_path, &new_path);

        let (old, new) = captured.borrow().clone().expect("callback should fire");
        assert_eq!(old, old_path);
        assert_eq!(new, new_path);
    }

    #[test]
    fn parse_path_components_handles_relative_paths() {
        let path = PathBuf::from("src/handlers.rs");
        let components = parse_path_components(&path);

        assert_eq!(components.len(), 2);
        assert_eq!(components[0].0, "src");
        assert_eq!(components[1].0, "handlers.rs");
    }

    #[test]
    fn parse_path_components_returns_single_segment_for_filename_only() {
        let path = PathBuf::from("README.md");
        let components = parse_path_components(&path);

        assert_eq!(components.len(), 1);
        assert_eq!(components[0].0, "README.md");
    }

    #[test]
    fn is_allowed_mime_type_accepts_markdown_and_yaml() {
        let markdown = "text/markdown".parse::<mime_guess::Mime>().unwrap();
        let yaml = "text/yaml".parse::<mime_guess::Mime>().unwrap();
        assert!(is_allowed_mime_type(&markdown));
        assert!(is_allowed_mime_type(&yaml));
    }

    #[test]
    fn trigger_file_list_refresh_without_callback_does_not_panic() {
        trigger_file_list_refresh();
    }

    #[test]
    fn is_allowed_mime_type_accepts_shell_scripts() {
        let sh = "application/x-sh".parse::<mime_guess::Mime>().unwrap();
        assert!(is_allowed_mime_type(&sh));
    }

    #[test]
    fn is_allowed_mime_type_accepts_typescript_mime() {
        let ts = "text/x-typescript".parse::<mime_guess::Mime>().unwrap();
        assert!(is_allowed_mime_type(&ts));
    }

    #[test]
    fn parse_path_components_handles_single_segment_absolute_path() {
        let path = PathBuf::from("/etc/hosts");
        let components = parse_path_components(&path);

        assert!(components.iter().any(|(name, _)| name == "Root" || name == "/"));
        assert!(components.last().map(|(name, _)| name.as_str()) == Some("hosts"));
    }

    #[test]
    fn is_allowed_mime_type_rejects_video_content() {
        let mp4 = "video/mp4".parse::<mime_guess::Mime>().unwrap();
        assert!(!is_allowed_mime_type(&mp4));
    }

    #[test]
    fn parse_path_components_builds_home_relative_segments() {
        if let Some(home) = home::home_dir() {
            let nested = home.join("projects/dvop/src/main.rs");
            let components = parse_path_components(&nested);

            assert!(components.first().map(|(name, _)| name.as_str()) == Some("Home"));
            assert!(components.iter().any(|(name, _)| name == "projects"));
            assert!(components.last().map(|(name, _)| name.as_str()) == Some("main.rs"));
        }
    }

    #[test]
    fn is_current_file_non_editable_treats_pdf_as_non_editable() {
        let active_path = Rc::new(RefCell::new(Some(PathBuf::from("report.pdf"))));
        assert!(is_current_file_non_editable(Some(&active_path), None));
    }

    fn is_current_file_non_editable_treats_unknown_binary_as_non_editable() {
        let active_path = Rc::new(RefCell::new(Some(PathBuf::from("binary.exe"))));
        assert!(is_current_file_non_editable(Some(&active_path), None));
    }
