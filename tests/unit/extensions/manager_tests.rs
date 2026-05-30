    use super::*;
    use crate::extensions::{
        CommandContribution, ContextMenuContributions, CssContribution, EditorContextMenu,
        ExtensionContributions, FileExplorerContextMenu, HooksContribution, KeybindingContribution,
        LinterContribution, SidebarPanelContribution, StatusBarContribution,
        TextTransformContribution,
    };

    fn manifest(id: &str, enabled: bool) -> ExtensionManifest {
        ExtensionManifest {
            id: id.to_string(),
            name: format!("{} extension", id),
            version: "1.0.0".to_string(),
            description: "test extension".to_string(),
            author: "tests".to_string(),
            enabled,
            icon: None,
            is_native: false,
            contributions: ExtensionContributions::default(),
        }
    }

    fn manager_with_extensions(extensions: Vec<Extension>) -> ExtensionManager {
        ExtensionManager {
            extensions,
            extensions_dir: PathBuf::new(),
        }
    }

    #[test]
    fn load_extensions_reads_valid_manifests_and_skips_invalid_entries() {
        let dir = tempfile::tempdir().unwrap();
        let valid_dir = dir.path().join("valid");
        let invalid_dir = dir.path().join("invalid");
        let no_manifest_dir = dir.path().join("no-manifest");
        std::fs::create_dir_all(&valid_dir).unwrap();
        std::fs::create_dir_all(&invalid_dir).unwrap();
        std::fs::create_dir_all(&no_manifest_dir).unwrap();
        std::fs::write(dir.path().join("not-a-dir"), "ignored").unwrap();

        let valid_manifest = manifest("valid", true);
        std::fs::write(
            valid_dir.join("manifest.json"),
            serde_json::to_string(&valid_manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(invalid_dir.join("manifest.json"), "{not json").unwrap();

        let mut manager = ExtensionManager {
            extensions: vec![Extension::new(manifest("stale", true), PathBuf::new())],
            extensions_dir: dir.path().to_path_buf(),
        };

        manager.load_extensions();

        assert_eq!(manager.get_extensions().len(), 1);
        assert_eq!(manager.get_extensions()[0].manifest.id, "valid");
        assert_eq!(manager.get_extensions()[0].path, valid_dir);
    }

    #[test]
    fn load_extensions_creates_missing_directory() {
        let dir = tempfile::tempdir().unwrap();
        let extensions_dir = dir.path().join("extensions");
        let mut manager = ExtensionManager {
            extensions: Vec::new(),
            extensions_dir: extensions_dir.clone(),
        };

        manager.load_extensions();

        assert!(extensions_dir.exists());
        assert!(manager.get_extensions().is_empty());
    }

    #[test]
    fn set_enabled_updates_manifest_on_disk() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("toggle");
        std::fs::create_dir_all(&ext_dir).unwrap();
        let mut ext_manifest = manifest("toggle", true);
        std::fs::write(
            ext_dir.join("manifest.json"),
            serde_json::to_string(&ext_manifest).unwrap(),
        )
        .unwrap();
        let mut manager = ExtensionManager {
            extensions: vec![Extension::new(ext_manifest.clone(), ext_dir.clone())],
            extensions_dir: dir.path().to_path_buf(),
        };

        manager.set_enabled("toggle", false);

        assert!(!manager.get_extensions()[0].manifest.enabled);
        ext_manifest = serde_json::from_str(
            &std::fs::read_to_string(ext_dir.join("manifest.json")).unwrap(),
        )
        .unwrap();
        assert!(!ext_manifest.enabled);
    }

    #[test]
    fn remove_extension_deletes_directory_and_returns_name() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("remove-me");
        std::fs::create_dir_all(&ext_dir).unwrap();
        let mut manager = ExtensionManager {
            extensions: vec![Extension::new(manifest("remove-me", true), ext_dir.clone())],
            extensions_dir: dir.path().to_path_buf(),
        };

        let removed = manager.remove_extension("remove-me").unwrap();

        assert_eq!(removed, "remove-me extension");
        assert!(manager.get_extensions().is_empty());
        assert!(!ext_dir.exists());
        assert_eq!(
            manager.remove_extension("remove-me").unwrap_err(),
            "Extension not found"
        );
    }

    #[test]
    fn run_status_bar_scripts_joins_enabled_non_empty_outputs() {
        let dir = tempfile::tempdir().unwrap();
        let enabled_dir = dir.path().join("enabled");
        let second_dir = dir.path().join("second");
        let disabled_dir = dir.path().join("disabled");
        std::fs::create_dir_all(&enabled_dir).unwrap();
        std::fs::create_dir_all(&second_dir).unwrap();
        std::fs::create_dir_all(&disabled_dir).unwrap();
        std::fs::write(enabled_dir.join("status.sh"), "printf 'one:%s' \"$1\"").unwrap();
        std::fs::write(second_dir.join("status.sh"), "printf 'two'").unwrap();
        std::fs::write(disabled_dir.join("status.sh"), "printf 'disabled'").unwrap();

        let mut enabled = manifest("enabled", true);
        enabled.contributions.status_bar = Some(StatusBarContribution {
            script: "status.sh".to_string(),
        });
        let mut second = manifest("second", true);
        second.contributions.status_bar = Some(StatusBarContribution {
            script: "status.sh".to_string(),
        });
        let mut disabled = manifest("disabled", false);
        disabled.contributions.status_bar = Some(StatusBarContribution {
            script: "status.sh".to_string(),
        });
        let mut missing_script = manifest("missing", true);
        missing_script.contributions.status_bar = Some(StatusBarContribution {
            script: "missing.sh".to_string(),
        });

        let manager = manager_with_extensions(vec![
            Extension::new(enabled, enabled_dir),
            Extension::new(second, second_dir),
            Extension::new(disabled, disabled_dir),
            Extension::new(missing_script, dir.path().join("missing")),
        ]);

        let text = manager.run_status_bar_scripts(Path::new("file.rs"));

        assert_eq!(text, "one:file.rs | two");
    }

    #[test]
    fn contribution_getters_filter_enabled_extensions() {
        let dir = tempfile::tempdir().unwrap();
        let enabled_dir = dir.path().join("enabled");
        let disabled_dir = dir.path().join("disabled");
        std::fs::create_dir_all(&enabled_dir).unwrap();
        std::fs::create_dir_all(&disabled_dir).unwrap();
        std::fs::write(enabled_dir.join("style.css"), ".enabled {}").unwrap();
        std::fs::write(disabled_dir.join("style.css"), ".disabled {}").unwrap();

        let mut enabled = manifest("enabled", true);
        enabled.contributions.css = Some(CssContribution {
            file: "style.css".to_string(),
        });
        enabled.contributions.commands.push(CommandContribution {
            id: "cmd".to_string(),
            title: "Command".to_string(),
            script: "cmd.sh".to_string(),
            keywords: vec!["run".to_string()],
        });
        enabled
            .contributions
            .text_transforms
            .push(TextTransformContribution {
                id: "upper".to_string(),
                title: "Uppercase".to_string(),
                script: "upper.sh".to_string(),
            });
        enabled.contributions.context_menus = Some(ContextMenuContributions {
            editor: vec![EditorContextMenu {
                label: "Format".to_string(),
                script: "format.sh".to_string(),
            }],
            file_explorer: vec![FileExplorerContextMenu {
                label: "Rename".to_string(),
                script: "rename.sh".to_string(),
            }],
        });
        enabled.contributions.keybindings.push(KeybindingContribution {
            key: "Ctrl+Alt+T".to_string(),
            title: "Transform".to_string(),
            script: "transform.sh".to_string(),
        });
        enabled.contributions.linters.push(LinterContribution {
            languages: vec!["rs".to_string()],
            script: "lint.sh".to_string(),
        });
        enabled.contributions.hooks = Some(HooksContribution {
            on_file_open: Some("open.sh".to_string()),
            on_file_save: None,
            on_file_close: None,
            on_app_start: None,
        });

        let mut disabled = manifest("disabled", false);
        disabled.contributions.css = Some(CssContribution {
            file: "style.css".to_string(),
        });
        disabled.contributions.commands.push(CommandContribution {
            id: "disabled-cmd".to_string(),
            title: "Disabled Command".to_string(),
            script: "disabled.sh".to_string(),
            keywords: Vec::new(),
        });

        let manager = manager_with_extensions(vec![
            Extension::new(enabled, enabled_dir.clone()),
            Extension::new(disabled, disabled_dir),
        ]);

        assert_eq!(
            manager.get_extension_css_paths(),
            vec![enabled_dir.join("style.css")]
        );
        assert_eq!(manager.get_extension_commands().len(), 1);
        assert_eq!(manager.get_extension_transforms().len(), 1);
        assert_eq!(manager.get_file_explorer_context_menu_entries().len(), 1);
    }

    #[test]
    fn sidebar_panels_include_disabled_extensions_with_enabled_flag() {
        let mut enabled = manifest("enabled", true);
        enabled
            .contributions
            .sidebar_panels
            .push(SidebarPanelContribution {
                id: "enabled-panel".to_string(),
                title: "Enabled".to_string(),
                icon: "icon".to_string(),
                script: "enabled.sh".to_string(),
            });
        let mut disabled = manifest("disabled", false);
        disabled
            .contributions
            .sidebar_panels
            .push(SidebarPanelContribution {
                id: "disabled-panel".to_string(),
                title: "Disabled".to_string(),
                icon: "icon".to_string(),
                script: "disabled.sh".to_string(),
            });
        let manager = manager_with_extensions(vec![
            Extension::new(enabled, PathBuf::from("enabled")),
            Extension::new(disabled, PathBuf::from("disabled")),
        ]);

        let panels = manager.get_extension_sidebar_panels();

        assert_eq!(panels.len(), 2);
        assert_eq!(panels[0].0, "enabled");
        assert!(panels[0].2);
        assert_eq!(panels[1].0, "disabled");
        assert!(!panels[1].2);
    }

    #[test]
    fn get_extensions_dir_lives_under_config_dvop() {
        let dir = get_extensions_dir();
        let path = dir.to_string_lossy();
        assert!(path.contains("dvop"));
        assert!(path.contains("extensions"));
    }

    #[test]
    fn get_all_extensions_includes_manager_extensions() {
        let enabled = manifest("enabled", true);
        let manager = manager_with_extensions(vec![Extension::new(
            enabled.clone(),
            PathBuf::from("/tmp/enabled"),
        )]);

        assert_eq!(manager.get_extensions().len(), 1);
        let all = manager.get_all_extensions();
        assert!(all.len() >= 1);
        assert!(all.iter().any(|e| e.manifest.id == "enabled"));
    }

    #[test]
    fn set_enabled_on_missing_extension_is_noop() {
        let mut manager = manager_with_extensions(Vec::new());
        manager.set_enabled("missing-extension", true);
    }

    #[test]
    fn install_from_archive_rejects_missing_archive() {
        let missing = PathBuf::from("/tmp/dvop-missing-extension-archive.tar.gz");
        assert!(install_from_archive(&missing).is_err());
    }

    #[test]
    fn run_status_bar_scripts_ignores_blank_script_output() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("blank-status");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("status.sh"), "printf '   \\n'").unwrap();

        let mut enabled = manifest("blank-status", true);
        enabled.contributions.status_bar = Some(StatusBarContribution {
            script: "status.sh".to_string(),
        });

        let manager = manager_with_extensions(vec![Extension::new(enabled, ext_dir)]);
        assert!(manager.run_status_bar_scripts(Path::new("/tmp/file.rs")).is_empty());
    }

    #[test]
    fn get_extension_css_paths_returns_enabled_stylesheets() {
        let dir = tempfile::tempdir().unwrap();
        let ext_dir = dir.path().join("styled");
        std::fs::create_dir_all(&ext_dir).unwrap();
        std::fs::write(ext_dir.join("theme.css"), ".tab { color: red; }").unwrap();

        let mut enabled = manifest("styled", true);
        enabled.contributions.css = Some(CssContribution {
            file: "theme.css".to_string(),
        });
        let mut disabled = manifest("hidden", false);
        disabled.contributions.css = Some(CssContribution {
            file: "hidden.css".to_string(),
        });

        let manager = manager_with_extensions(vec![
            Extension::new(enabled, ext_dir.clone()),
            Extension::new(disabled, dir.path().join("hidden")),
        ]);

        let paths = manager.get_extension_css_paths();
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0], ext_dir.join("theme.css"));
    }
