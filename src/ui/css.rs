// CSS styling module for the Basado Text Editor
// Contains all CSS styles and application logic

use gtk4;

/// Apply custom CSS to enhance the appearance of tabs
/// 
/// This function creates and applies CSS styles to improve the tab appearance,
/// making them look less flat and more visually distinct.
pub fn apply_custom_css() {
    let provider = gtk4::CssProvider::new();
    
    let css = build_complete_css();
    
    // Load and apply the CSS
    provider.load_from_data(&css);
    
    gtk4::style_context_add_provider_for_display(
        &gtk4::gdk::Display::default().expect("Could not get default display"),
        &provider,
        gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION
    );
}

/// Builds the complete CSS string by combining all component styles
fn build_complete_css() -> String {
    format!(
        "{}{}{}{}",
        get_notebook_tab_styles(),
        get_button_styles(),
        get_status_bar_styles(),
        get_path_navigation_styles()
    )
}

/// Returns CSS styles for notebook tabs and related components
fn get_notebook_tab_styles() -> &'static str {
    "
    /* === NOTEBOOK AND TAB STYLES === */
    
    /* Notebook header container */
    notebook > header {
        padding: 1px;
        margin: 0;
    }
    
    notebook > header > tabs {
        margin: 0;
        padding: 1px;
    }
    
    /* Base tab styling */
    tab {
        padding: 3px 6px;
        min-width: 120px;
        min-height: 26px;
        border-radius: 5px 5px 0 0;
        border-bottom: 3px solid transparent;
        background-color: shade(@theme_bg_color, 0.85);
        box-shadow: 0 -1px 2px -1px shade(@theme_bg_color, 1.1) inset;
        transition: all 0.2s ease;
        margin: 1px 2px 0 2px;
        margin-bottom: -1px;
    }
    
    /* Active/selected tab styling */
    tab:checked {
        background-color: shade(@theme_bg_color, 1.5);
        border-bottom: 3px solid @theme_selected_bg_color;
        box-shadow: 0 -2px 3px -1px shade(@theme_bg_color, 1.2) inset;
    }
    
    /* Tab label styling */
    .tab-label {
        min-width: 80px;
        padding: 1px 3px;
        margin: 0;
        font-size: 95%;
        opacity: 0.85;
    }
    
    tab:checked .tab-label {
        opacity: 1.0;
        font-weight: 500;
    }
    "
}

/// Returns CSS styles for buttons, including circular close buttons
fn get_button_styles() -> String {
    let is_dark_mode = crate::syntax::is_dark_mode_enabled();
    let active_tab_shade = if is_dark_mode { "2" } else { "0.85" };
    
    format!(
        "
    /* === BUTTON STYLES === */
    
    /* Circular button base styling */
    button.circular {{
        background-color: shade(@theme_bg_color, 0.85);
        min-height: 20px;
        min-width: 20px;
        padding: 1px;
        margin: 0;
        border: none;
        border-radius: 50%;
    }}
    
    /* Circular button icon styling */
    button.circular image {{
        background-color: shade(@theme_bg_color, 0.85);
        -gtk-icon-transform: scale(0.8);
        border-radius: 50%;
        min-height: 20px;
        min-width: 20px;
    }}
    
    /* Circular button styling in active tabs */
    tab:checked button.circular,
    tab:checked button.circular image {{
        background-color: shade(@theme_bg_color, {});
        border-radius: 50%;
        min-height: 20px;
        min-width: 20px;
    }}
    ",
        active_tab_shade
    )
}

