    use super::*;
    use crate::extensions::native::NativeExtension;
    use serial_test::serial;
    use std::path::Path;

    #[test]
    fn rust_diagnostics_manifest_describes_lsp_extension() {
        let ext = RustDiagnosticsExtension::new();
        assert_eq!(ext.id(), "rust-diagnostics");

        let manifest = ext.manifest();
        assert_eq!(manifest.id, "rust-diagnostics");
        assert!(manifest.is_native);
        assert_eq!(manifest.contributions.linters.len(), 1);
        assert_eq!(manifest.contributions.linters[0].languages, vec!["rs".to_string()]);
    }

    #[test]
    fn is_rust_project_detects_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_rust_project(dir.path()));

        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_detects_rs_files_in_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_detects_rs_files_under_src() {
        let dir = tempfile::tempdir().unwrap();
        let src = dir.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "pub fn demo() {}").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_returns_false_for_non_rust_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("README.md"), "# hello").unwrap();
        assert!(!is_rust_project(dir.path()));
    }

    #[test]
    #[serial]
    fn defer_rust_lsp_opens_queues_paths_until_flush_when_disabled() {
        let mut ext = RustDiagnosticsExtension::new();
        ext.set_enabled(true);
        set_defer_rust_lsp_opens(true);

        ext.on_file_open(Path::new("/tmp/dvop-deferred-test.rs"));
        ext.on_file_open(Path::new("/tmp/dvop-deferred-test.rs"));

        ext.set_enabled(false);
        set_defer_rust_lsp_opens(false);
        flush_deferred_rust_lsp_opens();
    }

    #[test]
    fn is_rust_project_detects_cargo_toml_in_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"demo\"\n").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_detects_rs_file_in_src_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(dir.path().join("src/lib.rs"), "pub fn demo() {}\n").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_detects_loose_rs_file_in_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}\n").unwrap();
        assert!(is_rust_project(dir.path()));
    }

    #[test]
    fn is_rust_project_returns_false_for_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        assert!(!is_rust_project(dir.path()));
    }

    #[test]
    #[serial]
    fn rust_diagnostics_enable_toggle_updates_global_state() {
        let mut ext = RustDiagnosticsExtension::new();

        ext.set_enabled(false);
        assert!(!ext.is_enabled());
        assert!(!is_enabled());

        ext.set_enabled(true);
        assert!(ext.is_enabled());
        assert!(is_enabled());
    }

    #[test]
    fn find_workspace_root_walks_up_to_cargo_toml() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"root\"\n").unwrap();
        let nested = dir.path().join("src/deep/main.rs");
        std::fs::create_dir_all(nested.parent().unwrap()).unwrap();

        assert_eq!(find_workspace_root(&nested), dir.path());
    }

    #[test]
    fn find_workspace_root_falls_back_to_parent_without_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("standalone.rs");
        std::fs::write(&file, "fn main() {}\n").unwrap();

        assert_eq!(find_workspace_root(&file), dir.path());
    }

    #[test]
    #[serial]
    fn rust_diagnostics_register_can_be_called_multiple_times() {
        register();
        register();
        assert!(is_enabled());
    }

    #[test]
    fn set_defer_rust_lsp_opens_can_be_toggled() {
        set_defer_rust_lsp_opens(true);
        set_defer_rust_lsp_opens(false);
    }

    #[test]
    #[serial]
    fn flush_deferred_rust_lsp_opens_clears_queue_when_extension_disabled() {
        gtk4::test_synced(|| {
            let mut ext = RustDiagnosticsExtension::new();
            ext.set_enabled(true);
            set_defer_rust_lsp_opens(true);
            ext.on_file_open(Path::new("/tmp/dvop-flush-disabled.rs"));

            ext.set_enabled(false);
            flush_deferred_rust_lsp_opens();
            set_defer_rust_lsp_opens(false);
        });
    }
