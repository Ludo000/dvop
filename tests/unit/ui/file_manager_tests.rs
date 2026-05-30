    use super::*;

    #[test]
    fn internal_clipboard_tracks_copy_and_cut_operations() {
        let copy_path = PathBuf::from("/tmp/dvop-copy-test.txt");
        copy_file_to_clipboard(&copy_path);

        let clip = get_clipboard_content().expect("copy should be stored");
        assert_eq!(clip.file_path, copy_path);
        assert_eq!(clip.operation, ClipboardOperation::Copy);
        assert!(!is_file_cut(&copy_path));

        let cut_path = PathBuf::from("/tmp/dvop-cut-test.txt");
        cut_file_to_clipboard(&cut_path);

        let clip = get_clipboard_content().expect("cut should be stored");
        assert_eq!(clip.file_path, cut_path);
        assert_eq!(clip.operation, ClipboardOperation::Cut);
        assert!(is_file_cut(&cut_path));

        clear_clipboard();
        assert!(!is_file_cut(&cut_path));
    }

    #[test]
    fn uri_to_path_decodes_common_file_uris() {
        assert_eq!(
            uri_to_path("file:///tmp/hello world.txt"),
            Some(PathBuf::from("/tmp/hello world.txt"))
        );
        assert_eq!(
            uri_to_path("file:///tmp/a%20b%25c.txt"),
            Some(PathBuf::from("/tmp/a b%c.txt"))
        );
        assert!(uri_to_path("/tmp/plain-path.txt").is_none());
    }

    #[test]
    fn parse_text_format_recognizes_dvop_prefixes() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("clip.txt");
        std::fs::write(&file_path, "payload").unwrap();

        let copy_text = format!("DVOP_COPY:{}", file_path.display());
        let parsed = parse_text_format(&copy_text).expect("DVOP_COPY should parse");
        assert_eq!(parsed.operation, ClipboardOperation::Copy);
        assert_eq!(parsed.file_path, file_path);

        let cut_text = format!("DVOP_CUT:{}", file_path.display());
        let parsed = parse_text_format(&cut_text).expect("DVOP_CUT should parse");
        assert_eq!(parsed.operation, ClipboardOperation::Cut);
    }

    #[test]
    fn parse_uri_list_format_reads_first_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("listed.txt");
        std::fs::write(&file_path, "data").unwrap();

        let uri = format!("file://{}\n", file_path.display());
        let parsed = parse_uri_list_format(&uri).expect("uri list should parse");
        assert_eq!(parsed.operation, ClipboardOperation::Copy);
        assert_eq!(parsed.file_path, file_path);
    }

    #[test]
    fn parse_gnome_clipboard_format_distinguishes_copy_and_cut() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("gnome.txt");
        std::fs::write(&file_path, "data").unwrap();

        let copy_payload = format!("copy\nfile://{}", file_path.display());
        let parsed = parse_gnome_clipboard_format(&copy_payload).expect("copy gnome format");
        assert_eq!(parsed.operation, ClipboardOperation::Copy);

        let cut_payload = format!("cut\nfile://{}", file_path.display());
        let parsed = parse_gnome_clipboard_format(&cut_payload).expect("cut gnome format");
        assert_eq!(parsed.operation, ClipboardOperation::Cut);
    }

    #[test]
    fn generate_unique_filename_avoids_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("report.pdf");
        std::fs::write(&original, "v1").unwrap();

        let unique = generate_unique_filename(&original);
        assert_ne!(unique, original);
        assert!(unique.file_name().unwrap().to_string_lossy().contains("(1)"));
        assert!(!unique.exists());
    }

    #[test]
    fn parse_text_format_accepts_existing_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("plain.txt");
        std::fs::write(&file_path, "payload").unwrap();

        let parsed = parse_text_format(&file_path.to_string_lossy())
            .expect("absolute path text should parse");
        assert_eq!(parsed.operation, ClipboardOperation::Copy);
        assert_eq!(parsed.file_path, file_path.canonicalize().unwrap());
    }

    #[test]
    fn generate_unique_filename_increments_when_numbered_copy_exists() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("notes.txt");
        let first_copy = dir.path().join("notes (1).txt");
        std::fs::write(&original, "v0").unwrap();
        std::fs::write(&first_copy, "v1").unwrap();

        let unique = generate_unique_filename(&original);
        assert_eq!(unique.file_name().unwrap().to_string_lossy(), "notes (2).txt");
    }

    #[test]
    fn copy_populates_internal_clipboard_content() {
        clear_clipboard();
        let path = PathBuf::from("/tmp/dvop-has-clip.txt");
        copy_file_to_clipboard(&path);

        let clip = get_clipboard_content().expect("internal copy should be readable");
        assert_eq!(clip.file_path, path);
        assert_eq!(clip.operation, ClipboardOperation::Copy);
    }

    #[test]
    fn parse_text_format_rejects_unknown_prefixes() {
        assert!(parse_text_format("UNKNOWN:/tmp/file.txt").is_none());
        assert!(parse_text_format("DVOP_COPY:").is_none());
    }

    #[test]
    fn generate_unique_filename_uses_numbered_copy_when_original_is_free() {
        let dir = tempfile::tempdir().unwrap();
        let original = dir.path().join("fresh.txt");
        let unique = generate_unique_filename(&original);
        assert_eq!(
            unique.file_name().unwrap().to_string_lossy(),
            "fresh (1).txt"
        );
        assert!(!unique.exists());
    }

    #[test]
    fn has_clipboard_content_is_true_after_internal_copy() {
        clear_clipboard();
        copy_file_to_clipboard(&PathBuf::from("/tmp/dvop-has-clip-copy.txt"));
        assert!(has_clipboard_content());
    }

    #[test]
    fn uri_to_path_rejects_non_file_schemes() {
        assert!(uri_to_path("http://example.com/file.txt").is_none());
        assert!(uri_to_path("ftp://host/path").is_none());
    }

    #[test]
    fn parse_uri_list_format_skips_missing_files() {
        let parsed = parse_uri_list_format("file:///tmp/does-not-exist-dvop.txt\n");
        assert!(parsed.is_none());
    }