/// Returns CSS styles for the status bar and path bar
fn get_status_bar_styles() -> &'static str {
    "
    /* === STATUS BAR STYLES === */
    
    .status-bar {
        background-color: shade(@theme_bg_color, 0.97);
        border-top: 1px solid alpha(@theme_fg_color, 0.2);
        min-height: 24px;
    }
    
    .status-text {
        font-size: 0.9em;
        color: @theme_fg_color;
    }
    
    .status-button {
        background: none;
        border: none;
        padding: 2px 6px;
        border-radius: 4px;
    }
    
    .status-button:hover {
        background-color: alpha(@theme_selected_bg_color, 0.1);
    }
    
    .status-button:active {
        background-color: alpha(@theme_selected_bg_color, 0.2);
    }
    
    .status-secondary {
        font-size: 0.8em;
        color: alpha(@theme_fg_color, 0.7);
        font-family: monospace;
    }
    
    /* Log History Popup Styles */
    .log-history-list {
        background-color: @theme_base_color;
    }
    
    .log-history-list > row {
        border-bottom: 1px solid alpha(@theme_fg_color, 0.1);
    }
    
    .log-history-list > row:last-child {
        border-bottom: none;
    }
    
    .log-level-badge {
        font-size: 0.8em;
        font-weight: bold;
        padding: 2px 6px;
        border-radius: 4px;
        font-family: monospace;
    }
    
    .log-level-info {
        background-color: alpha(@theme_selected_bg_color, 0.2);
        color: @theme_fg_color;
    }
    
    .log-level-success {
        background-color: alpha(#27ae60, 0.2);
        color: #27ae60;
    }
    
    .log-level-warning {
        background-color: alpha(#f39c12, 0.2);
        color: #f39c12;
    }
    
    .log-level-error {
        background-color: alpha(#e74c3c, 0.2);
        color: #e74c3c;
    }
    
    .log-timestamp {
        font-size: 0.8em;
        color: alpha(@theme_fg_color, 0.6);
        font-family: monospace;
    }
    
    .log-message {
        font-size: 0.9em;
        color: @theme_fg_color;
        margin-top: 2px;
    }
    
    /* Status message type styling */
    .status-log-info {
        color: @theme_fg_color;
    }
    
    .status-log-success {
        color: @theme_fg_color;
    }
    
    .status-log-warning {
        color: @theme_fg_color;
    }
    
    .status-log-error {
        color: @theme_fg_color;
    }
    
    .basado-status-bar {
        border-top: 1px solid alpha(#999, 0.3);
    }
    
    /* === GLOBAL VOLUME CONTROL STYLES === */
    
    .global-volume-scale {
        min-width: 120px;
        min-height: 20px;
    }
    
    .global-volume-scale > trough {
        min-height: 8px;
        border-radius: 4px;
        background-color: alpha(@theme_fg_color, 0.15);
    }
    
    .global-volume-scale > trough > highlight {
        background-color: @theme_selected_bg_color;
        border-radius: 4px;
    }
    
    .global-volume-scale > trough > slider {
        min-width: 14px;
        min-height: 14px;
        margin: -3px;
        border-radius: 7px;
        background-color: @theme_selected_bg_color;
        border: 1px solid alpha(@theme_fg_color, 0.2);
    }
    
    .global-volume-scale > trough > slider:hover {
        background-color: shade(@theme_selected_bg_color, 1.1);
        box-shadow: 0 0 0 2px alpha(@theme_selected_bg_color, 0.3);
    }
    
    .global-volume-scale > trough > slider:active {
        background-color: shade(@theme_selected_bg_color, 0.9);
        box-shadow: 0 0 0 3px alpha(@theme_selected_bg_color, 0.4);
    }
    
    /* === PATH BAR STYLES === */
    
    .basado-path-bar {
        background-color: shade(@theme_bg_color, 0.98);
    }
    "
}

/// Returns CSS styles for path navigation components
fn get_path_navigation_styles() -> &'static str {
    "
    /* === PATH NAVIGATION STYLES === */
    
    .path-box {
        padding: 2px;
    }
    
    .path-segment-button {
        padding: 2px 4px;
        margin: 0 1px;
        border-radius: 4px;
        min-height: 24px;
        min-width: 24px;
        border: 1px solid transparent;
        transition: all 0.15s ease;
    }
    
    .path-segment-button:hover {
        background-color: alpha(#888, 0.1);
        border-color: alpha(#888, 0.3);
    }
    
    .path-separator {
        opacity: 0.7;
        margin: 0 1px;
        font-family: monospace;
    }
    
    /* === PATH INPUT ENTRY STYLES === */
    
    entry.path-input {
        margin: 1px 0 1px 1px;
        padding: 2px 6px;
        border-radius: 4px 0 0 4px;
        border-right: none;
        /* Ensure the entry takes all available space */
        min-width: 0;
    }
    
    entry.path-input.error {
        border-color: #e74c3c;
        background-color: alpha(#e74c3c, 0.1);
    }
    
    /* === PATH INPUT CLOSE BUTTON STYLES === */
    
    button.path-input-close {
        border-left: none;
        border-top: 1px solid alpha(@theme_fg_color, 0.3);
        border-right: 1px solid alpha(@theme_fg_color, 0.3);
        border-bottom: 1px solid alpha(@theme_fg_color, 0.3);
        border-radius: 0 6px 6px 0;
        margin: 1px 1px 1px 0;
        padding: 2px 4px;
        min-width: 20px;
        min-height: 24px;
        max-height: 24px;
        background-color: shade(@theme_bg_color, 0.95);
        transition: all 0.15s ease;
    }
    
    button.path-input-close:hover {
        background-color: alpha(@theme_selected_bg_color, 0.1);
        border-color: alpha(@theme_selected_bg_color, 0.5);
    }
    
    button.path-input-close:active {
        background-color: alpha(@theme_selected_bg_color, 0.2);
        border-color: @theme_selected_bg_color;
    }

    /* === NAVIGATION SECTION STYLES === */
    
    .file-manager-panel {
        background-color: transparent;
    }
    
    .nav-buttons-section {
        background-color: @view_bg_color;
    }
    
    .file-manager-panel listbox {
        background-color: @view_bg_color;
    }
    
    /* === FILE SELECTION STYLING === */
    
    /* File selected by tab switch - subtle highlight */
    .file-selected-by-tab {
        border: 2px solid @theme_selected_bg_color;
        border-left: 4px solid @theme_selected_bg_color;
        background-color: alpha(@theme_selected_bg_color, 0);
        border-radius: 0 6px 6px 0;
        margin-left: 2px;
        margin-right: 4px;
        transition: all 0.2s ease;
        color: @theme_fg_color;
    }
    
    /* File selected by direct click - more prominent highlight with icon */
    .file-selected-by-click {
        background-color: alpha(@theme_selected_bg_color, 1);
        border-left: 4px solid @theme_selected_bg_color;
        border-right: 2px solid alpha(@theme_selected_bg_color, 1);
        border-top: 2px solid alpha(@theme_selected_bg_color, 1);
        border-bottom: 2px solid alpha(@theme_selected_bg_color, 1);
        border-radius: 0 6px 6px 0;
        margin-left: 2px;
        margin-right: 4px;
        box-shadow: 0 2px 4px alpha(#000, 0.15);
    }

    /* === PATH INPUT CONTAINER STYLES === */
    
    .path-input-container {
        margin: 0;
        padding: 0;
    }
    
    .path-input-container entry {
        min-height: 24px;
        max-height: 24px;
    }
    
    .path-input-container button {
        min-height: 24px;
        max-height: 24px;
    }
    "
}
