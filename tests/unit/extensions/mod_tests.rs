    use super::*;

    fn sample_manifest() -> ExtensionManifest {
        ExtensionManifest {
            id: "test-ext".to_string(),
            name: "Test Extension".to_string(),
            version: "0.1.0".to_string(),
            description: "A test extension".to_string(),
            author: "tests".to_string(),
            enabled: true,
            icon: Some("application-x-addon-symbolic".to_string()),
            is_native: false,
            contributions: ExtensionContributions {
                keybindings: vec![KeybindingContribution {
                    key: "Ctrl+Shift+T".to_string(),
                    title: "Run Test".to_string(),
                    script: "run.sh".to_string(),
                }],
                linters: vec![LinterContribution {
                    languages: vec!["py".to_string()],
                    script: "lint.sh".to_string(),
                }],
                ..Default::default()
            },
        }
    }

    #[test]
    fn extension_manifest_roundtrips_through_json() {
        let manifest = sample_manifest();
        let json = serde_json::to_string(&manifest).unwrap();
        let parsed: ExtensionManifest = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, "test-ext");
        assert_eq!(parsed.contributions.keybindings.len(), 1);
        assert_eq!(parsed.contributions.linters[0].languages, vec!["py"]);
        assert!(parsed.enabled);
    }

    #[test]
    fn extension_manifest_defaults_optional_fields() {
        let json = r#"{
            "id": "minimal",
            "name": "Minimal",
            "version": "1.0.0",
            "description": "desc",
            "author": "author"
        }"#;

        let parsed: ExtensionManifest = serde_json::from_str(json).unwrap();
        assert!(!parsed.enabled);
        assert!(!parsed.is_native);
        assert!(parsed.icon.is_none());
        assert!(parsed.contributions.keybindings.is_empty());
    }

    #[test]
    fn extension_new_stores_manifest_and_path() {
        let manifest = sample_manifest();
        let path = std::path::PathBuf::from("/tmp/extensions/test-ext");
        let ext = Extension::new(manifest.clone(), path.clone());

        assert_eq!(ext.manifest.id, manifest.id);
        assert_eq!(ext.path, path);
    }

    #[test]
    fn sidebar_panel_default_icon_is_addon_symbolic() {
        assert_eq!(
            default_panel_icon(),
            "application-x-addon-symbolic".to_string()
        );
    }

    #[test]
    fn hooks_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "hooks-ext",
            "name": "Hooks",
            "version": "1.0.0",
            "description": "hooks",
            "author": "tests",
            "contributions": {
                "hooks": {
                    "on_file_open": "open.sh",
                    "on_file_save": "save.sh",
                    "on_app_start": "start.sh"
                }
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        let hooks = manifest.contributions.hooks.expect("hooks contribution");
        assert_eq!(hooks.on_file_open.as_deref(), Some("open.sh"));
        assert_eq!(hooks.on_file_save.as_deref(), Some("save.sh"));
        assert_eq!(hooks.on_app_start.as_deref(), Some("start.sh"));
    }

    #[test]
    fn command_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "cmd-ext",
            "name": "Commands",
            "version": "1.0.0",
            "description": "commands",
            "author": "tests",
            "contributions": {
                "commands": [{
                    "id": "format",
                    "title": "Format Selection",
                    "script": "format.sh",
                    "keywords": ["format", "pretty"]
                }]
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.contributions.commands.len(), 1);
        assert_eq!(manifest.contributions.commands[0].id, "format");
        assert_eq!(manifest.contributions.commands[0].keywords, vec!["format", "pretty"]);
    }

    #[test]
    fn css_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "css-ext",
            "name": "CSS",
            "version": "1.0.0",
            "description": "css",
            "author": "tests",
            "contributions": {
                "css": { "file": "theme.css" }
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        let css = manifest.contributions.css.expect("css contribution");
        assert_eq!(css.file, "theme.css");
    }

    #[test]
    fn status_bar_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "status-ext",
            "name": "Status",
            "version": "1.0.0",
            "description": "status",
            "author": "tests",
            "contributions": {
                "status_bar": { "script": "status.sh" }
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        let status = manifest.contributions.status_bar.expect("status contribution");
        assert_eq!(status.script, "status.sh");
    }

    #[test]
    fn text_transform_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "transform-ext",
            "name": "Transform",
            "version": "1.0.0",
            "description": "transform",
            "author": "tests",
            "contributions": {
                "text_transforms": [{
                    "id": "upper",
                    "title": "Uppercase",
                    "script": "upper.sh"
                }]
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.contributions.text_transforms.len(), 1);
        assert_eq!(manifest.contributions.text_transforms[0].id, "upper");
    }

    #[test]
    fn linter_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "lint-ext",
            "name": "Lint",
            "version": "1.0.0",
            "description": "lint",
            "author": "tests",
            "contributions": {
                "linters": [{
                    "languages": ["py", "js"],
                    "script": "lint.sh"
                }]
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.contributions.linters.len(), 1);
        assert_eq!(manifest.contributions.linters[0].languages, vec!["py", "js"]);
    }

    #[test]
    fn keybinding_contribution_deserializes_from_manifest_json() {
        let json = r#"{
            "id": "keys-ext",
            "name": "Keys",
            "version": "1.0.0",
            "description": "keys",
            "author": "tests",
            "contributions": {
                "keybindings": [{
                    "key": "Ctrl+Shift+P",
                    "title": "Palette",
                    "script": "palette.sh"
                }]
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        assert_eq!(manifest.contributions.keybindings.len(), 1);
        assert_eq!(manifest.contributions.keybindings[0].key, "Ctrl+Shift+P");
    }

    #[test]
    fn context_menu_contributions_deserialize_editor_and_explorer_entries() {
        let json = r#"{
            "id": "ctx-ext",
            "name": "Context",
            "version": "1.0.0",
            "description": "context",
            "author": "tests",
            "contributions": {
                "context_menus": {
                    "editor": [{
                        "label": "Format",
                        "script": "format.sh"
                    }],
                    "file_explorer": [{
                        "label": "Reveal",
                        "script": "reveal.sh"
                    }]
                }
            }
        }"#;

        let manifest: ExtensionManifest = serde_json::from_str(json).unwrap();
        let menus = manifest.contributions.context_menus.expect("context menus");
        assert_eq!(menus.editor.len(), 1);
        assert_eq!(menus.file_explorer.len(), 1);
        assert_eq!(menus.editor[0].label, "Format");
        assert_eq!(menus.file_explorer[0].label, "Reveal");
    }
