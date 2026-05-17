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
