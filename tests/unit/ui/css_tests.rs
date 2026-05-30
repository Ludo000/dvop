    use super::*;

    #[test]
    fn complete_css_contains_all_component_sections() {
        let css = build_complete_css();

        for section in [
            "NOTEBOOK AND TAB STYLES",
            "BUTTON STYLES",
            "STATUS BAR STYLES",
            "PATH NAVIGATION STYLES",
            "DRAG AND DROP STYLES",
            "FILE OPERATION STYLES",
            "LIST STYLES",
            "ACTIVITY BAR STYLES",
            "SEARCH UI STYLES",
            "DIAGNOSTICS PANEL STYLES",
            "EXTENSIONS PANEL STYLES",
        ] {
            assert!(css.contains(section), "missing CSS section: {section}");
        }
    }

    #[test]
    fn complete_css_includes_runtime_widget_classes() {
        let css = build_complete_css();

        for class_name in [
            ".tab-label",
            ".global-volume-scale",
            ".path-drop-target",
            ".file-selected-by-tab",
            ".file-cut",
            ".zebra-list",
            ".activity-bar-button",
            ".case-toggle-button",
            ".diagnostic-error",
            ".extension-card",
        ] {
            assert!(css.contains(class_name), "missing CSS class: {class_name}");
        }
    }

    #[test]
    fn button_styles_include_theme_specific_active_tab_rule() {
        let css = get_button_styles();

        assert!(css.contains("tab:checked button.circular"));
        assert!(
            css.contains("shade(@theme_bg_color, 2)")
                || css.contains("shade(@theme_bg_color, 0.85)")
        );
    }

    #[test]
    fn drop_target_styles_define_matching_animations() {
        let css = get_drag_drop_styles();

        assert!(css.contains(".drop-target"));
        assert!(css.contains(".drop-target-background"));
        assert!(css.contains("@keyframes drop-target-pulse"));
        assert!(css.contains("@keyframes drop-target-pulse-bg"));
    }

    #[test]
    fn diagnostics_styles_cover_all_severities() {
        let css = get_diagnostics_styles();

        assert!(css.contains(".diagnostic-error"));
        assert!(css.contains(".diagnostic-warning"));
        assert!(css.contains(".diagnostic-info"));
        assert!(css.contains("@media (prefers-color-scheme: light)"));
    }

    #[test]
    fn status_bar_and_path_styles_include_core_selectors() {
        let status_css = get_status_bar_styles();
        let path_css = get_path_navigation_styles();

        assert!(status_css.contains(".status-bar"));
        assert!(path_css.contains(".path-segment-button"));
        assert!(path_css.contains(".path-separator"));
    }

    #[test]
    fn search_and_git_panel_styles_include_panel_selectors() {
        let search_css = get_search_styles();
        let git_css = get_git_panel_styles();

        assert!(search_css.contains(".case-toggle-button"));
        assert!(git_css.contains(".git-diff-panel"));
        assert!(git_css.contains(".git-file-list"));
    }

    #[test]
    fn extension_styles_define_card_layout_classes() {
        let css = get_extension_styles();
        assert!(css.contains(".extension-card"));
        assert!(css.contains(".extension-name"));
    }

    #[test]
    fn activity_bar_and_list_styles_include_runtime_classes() {
        let activity_css = get_activity_bar_styles();
        let list_css = get_list_styles();
        let file_ops_css = get_file_operation_styles();

        assert!(activity_css.contains(".activity-bar-button"));
        assert!(list_css.contains(".zebra-list"));
        assert!(file_ops_css.contains(".file-cut"));
    }

    #[test]
    fn notebook_tab_styles_include_tab_checked_selector() {
        let css = get_notebook_tab_styles();
        assert!(css.contains("tab:checked"));
        assert!(css.contains(".tab-label"));
    }

    #[test]
    fn drag_drop_styles_define_target_selectors() {
        let css = get_drag_drop_styles();
        assert!(css.contains(".drop-target"));
        assert!(css.contains("@keyframes drop-target-pulse"));
    }

    #[test]
    fn complete_css_includes_git_panel_section() {
        let css = build_complete_css();
        assert!(css.contains("SOURCE CONTROL PANEL STYLES"));
    }

    #[test]
    fn complete_css_includes_terminal_panel_section() {
        let css = build_complete_css();
        assert!(css.contains("GLOBAL VOLUME CONTROL STYLES"));
    }
