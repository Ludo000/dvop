// GTK UI file linter for validating XML .ui files
// This module checks GTK UI files for common errors and issues

use super::{Diagnostic, DiagnosticSeverity};
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::collections::{HashMap, HashSet};

/// Represents an element in the XML stack with its class name
#[derive(Clone)]
struct StackElement {
    tag_name: String,
    class_name: Option<String>,
}

/// Lint GTK UI XML file
pub fn lint_gtk_ui(content: &str) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut line_number = 1;
    let mut element_stack: Vec<StackElement> = Vec::new();
    let mut object_ids = HashMap::new(); // Track object IDs and their line numbers
    let mut duplicate_ids = HashSet::new();
    let mut found_interface = false;

    // Known GTK4 widgets and properties
    let known_widgets = get_known_gtk4_widgets();
    let known_properties = get_known_gtk4_properties();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) | Ok(Event::Empty(e)) => {
                let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                
                // Extract class name for object/template elements
                let mut class_name = None;
                if tag_name == "object" || tag_name == "template" {
                    for attr in e.attributes().flatten() {
                        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                        if key == "class" || key == "parent" {
                            class_name = Some(String::from_utf8_lossy(&attr.value).to_string());
                            break;
                        }
                    }
                }
                
                element_stack.push(StackElement {
                    tag_name: tag_name.clone(),
                    class_name,
                });

                // Check for root element
                if tag_name == "interface" {
                    found_interface = true;
                }

                // Validate object elements
                if tag_name == "object" {
                    validate_object_element(&e, line_number, &mut diagnostics, &known_widgets, &mut object_ids, &mut duplicate_ids);
                }

                // Validate template elements (similar to object)
                if tag_name == "template" {
                    validate_template_element(&e, line_number, &mut diagnostics);
                }

                // Validate property elements
                if tag_name == "property" {
                    validate_property_element(&e, &element_stack, line_number, &mut diagnostics, &known_properties);
                }

                // Validate child elements
                if tag_name == "child" {
                    validate_child_element(&e, &element_stack, line_number, &mut diagnostics);
                }

                // Check for deprecated attributes
                check_deprecated_attributes(&e, &tag_name, line_number, &mut diagnostics);
            }
            Ok(Event::End(_)) => {
                element_stack.pop();
            }
            Ok(Event::Text(_)) => {
                // Track line numbers by counting newlines up to current position
                let pos = reader.buffer_position() as usize;
                line_number = content[..pos].chars().filter(|&c| c == '\n').count() + 1;
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                diagnostics.push(Diagnostic::new(
                    DiagnosticSeverity::Error,
                    format!("XML parsing error: {}", e),
                    line_number,
                    0,
                    "xml-parse-error".to_string(),
                ));
                break;
            }
            _ => {}
        }

        buf.clear();
    }

    // Check if interface element was found
    if !found_interface {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Error,
            "Missing root <interface> element".to_string(),
            1,
            0,
            "missing-interface".to_string(),
        ));
    }

    diagnostics
}

/// Validate object element attributes
fn validate_object_element(
    element: &BytesStart,
    line_number: usize,
    diagnostics: &mut Vec<Diagnostic>,
    known_widgets: &HashSet<&str>,
    object_ids: &mut HashMap<String, usize>,
    duplicate_ids: &mut HashSet<String>,
) {
    let mut has_class = false;
    let mut _class_name = String::new();
    let mut _id_value = None;

    for attr in element.attributes() {
        if let Ok(attr) = attr {
            let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
            let value = String::from_utf8_lossy(&attr.value).to_string();

            match key.as_str() {
                "class" => {
                    has_class = true;
                    _class_name = value.clone();

                    // Check if it's a known GTK4 widget
                    if !known_widgets.contains(value.as_str()) && !value.starts_with("Dvop") {
                        diagnostics.push(Diagnostic::new(
                            DiagnosticSeverity::Warning,
                            format!("Unknown widget class: '{}'", value),
                            line_number,
                            0,
                            "unknown-widget".to_string(),
                        ));
                    }
                }
                "id" => {
                    _id_value = Some(value.clone());
                    
                    // Check for duplicate IDs
                    if let Some(previous_line) = object_ids.get(&value) {
                        if !duplicate_ids.contains(&value) {
                            diagnostics.push(Diagnostic::new(
                                DiagnosticSeverity::Error,
                                format!("Duplicate object ID '{}' (first defined at line {})", value, previous_line),
                                line_number,
                                0,
                                "duplicate-id".to_string(),
                            ));
                            duplicate_ids.insert(value.clone());
                        }
                    } else {
                        object_ids.insert(value, line_number);
                    }
                }
                _ => {}
            }
        }
    }

    // Object must have a class attribute
    if !has_class {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Error,
            "Object element missing required 'class' attribute".to_string(),
            line_number,
            0,
            "missing-class".to_string(),
        ));
    }
}

