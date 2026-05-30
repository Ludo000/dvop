    use super::*;
    use crate::extensions::{
        CommandContribution, ContextMenuContributions, CssContribution, EditorContextMenu,
        ExtensionContributions, FileExplorerContextMenu, HooksContribution, KeybindingContribution,
        LinterContribution, SidebarPanelContribution, StatusBarContribution,
        TextTransformContribution,
    };
    use gtk4::prelude::*;
    use serial_test::serial;

    #[test]
    fn collect_badges_lists_keybinding_and_linter_contributions() {
        let contribs = ExtensionContributions {
            keybindings: vec![KeybindingContribution {
                key: "Ctrl+L".to_string(),
                title: "Lint".to_string(),
                script: "lint.sh".to_string(),
            }],
            linters: vec![LinterContribution {
                languages: vec!["rs".to_string()],
                script: "lint.sh".to_string(),
            }],
            ..Default::default()
        };

        let badges = collect_badges(&contribs);
        assert!(badges.contains(&"Keybindings"));
        assert!(badges.contains(&"Linter"));
    }

    #[test]
    fn collect_badges_includes_all_supported_contribution_kinds() {
        let contribs = ExtensionContributions {
            status_bar: Some(StatusBarContribution {
                script: "status.sh".to_string(),
            }),
            css: Some(CssContribution {
                file: "theme.css".to_string(),
            }),
            keybindings: vec![KeybindingContribution {
                key: "Ctrl+K".to_string(),
                title: "Run".to_string(),
                script: "run.sh".to_string(),
            }],
            commands: vec![CommandContribution {
                id: "hello".to_string(),
                title: "Hello".to_string(),
                script: "hello.sh".to_string(),
                keywords: vec!["greet".to_string()],
            }],
            context_menus: Some(ContextMenuContributions {
                editor: vec![EditorContextMenu {
                    label: "Format".to_string(),
                    script: "format.sh".to_string(),
                }],
                file_explorer: vec![FileExplorerContextMenu {
                    label: "Reveal".to_string(),
                    script: "reveal.sh".to_string(),
                }],
            }),
            linters: vec![LinterContribution {
                languages: vec!["py".to_string()],
                script: "lint.sh".to_string(),
            }],
            hooks: Some(HooksContribution {
                on_app_start: Some("start.sh".to_string()),
                on_file_open: None,
                on_file_save: None,
                on_file_close: None,
            }),
            text_transforms: vec![TextTransformContribution {
                id: "upper".to_string(),
                title: "Uppercase".to_string(),
                script: "upper.sh".to_string(),
            }],
            sidebar_panels: vec![SidebarPanelContribution {
                id: "panel".to_string(),
                title: "Panel".to_string(),
                icon: "application-x-addon-symbolic".to_string(),
                script: "panel.sh".to_string(),
            }],
        };

        let badges = collect_badges(&contribs);
        for expected in [
            "Status Bar",
            "Theme",
            "Keybindings",
            "Commands",
            "Context Menu",
            "Linter",
            "Hooks",
            "Transforms",
            "Panel",
        ] {
            assert!(badges.contains(&expected), "missing badge: {expected}");
        }
    }

    #[test]
    fn collect_badges_is_empty_for_default_contributions() {
        let badges = collect_badges(&ExtensionContributions::default());
        assert!(badges.is_empty());
    }

    #[test]
    #[serial]
    fn card_matches_query_matches_top_level_label() {
        gtk4::test_synced(|| {
            let card = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            card.append(&gtk4::Label::new(Some("Rust Diagnostics")));

            assert!(card_matches_query(&card, "rust"));
            assert!(!card_matches_query(&card, "python"));
        });
    }

    #[test]
    #[serial]
    fn card_matches_query_matches_nested_label_text() {
        gtk4::test_synced(|| {
            let card = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 4);
            let name_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 2);
            name_box.append(&gtk4::Label::new(Some("Code Completion")));
            row.append(&name_box);
            card.append(&row);

            assert!(card_matches_query(&card, "completion"));
            assert!(!card_matches_query(&card, "diagnostics"));
        });
    }

    #[test]
    #[serial]
    fn card_matches_query_is_case_insensitive() {
        gtk4::test_synced(|| {
            let card = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
            card.append(&gtk4::Label::new(Some("Sample Extension")));

            assert!(card_matches_query(&card, "sample"));
            assert!(card_matches_query(&card, "extension"));
        });
    }
