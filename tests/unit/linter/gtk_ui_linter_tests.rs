    use super::*;

    #[test]
    fn test_lint_gtk_ui_valid() {
        let valid_ui = r#"<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkWindow" id="window1">
    <property name="title">Test Window</property>
    <property name="default-width">800</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(valid_ui);
        // Valid UI might still have some warnings, just check it doesn't panic
        // (diagnostics length is always >= 0 by definition, so just check it exists)
        drop(diagnostics);
    }

    #[test]
    fn test_lint_gtk_ui_duplicate_id() {
        let invalid_ui = r#"<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkWindow" id="window1">
    <property name="title">Test</property>
  </object>
  <object class="GtkButton" id="window1">
    <property name="label">Button</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        // Should detect duplicate ID
        assert!(diagnostics.iter().any(|d| d.message.contains("Duplicate")));
    }

    #[test]
    fn test_lint_gtk_ui_unknown_widget() {
        let invalid_ui = r#"<?xml version="1.0" encoding="UTF-8"?>
<interface>
  <object class="GtkNonExistentWidget" id="widget1">
    <property name="title">Test</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        // Should detect unknown widget
        assert!(diagnostics.iter().any(|d| d.message.contains("Unknown") || d.message.contains("widget")));
    }

    #[test]
    fn test_lint_gtk_ui_missing_interface() {
        let invalid_ui = r#"<?xml version="1.0" encoding="UTF-8"?>
<object class="GtkWindow" id="window1">
  <property name="title">Test</property>
</object>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        // Should warn about missing interface root
        assert!(diagnostics.iter().any(|d| d.message.contains("interface")));
    }

    #[test]
    fn test_lint_gtk_ui_missing_object_class() {
        let invalid_ui = r#"<interface>
  <object id="window1">
    <property name="title">Test</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "missing-class" && d.severity == DiagnosticSeverity::Error));
    }

    #[test]
    fn test_lint_gtk_ui_template_requires_class_and_parent() {
        let invalid_ui = r#"<interface>
  <template>
    <property name="title">Test</property>
  </template>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "missing-class"));
        assert!(diagnostics.iter().any(|d| d.rule == "missing-parent"));
    }

    #[test]
    fn test_lint_gtk_ui_unknown_property() {
        let invalid_ui = r#"<interface>
  <object class="GtkButton" id="button1">
    <property name="definitely-not-a-button-property">Test</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "unknown-property" && d.message.contains("GtkButton")));
    }

    #[test]
    fn test_lint_gtk_ui_deprecated_gtk3_property() {
        let invalid_ui = r#"<interface>
  <object class="GtkButton" id="button1">
    <property name="stock">gtk-open</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        assert!(diagnostics
            .iter()
            .any(|d| d.rule == "deprecated-property" && d.message.contains("icon-name")));
    }

    #[test]
    fn test_lint_gtk_ui_empty() {
        let empty_ui = "";
        let diagnostics = lint_gtk_ui(empty_ui);
        // Should handle empty input without panicking
        drop(diagnostics);
    }

    #[test]
    fn test_get_known_gtk4_widgets() {
        let widgets = get_known_gtk4_widgets();
        assert!(widgets.contains("GtkWindow"));
        assert!(widgets.contains("GtkButton"));
        assert!(widgets.contains("GtkLabel"));
        assert!(widgets.contains("GtkBox"));
        assert!(!widgets.is_empty());
    }

    #[test]
    fn test_get_known_gtk4_properties() {
        let properties = get_known_gtk4_properties();
        
        // Check GtkWindow properties
        if let Some(window_props) = properties.get("GtkWindow") {
            assert!(window_props.contains("title"));
            assert!(window_props.contains("default-width"));
        }
        
        // Check GtkButton properties
        if let Some(button_props) = properties.get("GtkButton") {
            assert!(button_props.contains("label"));
        }
        
        assert!(!properties.is_empty());
    }