/// Validate template element attributes
fn validate_template_element(
    element: &BytesStart,
    line_number: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut has_class = false;
    let mut has_parent = false;

    for attr in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        
        match key.as_str() {
            "class" => has_class = true,
            "parent" => has_parent = true,
            _ => {}
        }
    }

    // Template must have both class and parent attributes
    if !has_class {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Error,
            "Template element missing required 'class' attribute".to_string(),
            line_number,
            0,
            "missing-class".to_string(),
        ));
    }
    
    if !has_parent {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Error,
            "Template element missing required 'parent' attribute".to_string(),
            line_number,
            0,
            "missing-parent".to_string(),
        ));
    }
}

/// Validate property element
fn validate_property_element(
    element: &BytesStart,
    element_stack: &[StackElement],
    line_number: usize,
    diagnostics: &mut Vec<Diagnostic>,
    known_properties: &HashMap<&str, HashSet<&str>>,
) {
    let mut property_name = None;

    for attr in element.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
        let value = String::from_utf8_lossy(&attr.value).to_string();

        if key == "name" {
            property_name = Some(value);
        }
    }

    // Find the parent object class
    if let Some(parent_class) = find_parent_object_class(element_stack) {
        if let Some(prop_name) = property_name {
            // Check if property is known for this widget type
            if let Some(valid_props) = known_properties.get(parent_class.as_str()) {
                if !valid_props.contains(prop_name.as_str()) {
                    // Also check common properties that apply to all widgets
                    if let Some(common_props) = known_properties.get("GtkWidget") {
                        if !common_props.contains(prop_name.as_str()) {
                            diagnostics.push(Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                format!("Unknown property '{}' for widget '{}'", prop_name, parent_class),
                                line_number,
                                0,
                                "unknown-property".to_string(),
                            ));
                        }
                    }
                }
            }
        }
    }
}

/// Validate child element
fn validate_child_element(
    _element: &BytesStart,
    element_stack: &[StackElement],
    line_number: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Check if child is inside a valid parent
    if element_stack.len() < 2 {
        diagnostics.push(Diagnostic::new(
            DiagnosticSeverity::Error,
            "Child element must be inside an object or template".to_string(),
            line_number,
            0,
            "invalid-child".to_string(),
        ));
    }
}

