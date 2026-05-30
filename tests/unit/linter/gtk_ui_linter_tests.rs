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
    fn test_lint_gtk_ui_malformed_xml_reports_parse_error() {
        let broken_ui = r#"<interface>
  <object class="GtkWindow" id="window1">
    <property name="title">Missing close tag
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(broken_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "xml-parse-error"));
    }

    #[test]
    fn test_lint_gtk_ui_invalid_child_outside_object_reports_error() {
        let invalid_ui = r#"<interface>
  <child>
    <object class="GtkLabel" id="label1" />
  </child>
</interface>"#;

        let diagnostics = lint_gtk_ui(invalid_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "invalid-child"));
    }

    #[test]
    fn test_lint_gtk_ui_valid_child_inside_object_is_allowed() {
        let valid_ui = r#"<interface>
  <object class="GtkBox" id="box1">
    <child>
      <object class="GtkLabel" id="label1" />
    </child>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(valid_ui);
        assert!(!diagnostics.iter().any(|d| d.rule == "invalid-child"));
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

    #[test]
    fn test_lint_gtk_ui_reports_duplicate_object_ids() {
        let duplicate_ui = r#"<interface>
  <object class="GtkLabel" id="label1" />
  <object class="GtkButton" id="label1" />
</interface>"#;

        let diagnostics = lint_gtk_ui(duplicate_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "duplicate-id"));
    }

    #[test]
    fn test_lint_gtk_ui_reports_duplicate_id_only_once_per_id() {
        let duplicate_ui = r#"<interface>
  <object class="GtkLabel" id="dup" />
  <object class="GtkButton" id="dup" />
  <object class="GtkEntry" id="dup" />
</interface>"#;

        let diagnostics = lint_gtk_ui(duplicate_ui);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|d| d.rule == "duplicate-id")
                .count(),
            1
        );
    }

    #[test]
    fn test_lint_gtk_ui_warns_on_unknown_widget_class() {
        let unknown_ui = r#"<interface>
  <object class="GtkNotARealWidget" id="widget1" />
</interface>"#;

        let diagnostics = lint_gtk_ui(unknown_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "unknown-widget"));
    }

    #[test]
    fn test_lint_gtk_ui_reports_unknown_property_on_widget() {
        let bad_property_ui = r#"<interface>
  <object class="GtkWindow" id="window1">
    <property name="not-a-real-property">value</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(bad_property_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "unknown-property"));
    }

    #[test]
    fn test_lint_gtk_ui_object_missing_class_reports_error() {
        let missing_class_ui = r#"<interface>
  <object id="window1">
    <property name="title">No class</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(missing_class_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "missing-class"));
    }

    #[test]
    fn test_lint_gtk_ui_template_missing_parent_reports_error() {
        let template_ui = r#"<interface>
  <template class="TemplateWidget" />
</interface>"#;

        let diagnostics = lint_gtk_ui(template_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "missing-parent"));
    }

    #[test]
    fn test_lint_gtk_ui_allows_dvop_prefixed_custom_widgets() {
        let custom_ui = r#"<interface>
  <object class="DvopCustomPanel" id="panel1" />
</interface>"#;

        let diagnostics = lint_gtk_ui(custom_ui);
        assert!(!diagnostics.iter().any(|d| d.rule == "unknown-widget"));
    }

    #[test]
    fn test_lint_gtk_ui_warns_on_deprecated_stock_property_values() {
        let deprecated_ui = r#"<interface>
  <object class="GtkButton" id="button1">
    <property name="stock-id">gtk-open</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(deprecated_ui);
        assert!(diagnostics.iter().any(|d| d.rule == "deprecated-property"));
    }

    #[test]
    fn test_lint_gtk_ui_valid_template_with_class_and_parent_passes() {
        let template_ui = r#"<interface>
  <template class="TemplateWidget" parent="GtkBox">
    <property name="label">Template</property>
  </template>
</interface>"#;

        let diagnostics = lint_gtk_ui(template_ui);
        assert!(!diagnostics.iter().any(|d| d.rule == "missing-class"));
        assert!(!diagnostics.iter().any(|d| d.rule == "missing-parent"));
    }

    #[test]
    fn test_lint_gtk_ui_ignores_non_deprecated_name_property_values() {
        let ui = r#"<interface>
  <object class="GtkButton">
    <property name="label">Click me</property>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(ui);
        assert!(!diagnostics.iter().any(|d| d.rule == "deprecated-property"));
    }

    #[test]
    fn test_lint_gtk_ui_allows_child_with_placeholder_type() {
        let ui = r#"<interface>
  <object class="GtkStack" id="stack1">
    <child type="placeholder">
      <object class="GtkLabel" id="placeholder_label">
        <property name="label">Placeholder</property>
      </object>
    </child>
  </object>
</interface>"#;

        let diagnostics = lint_gtk_ui(ui);
        assert!(!diagnostics.iter().any(|d| d.rule == "invalid-child"));
    }