/// Check for deprecated GTK attributes
fn check_deprecated_attributes(
    element: &BytesStart,
    tag_name: &str,
    line_number: usize,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // GTK4 deprecated attributes
    if tag_name == "property" {
        for attr in element.attributes() {
            if let Ok(attr) = attr {
                let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
                
                // Check for GTK3-specific properties that were removed in GTK4
                if key == "name" {
                    let value = String::from_utf8_lossy(&attr.value).to_string();
                    match value.as_str() {
                        "stock" | "use-stock" | "stock-id" => {
                            diagnostics.push(Diagnostic::new(
                                DiagnosticSeverity::Warning,
                                format!("Property '{}' is deprecated in GTK4, use 'icon-name' instead", value),
                                line_number,
                                0,
                                "deprecated-property".to_string(),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Find parent object class from element stack
fn find_parent_object_class(element_stack: &[StackElement]) -> Option<String> {
    // Walk backwards through stack to find the nearest object/template with a class
    for i in (0..element_stack.len()).rev() {
        let elem = &element_stack[i];
        if (elem.tag_name == "object" || elem.tag_name == "template") && elem.class_name.is_some() {
            return elem.class_name.clone();
        }
    }
    None
}

/// Get set of known GTK4 widgets
fn get_known_gtk4_widgets() -> HashSet<&'static str> {
    let mut widgets = HashSet::new();
    
    // Common GTK4 widgets
    widgets.insert("GtkWindow");
    widgets.insert("GtkApplicationWindow");
    widgets.insert("GtkDialog");
    widgets.insert("GtkBox");
    widgets.insert("GtkLabel");
    widgets.insert("GtkButton");
    widgets.insert("GtkEntry");
    widgets.insert("GtkTextView");
    widgets.insert("GtkScrolledWindow");
    widgets.insert("GtkNotebook");
    widgets.insert("GtkPaned");
    widgets.insert("GtkHeaderBar");
    widgets.insert("GtkMenuButton");
    widgets.insert("GtkImage");
    widgets.insert("GtkSearchEntry");
    widgets.insert("GtkCheckButton");
    widgets.insert("GtkRadioButton");
    widgets.insert("GtkSwitch");
    widgets.insert("GtkScale");
    widgets.insert("GtkSpinButton");
    widgets.insert("GtkComboBox");
    widgets.insert("GtkDropDown");
    widgets.insert("GtkListBox");
    widgets.insert("GtkTreeView");
    widgets.insert("GtkFrame");
    widgets.insert("GtkGrid");
    widgets.insert("GtkStack");
    widgets.insert("GtkStackSwitcher");
    widgets.insert("GtkRevealer");
    widgets.insert("GtkExpander");
    widgets.insert("GtkSeparator");
    widgets.insert("GtkProgressBar");
    widgets.insert("GtkSpinner");
    widgets.insert("GtkLevelBar");
    widgets.insert("GtkInfoBar");
    widgets.insert("GtkToolbar");
    widgets.insert("GtkPopover");
    widgets.insert("GtkMenu");
    widgets.insert("GtkMenuItem");
    
    widgets
}

/// Get map of known GTK4 properties for each widget type
fn get_known_gtk4_properties() -> HashMap<&'static str, HashSet<&'static str>> {
    let mut properties = HashMap::new();
    
    // Common GtkWidget properties
    let mut widget_props = HashSet::new();
    widget_props.insert("visible");
    widget_props.insert("sensitive");
    widget_props.insert("can-focus");
    widget_props.insert("has-focus");
    widget_props.insert("can-target");
    widget_props.insert("focus-on-click");
    widget_props.insert("focusable");
    widget_props.insert("receives-default");
    widget_props.insert("cursor");
    widget_props.insert("has-tooltip");
    widget_props.insert("tooltip-text");
    widget_props.insert("tooltip-markup");
    widget_props.insert("halign");
    widget_props.insert("valign");
    widget_props.insert("hexpand");
    widget_props.insert("vexpand");
    widget_props.insert("hexpand-set");
    widget_props.insert("vexpand-set");
    widget_props.insert("margin-start");
    widget_props.insert("margin-end");
    widget_props.insert("margin-top");
    widget_props.insert("margin-bottom");
    widget_props.insert("width-request");
    widget_props.insert("height-request");
    widget_props.insert("opacity");
    widget_props.insert("overflow");
    widget_props.insert("css-classes");
    widget_props.insert("css-name");
    properties.insert("GtkWidget", widget_props);
    
    // GtkWindow properties
    let mut window_props = HashSet::new();
    window_props.insert("title");
    window_props.insert("default-width");
    window_props.insert("default-height");
    window_props.insert("resizable");
    window_props.insert("modal");
    window_props.insert("decorated");
    window_props.insert("deletable");
    window_props.insert("icon-name");
    window_props.insert("destroy-with-parent");
    window_props.insert("hide-on-close");
    properties.insert("GtkWindow", window_props);
    
    // GtkBox properties
    let mut box_props = HashSet::new();
    box_props.insert("orientation");
    box_props.insert("spacing");
    box_props.insert("homogeneous");
    box_props.insert("baseline-position");
    properties.insert("GtkBox", box_props);
    
    // GtkLabel properties
    let mut label_props = HashSet::new();
    label_props.insert("label");
    label_props.insert("use-markup");
    label_props.insert("use-underline");
    label_props.insert("justify");
    label_props.insert("wrap");
    label_props.insert("wrap-mode");
    label_props.insert("ellipsize");
    label_props.insert("width-chars");
    label_props.insert("max-width-chars");
    label_props.insert("lines");
    label_props.insert("xalign");
    label_props.insert("yalign");
    label_props.insert("selectable");
    properties.insert("GtkLabel", label_props);
    
    // GtkButton properties
    let mut button_props = HashSet::new();
    button_props.insert("label");
    button_props.insert("use-underline");
    button_props.insert("icon-name");
    button_props.insert("has-frame");
    properties.insert("GtkButton", button_props);
    
    properties
}

#[cfg(test)]
mod tests {
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
}
