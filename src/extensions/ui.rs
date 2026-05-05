//! # Extensions Panel UI — Card-Based Extension Manager
//!
//! Renders the "Extensions" sidebar panel with:
//! - A searchable list of extension cards (icon, name, description, toggle)
//! - A detail view with tabs (Overview, Contributions, Controls)
//! - An "Install from file" dialog accepting `.tar.gz` archives
//! - A "Disable all" button
//!
//! The panel is rebuilt from scratch each time `populate_extensions_panel()`
//! is called (after install/remove/toggle operations). Each card includes
//! a `Switch` widget for enable/disable and badges showing the extension’s
//! contributions (linter, keybindings, commands, etc.).
//!
//! See FEATURES.md: Feature #90 — Extensions Panel
//! See FEATURES.md: Feature #88 — Extension Install from Archive

use gtk4::prelude::*;
use gtk4::{self, Label, Switch};
use super::Extension;

/// Populates the extensions panel (the GtkBox from the sidebar stack) with extension cards.
pub fn populate_extensions_panel(panel: &gtk4::Box) {
    // Clear existing children
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    panel.set_orientation(gtk4::Orientation::Vertical);
    panel.set_spacing(0);
    panel.set_margin_start(0);
    panel.set_margin_end(0);

    // Header row: title + install button
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.set_margin_start(12);
    header.set_margin_end(8);
    header.set_margin_top(8);
    header.set_margin_bottom(4);

    let title_label = Label::new(Some("EXTENSIONS"));
    title_label.add_css_class("extension-panel-title");
    title_label.set_halign(gtk4::Align::Start);
    title_label.set_hexpand(true);
    header.append(&title_label);

    // Install from tar.gz button
    let install_button = gtk4::Button::new();
    install_button.set_icon_name("list-add-symbolic");
    install_button.set_tooltip_text(Some("Install extension from .tar.gz"));
    install_button.add_css_class("flat");
    let panel_weak = glib::object::WeakRef::new();
    panel_weak.set(Some(panel));
    // The "move" keyword forces the closure to take ownership of the variables it uses.
    install_button.connect_clicked(move |btn| {
        let panel_ref = panel_weak.clone();
        show_install_dialog(btn, panel_ref);
    });
    header.append(&install_button);

    // Disable All button
    let disable_all_button = gtk4::Button::new();
    disable_all_button.set_icon_name("action-unavailable-symbolic");
    disable_all_button.set_tooltip_text(Some("Disable all extensions"));
    disable_all_button.add_css_class("flat");
    let panel_weak2 = glib::object::WeakRef::new();
    panel_weak2.set(Some(panel));
    // The "move" keyword forces the closure to take ownership of the variables it uses.
    disable_all_button.connect_clicked(move |_| {
        disable_all_extensions();
        if let Some(panel) = panel_weak2.upgrade() {
            populate_extensions_panel(&panel);
        }
    });
    header.append(&disable_all_button);

    panel.append(&header);

    // Search entry
    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search extensions..."));
    search_entry.set_margin_start(8);
    search_entry.set_margin_end(8);
    search_entry.set_margin_top(4);
    search_entry.set_margin_bottom(8);
    panel.append(&search_entry);

    // Scrollable list of extension cards
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_vexpand(true);
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);

    // Box::new(...) allocates the data on the heap rather than the stack.
    let list_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    list_box.add_css_class("extension-list");

    // Get extensions from manager (includes both script and native extensions)
    let mgr = super::manager::get_manager();
    let extensions: Vec<_> = mgr.get_all_extensions();
    drop(mgr);

    if extensions.is_empty() {
        let empty_label = Label::new(Some("No extensions installed"));
        empty_label.add_css_class("dim-label");
        empty_label.set_margin_top(24);
        empty_label.set_margin_bottom(24);
        list_box.append(&empty_label);
    } else {
        for ext in &extensions {
            let card = build_extension_card(ext, panel);
            list_box.append(&card);
        }
    }

    scrolled.set_child(Some(&list_box));
    panel.append(&scrolled);

    // Wire search filtering
    let list_box_clone = list_box.clone();
    // The "move" keyword forces the closure to take ownership of the variables it uses.
    search_entry.connect_search_changed(move |entry| {
        let query = entry.text().to_lowercase();
        let mut child = list_box_clone.first_child();
        while let Some(widget) = child {
            if let Some(card) = widget.downcast_ref::<gtk4::Box>() {
                if query.is_empty() {
                    card.set_visible(true);
                } else {
                    let visible = card_matches_query(card, &query);
                    card.set_visible(visible);
                }
            }
            child = widget.next_sibling();
        }
    });
}

/// Disable all extensions (both script-based and native)
fn disable_all_extensions() {
    let mut mgr = super::manager::get_manager();
    let all_ids: Vec<String> = mgr.get_all_extensions().iter().map(|e| e.manifest.id.clone()).collect();
    for id in &all_ids {
        mgr.set_enabled(id, false);
    }
    drop(mgr);
    // Fire refresh hooks so running extensions shut down
    for id in &all_ids {
        super::hooks::refresh_extension(id, false);
    }
    crate::status_log::log_info("All extensions disabled");
}

/// Opens a file chooser dialog for installing a .tar.gz extension
fn show_install_dialog(
    btn: &gtk4::Button,
    panel_ref: glib::object::WeakRef<gtk4::Box>,
) {
    let window = btn
        .root()
        .and_then(|r| r.downcast::<gtk4::Window>().ok());

    let dialog = gtk4::FileChooserDialog::new(
        Some("Install Extension"),
        window.as_ref(),
        gtk4::FileChooserAction::Open,
        &[
            ("Cancel", gtk4::ResponseType::Cancel),
            ("Install", gtk4::ResponseType::Accept),
        ],
    );

    // Filter for tar.gz files
    let filter = gtk4::FileFilter::new();
    filter.set_name(Some("Extension archives (*.tar.gz)"));
    filter.add_pattern("*.tar.gz");
    dialog.add_filter(&filter);

    // The "move" keyword forces the closure to take ownership of the variables it uses.
    dialog.connect_response(move |dialog, response| {
        if response == gtk4::ResponseType::Accept {
            if let Some(file) = dialog.file() {
                if let Some(path) = file.path() {
                    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
                    match super::manager::install_from_archive(&path) {
                        Ok(name) => {
                            crate::status_log::log_success(&format!(
                                "Installed extension: {}",
                                name
                            ));
                            // Refresh panel
                            if let Some(panel) = panel_ref.upgrade() {
                                populate_extensions_panel(&panel);
                            }
                        }
                        Err(e) => {
                            crate::status_log::log_error(&format!(
                                "Failed to install extension: {}",
                                e
                            ));
                        }
                    }
                }
            }
        }
        dialog.close();
    });

    dialog.show();
}

/// Checks if an extension card matches the search query
fn card_matches_query(card: &gtk4::Box, query: &str) -> bool {
    let mut child = card.first_child();
    while let Some(widget) = child {
        if let Some(label) = widget.downcast_ref::<Label>() {
            if label.text().to_lowercase().contains(query) {
                return true;
            }
        }
        if let Some(inner_box) = widget.downcast_ref::<gtk4::Box>() {
            let mut inner_child = inner_box.first_child();
            while let Some(inner_widget) = inner_child {
                if let Some(label) = inner_widget.downcast_ref::<Label>() {
                    if label.text().to_lowercase().contains(query) {
                        return true;
                    }
                }
                // One more level (name_box inside top_row)
                if let Some(deep_box) = inner_widget.downcast_ref::<gtk4::Box>() {
                    let mut deep_child = deep_box.first_child();
                    while let Some(deep_widget) = deep_child {
                        if let Some(label) = deep_widget.downcast_ref::<Label>() {
                            if label.text().to_lowercase().contains(query) {
                                return true;
                            }
                        }
                        deep_child = deep_widget.next_sibling();
                    }
                }
                inner_child = inner_widget.next_sibling();
            }
        }
        child = widget.next_sibling();
    }
    false
}

/// Builds a single extension card widget
fn build_extension_card(ext: &Extension, panel: &gtk4::Box) -> gtk4::Box {
    // Box::new(...) allocates the data on the heap rather than the stack.
    let card = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    card.add_css_class("extension-card");
    card.set_margin_start(8);
    card.set_margin_end(8);
    card.set_margin_top(4);
    card.set_margin_bottom(4);

    // Top row: icon + name/meta + chevron
    let top_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    top_row.set_valign(gtk4::Align::Center);

    let icon = gtk4::Image::from_icon_name("application-x-addon-symbolic");
    icon.set_pixel_size(24);
    icon.add_css_class("extension-icon");
    top_row.append(&icon);

    // Box::new(...) allocates the data on the heap rather than the stack.
    let name_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    name_box.set_hexpand(true);

    let name_label = Label::new(Some(&ext.manifest.name));
    name_label.add_css_class("extension-name");
    name_label.set_halign(gtk4::Align::Start);
    name_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_box.append(&name_label);

    let meta_label = Label::new(Some(&format!(
        "v{} — {}",
        ext.manifest.version, ext.manifest.author
    )));
    meta_label.add_css_class("extension-meta");
    meta_label.set_halign(gtk4::Align::Start);
    meta_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    name_box.append(&meta_label);

    top_row.append(&name_box);

    // Enabled indicator dot
    if !ext.manifest.enabled {
        let disabled_label = Label::new(Some("off"));
        disabled_label.add_css_class("ext-disabled-indicator");
        top_row.append(&disabled_label);
    }

    let chevron = gtk4::Image::from_icon_name("go-next-symbolic");
    chevron.set_pixel_size(14);
    chevron.set_opacity(0.4);
    top_row.append(&chevron);

    card.append(&top_row);

    // Description
    let desc_label = Label::new(Some(&ext.manifest.description));
    desc_label.add_css_class("extension-description");
    desc_label.set_halign(gtk4::Align::Start);
    desc_label.set_wrap(true);
    desc_label.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    desc_label.set_max_width_chars(35);
    card.append(&desc_label);

    // Contribution badges
    let badges = collect_badges(&ext.manifest.contributions);
    if !badges.is_empty() {
        let badge_flow = gtk4::FlowBox::new();
        badge_flow.set_selection_mode(gtk4::SelectionMode::None);
        badge_flow.set_max_children_per_line(5);
        badge_flow.set_margin_top(4);
        badge_flow.set_column_spacing(4);
        badge_flow.set_row_spacing(2);

        for badge_text in &badges {
            let badge = Label::new(Some(badge_text));
            badge.add_css_class("extension-badge");
            badge.set_margin_start(2);
            badge.set_margin_end(2);
            badge_flow.insert(&badge, -1);
        }
        card.append(&badge_flow);
    }

    // Click gesture to open detail view
    let gesture = gtk4::GestureClick::new();
    let ext_id = ext.manifest.id.clone();
    let panel_weak = glib::object::WeakRef::new();
    panel_weak.set(Some(panel));
    gesture.connect_released(move |_, _, _, _| {
        if let Some(panel) = panel_weak.upgrade() {
            show_extension_detail(&panel, &ext_id);
        }
    });
    card.add_controller(gesture);
    card.set_cursor_from_name(Some("pointer"));

    card
}

/// Collect contribution badge labels
fn collect_badges(contribs: &super::ExtensionContributions) -> Vec<&'static str> {
    let mut badges: Vec<&str> = Vec::new();
    if contribs.status_bar.is_some() {
        badges.push("Status Bar");
    }
    if contribs.css.is_some() {
        badges.push("Theme");
    }
    if !contribs.keybindings.is_empty() {
        badges.push("Keybindings");
    }
    if !contribs.commands.is_empty() {
        badges.push("Commands");
    }
    if contribs.context_menus.is_some() {
        badges.push("Context Menu");
    }
    if !contribs.linters.is_empty() {
        badges.push("Linter");
    }
    if contribs.hooks.is_some() {
        badges.push("Hooks");
    }
    if !contribs.text_transforms.is_empty() {
        badges.push("Transforms");
    }
    if !contribs.sidebar_panels.is_empty() {
        badges.push("Panel");
    }
    badges
}

// ── Detail view ─────────────────────────────────────────────────

/// Show the detail view for a single extension with vertical tabs
fn show_extension_detail(panel: &gtk4::Box, ext_id: &str) {
    let mgr = super::manager::get_manager();
    // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
    let ext = match mgr.get_all_extensions().iter().find(|e| e.manifest.id == ext_id) {
        Some(e) => e.clone(),
        None => return,
    };
    drop(mgr);

    // Clear panel and rebuild with detail view
    while let Some(child) = panel.first_child() {
        panel.remove(&child);
    }

    // Back button header
    let header = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    header.set_margin_start(8);
    header.set_margin_end(8);
    header.set_margin_top(6);
    header.set_margin_bottom(2);

    let back_btn = gtk4::Button::new();
    back_btn.set_icon_name("go-previous-symbolic");
    back_btn.add_css_class("flat");
    back_btn.set_tooltip_text(Some("Back to extensions list"));
    let panel_weak = glib::object::WeakRef::new();
    panel_weak.set(Some(panel));
    back_btn.connect_clicked(move |_| {
        if let Some(panel) = panel_weak.upgrade() {
            populate_extensions_panel(&panel);
        }
    });
    header.append(&back_btn);

    let title = Label::new(Some(&ext.manifest.name));
    title.add_css_class("extension-detail-title");
    title.set_hexpand(true);
    title.set_halign(gtk4::Align::Start);
    title.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    header.append(&title);
    panel.append(&header);

    // Vertical tabs: custom tab bar (left) + stack (right)
    let hbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    hbox.set_vexpand(true);
    hbox.set_hexpand(true);

    let stack = gtk4::Stack::new();
    stack.set_transition_type(gtk4::StackTransitionType::Crossfade);
    stack.set_transition_duration(150);
    stack.set_hexpand(true);
    stack.set_vexpand(true);

    // Build tabs
    let overview = build_overview_tab(&ext, panel);
    stack.add_titled(&overview, Some("overview"), "Overview");

    let contributions = build_contributions_tab(&ext);
    stack.add_titled(&contributions, Some("contributions"), "Features");

    let contribs = &ext.manifest.contributions;
    let has_runnable = !contribs.commands.is_empty()
        || !contribs.text_transforms.is_empty()
        || !contribs.sidebar_panels.is_empty();
    if has_runnable {
        let controls = build_controls_tab(&ext);
        stack.add_titled(&controls, Some("controls"), "Controls");
    }

    // Custom vertical tab bar
    let tab_bar = build_vertical_tab_bar(&stack, has_runnable);
    hbox.append(&tab_bar);

    let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
    hbox.append(&sep);
    hbox.append(&stack);

    panel.append(&hbox);
}

/// Build a custom vertical tab bar for the detail stack
fn build_vertical_tab_bar(stack: &gtk4::Stack, has_controls: bool) -> gtk4::Box {
    // Box::new(...) allocates the data on the heap rather than the stack.
    let bar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    bar.add_css_class("ext-detail-tab-bar");
    bar.set_size_request(90, -1);

    let tabs: Vec<(&str, &str, &str)> = {
        let mut v = vec![
            ("overview", "Overview", "user-info-symbolic"),
            ("contributions", "Features", "view-list-symbolic"),
        ];
        if has_controls {
            v.push(("controls", "Controls", "media-playback-start-symbolic"));
        }
        v
    };

    let stack_ref = stack.clone();
    let buttons: Vec<gtk4::ToggleButton> = Vec::new();
    // Rc::new(...) creates a new Reference Counted pointer for shared ownership.
    let buttons_rc = std::rc::Rc::new(std::cell::RefCell::new(buttons));

    for (i, (name, label, icon_name)) in tabs.iter().enumerate() {
        let btn = gtk4::ToggleButton::new();
        btn.add_css_class("ext-tab-button");
        btn.set_active(i == 0);

        let btn_content = gtk4::Box::new(gtk4::Orientation::Vertical, 2);
        btn_content.set_halign(gtk4::Align::Center);
        btn_content.set_margin_top(8);
        btn_content.set_margin_bottom(8);

        let icon = gtk4::Image::from_icon_name(icon_name);
        icon.set_pixel_size(18);
        btn_content.append(&icon);

        let lbl = Label::new(Some(label));
        lbl.add_css_class("ext-tab-label");
        btn_content.append(&lbl);

        btn.set_child(Some(&btn_content));

        let page_name = name.to_string();
        let stack_clone = stack_ref.clone();
        let buttons_clone = buttons_rc.clone();
        btn.connect_toggled(move |b| {
            if b.is_active() {
                stack_clone.set_visible_child_name(&page_name);
                // borrow() gets read-only access to the data inside a RefCell.
                let btns = buttons_clone.borrow();
                for other in btns.iter() {
                    if other != b {
                        other.set_active(false);
                    }
                }
            }
        });

        bar.append(&btn);
        // borrow_mut() gets mutable access to the data inside a RefCell. Panics if already borrowed.
        buttons_rc.borrow_mut().push(btn);
    }

    bar
}

/// Build the Overview tab
fn build_overview_tab(ext: &Extension, panel: &gtk4::Box) -> gtk4::ScrolledWindow {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vexpand(true);

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    // Icon + Name
    let icon = gtk4::Image::from_icon_name("application-x-addon-symbolic");
    icon.set_pixel_size(48);
    icon.add_css_class("extension-icon");
    icon.set_halign(gtk4::Align::Start);
    vbox.append(&icon);

    let name_label = Label::new(Some(&ext.manifest.name));
    name_label.add_css_class("extension-detail-name");
    name_label.set_halign(gtk4::Align::Start);
    vbox.append(&name_label);

    // Description
    let desc = Label::new(Some(&ext.manifest.description));
    desc.add_css_class("extension-description");
    desc.set_halign(gtk4::Align::Start);
    desc.set_wrap(true);
    desc.set_wrap_mode(gtk4::pango::WrapMode::WordChar);
    desc.set_max_width_chars(50);
    vbox.append(&desc);

    // Badges
    let badges = collect_badges(&ext.manifest.contributions);
    if !badges.is_empty() {
        let badge_flow = gtk4::FlowBox::new();
        badge_flow.set_selection_mode(gtk4::SelectionMode::None);
        badge_flow.set_max_children_per_line(5);
        badge_flow.set_margin_top(4);
        badge_flow.set_margin_bottom(4);
        badge_flow.set_column_spacing(4);
        badge_flow.set_row_spacing(2);
        for badge_text in &badges {
            let badge = Label::new(Some(badge_text));
            badge.add_css_class("extension-badge");
            badge.set_margin_start(2);
            badge.set_margin_end(2);
            badge_flow.insert(&badge, -1);
        }
        vbox.append(&badge_flow);
    }

    let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sep.set_margin_top(4);
    sep.set_margin_bottom(4);
    vbox.append(&sep);

    // Info grid
    let grid = gtk4::Grid::new();
    grid.set_row_spacing(6);
    grid.set_column_spacing(12);

    let fields: &[(&str, String)] = &[
        ("ID", ext.manifest.id.clone()),
        ("Version", ext.manifest.version.clone()),
        ("Author", ext.manifest.author.clone()),
        ("Path", ext.path.to_string_lossy().into_owned()),
    ];

    for (i, (label, value)) in fields.iter().enumerate() {
        let key = Label::new(Some(label));
        key.add_css_class("ext-info-key");
        key.set_halign(gtk4::Align::Start);
        grid.attach(&key, 0, i as i32, 1, 1);

        let val = Label::new(Some(value));
        val.add_css_class("ext-info-value");
        val.set_halign(gtk4::Align::Start);
        val.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
        val.set_max_width_chars(30);
        val.set_selectable(true);
        grid.attach(&val, 1, i as i32, 1, 1);
    }
    vbox.append(&grid);

    let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sep2.set_margin_top(4);
    sep2.set_margin_bottom(4);
    vbox.append(&sep2);

    // Enable/Disable switch
    let switch_row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    let switch_label = Label::new(Some("Enabled"));
    switch_label.set_hexpand(true);
    switch_label.set_halign(gtk4::Align::Start);
    switch_label.add_css_class("ext-info-key");
    switch_row.append(&switch_label);

    let switch = Switch::new();
    switch.set_active(ext.manifest.enabled);
    switch.set_valign(gtk4::Align::Center);
    let ext_id = ext.manifest.id.clone();
    switch.connect_state_set(move |_sw, enabled| {
        let mut mgr = super::manager::get_manager();
        mgr.set_enabled(&ext_id, enabled);
        drop(mgr);
        crate::status_log::log_info(&format!(
            "Extension {}",
            if enabled { "enabled" } else { "disabled" }
        ));
        super::hooks::refresh_extension(&ext_id, enabled);
        glib::Propagation::Proceed
    });
    switch_row.append(&switch);
    vbox.append(&switch_row);

    // Uninstall button (not available for native/built-in extensions)
    if !ext.manifest.is_native {
        let uninstall_btn = gtk4::Button::with_label("Uninstall");
        uninstall_btn.add_css_class("destructive-action");
        uninstall_btn.set_margin_top(12);
        uninstall_btn.set_halign(gtk4::Align::Start);

        let ext_id_rm = ext.manifest.id.clone();
        let panel_weak = glib::object::WeakRef::new();
        panel_weak.set(Some(panel));
        uninstall_btn.connect_clicked(move |_| {
            let mut mgr = super::manager::get_manager();
            // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
            match mgr.remove_extension(&ext_id_rm) {
                Ok(name) => {
                    crate::status_log::log_success(&format!("Uninstalled {}", name));
                    drop(mgr);
                    if let Some(panel) = panel_weak.upgrade() {
                        populate_extensions_panel(&panel);
                    }
                }
                Err(e) => {
                    crate::status_log::log_error(&format!("Uninstall failed: {}", e));
                }
            }
        });
        vbox.append(&uninstall_btn);
    } else {
        let built_in_label = Label::new(Some("Built-in extension — cannot be uninstalled"));
        built_in_label.add_css_class("dim-label");
        built_in_label.set_margin_top(12);
        built_in_label.set_halign(gtk4::Align::Start);
        vbox.append(&built_in_label);
    }

    scrolled.set_child(Some(&vbox));
    scrolled
}

/// Build the Contributions/Features tab
fn build_contributions_tab(ext: &Extension) -> gtk4::ScrolledWindow {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vexpand(true);

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    let contribs = &ext.manifest.contributions;

    if let Some(ref sb) = contribs.status_bar {
        append_section_header(&vbox, "Status Bar");
        append_detail_row(&vbox, "Script", &sb.script);
    }

    if let Some(ref css) = contribs.css {
        append_section_header(&vbox, "Theme (CSS)");
        append_detail_row(&vbox, "File", &css.file);
    }

    if !contribs.keybindings.is_empty() {
        append_section_header(&vbox, "Keybindings");
        for kb in &contribs.keybindings {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.set_margin_top(2);
            row.set_margin_bottom(2);

            let key_label = Label::new(Some(&kb.key));
            key_label.add_css_class("ext-keybinding-key");
            row.append(&key_label);

            let title_label = Label::new(Some(&kb.title));
            title_label.add_css_class("ext-info-value");
            title_label.set_hexpand(true);
            title_label.set_halign(gtk4::Align::Start);
            row.append(&title_label);

            vbox.append(&row);
        }
    }

    if !contribs.commands.is_empty() {
        append_section_header(&vbox, "Commands");
        for cmd in &contribs.commands {
            append_detail_row(&vbox, &cmd.title, &cmd.script);
        }
    }

    if let Some(ref cm) = contribs.context_menus {
        if !cm.editor.is_empty() {
            append_section_header(&vbox, "Editor Context Menu");
            for entry in &cm.editor {
                append_detail_row(&vbox, &entry.label, &entry.script);
            }
        }
        if !cm.file_explorer.is_empty() {
            append_section_header(&vbox, "File Explorer Context Menu");
            for entry in &cm.file_explorer {
                append_detail_row(&vbox, &entry.label, &entry.script);
            }
        }
    }

    if !contribs.linters.is_empty() {
        append_section_header(&vbox, "Linters");
        for linter in &contribs.linters {
            let langs = linter.languages.join(", ");
            append_detail_row(&vbox, &format!("Languages: {}", langs), &linter.script);
        }
    }

    if let Some(ref hooks) = contribs.hooks {
        append_section_header(&vbox, "Lifecycle Hooks");
        if let Some(ref s) = hooks.on_app_start {
            append_detail_row(&vbox, "on_app_start", s);
        }
        if let Some(ref s) = hooks.on_file_open {
            append_detail_row(&vbox, "on_file_open", s);
        }
        if let Some(ref s) = hooks.on_file_save {
            append_detail_row(&vbox, "on_file_save", s);
        }
        if let Some(ref s) = hooks.on_file_close {
            append_detail_row(&vbox, "on_file_close", s);
        }
    }

    if !contribs.text_transforms.is_empty() {
        append_section_header(&vbox, "Text Transforms");
        for t in &contribs.text_transforms {
            append_detail_row(&vbox, &t.title, &t.script);
        }
    }

    if !contribs.sidebar_panels.is_empty() {
        append_section_header(&vbox, "Sidebar Panels");
        for p in &contribs.sidebar_panels {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
            row.set_margin_top(2);
            let icon = gtk4::Image::from_icon_name(&p.icon);
            icon.set_pixel_size(14);
            row.append(&icon);
            let label = Label::new(Some(&p.title));
            label.add_css_class("ext-info-value");
            label.set_halign(gtk4::Align::Start);
            row.append(&label);
            vbox.append(&row);
        }
    }

    if vbox.first_child().is_none() {
        let empty = Label::new(Some("No contributions"));
        empty.add_css_class("dim-label");
        empty.set_margin_top(24);
        vbox.append(&empty);
    }

    scrolled.set_child(Some(&vbox));
    scrolled
}

/// Build the Controls tab with runnable actions
fn build_controls_tab(ext: &Extension) -> gtk4::ScrolledWindow {
    let scrolled = gtk4::ScrolledWindow::new();
    scrolled.set_hscrollbar_policy(gtk4::PolicyType::Never);
    scrolled.set_vexpand(true);

    let vbox = gtk4::Box::new(gtk4::Orientation::Vertical, 4);
    vbox.set_margin_start(12);
    vbox.set_margin_end(12);
    vbox.set_margin_top(12);
    vbox.set_margin_bottom(12);

    let contribs = &ext.manifest.contributions;

    // Runnable commands
    if !contribs.commands.is_empty() {
        append_section_header(&vbox, "Run Commands");
        for cmd in &contribs.commands {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.set_margin_top(2);
            row.set_margin_bottom(2);

            let label = Label::new(Some(&cmd.title));
            label.add_css_class("ext-info-value");
            label.set_hexpand(true);
            label.set_halign(gtk4::Align::Start);
            label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
            row.append(&label);

            let run_btn = gtk4::Button::new();
            run_btn.set_icon_name("media-playback-start-symbolic");
            run_btn.add_css_class("flat");
            run_btn.set_tooltip_text(Some(&format!("Run: {}", cmd.title)));

            let ext_path = ext.path.clone();
            let script = cmd.script.clone();
            let cmd_title = cmd.title.clone();
            run_btn.connect_clicked(move |_| {
                let script_path = ext_path.join(&script);
                let file_path = super::hooks::ACTIVE_FILE_PATH.with(|fp| {
                    // borrow() gets read-only access to the data inside a RefCell.
                    fp.borrow().as_ref().map(|p| p.to_string_lossy().into_owned())
                });
                let fp = file_path.as_deref().unwrap_or("");
                // match statements evaluate different cases and MUST be exhaustive (cover all possibilities).
                match super::runner::run_script(&script_path, &[fp, ""], None) {
                    Ok(output) => {
                        if output.is_empty() {
                            crate::status_log::log_info(&format!("✓ {}", cmd_title));
                        } else {
                            crate::status_log::log_info(&format!(
                                "✓ {} → {}",
                                cmd_title,
                                &output[..output.len().min(80)]
                            ));
                        }
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!("✗ {}: {}", cmd_title, e));
                    }
                }
            });
            row.append(&run_btn);
            vbox.append(&row);
        }
    }

    // Runnable text transforms
    if !contribs.text_transforms.is_empty() {
        append_section_header(&vbox, "Run Transforms");

        let note = Label::new(Some("Transforms operate on the current selection"));
        note.add_css_class("dim-label");
        note.set_halign(gtk4::Align::Start);
        note.set_margin_bottom(4);
        vbox.append(&note);

        for t in &contribs.text_transforms {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.set_margin_top(2);
            row.set_margin_bottom(2);

            let label = Label::new(Some(&t.title));
            label.add_css_class("ext-info-value");
            label.set_hexpand(true);
            label.set_halign(gtk4::Align::Start);
            row.append(&label);

            let run_btn = gtk4::Button::new();
            run_btn.set_icon_name("media-playback-start-symbolic");
            run_btn.add_css_class("flat");
            run_btn.set_tooltip_text(Some(&format!("Run: {}", t.title)));

            let ext_path = ext.path.clone();
            let script = t.script.clone();
            let t_title = t.title.clone();
            run_btn.connect_clicked(move |_| {
                // Get current selection from active editor
                let result = super::hooks::ACTIVE_NOTEBOOK.with(|nb_cell| {
                    // borrow() gets read-only access to the data inside a RefCell.
                    let nb_opt = nb_cell.borrow();
                    let nb = nb_opt.as_ref()?;
                    let page_num = nb.current_page()?;
                    let (tv, _) =
                        crate::handlers::get_text_view_and_buffer_for_page(nb, page_num)?;
                    let sv = tv.downcast_ref::<sourceview5::View>()?;
                    let buf = sv.buffer();
                    let (start, end) = buf.selection_bounds()?;
                    let sel_text = buf.text(&start, &end, false).to_string();
                    Some((sel_text, buf, start, end))
                });

                if let Some((selection, buf, start, end)) = result {
                    let script_path = ext_path.join(&script);
                    let fp = super::hooks::ACTIVE_FILE_PATH
                        .with(|f| {
                            // borrow() gets read-only access to the data inside a RefCell.
                            f.borrow()
                                .as_ref()
                                .map(|p| p.to_string_lossy().into_owned())
                        })
                        .unwrap_or_default();
                    match super::runner::run_script(&script_path, &[&fp], Some(&selection)) {
                        Ok(output) if !output.is_empty() => {
                            let mut s = start;
                            let mut e = end;
                            buf.delete(&mut s, &mut e);
                            buf.insert(&mut s, &output);
                            crate::status_log::log_info(&format!("✓ {}", t_title));
                        }
                        Ok(_) => {
                            crate::status_log::log_info(&format!("✓ {} (no output)", t_title));
                        }
                        Err(err) => {
                            crate::status_log::log_error(&format!("✗ {}: {}", t_title, err));
                        }
                    }
                } else {
                    crate::status_log::log_info("Select text first to apply a transform");
                }
            });
            row.append(&run_btn);
            vbox.append(&row);
        }
    }

    // Sidebar panel refresh buttons
    if !contribs.sidebar_panels.is_empty() {
        append_section_header(&vbox, "Sidebar Panels");
        for p in &contribs.sidebar_panels {
            let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
            row.set_margin_top(2);
            row.set_margin_bottom(2);

            let icon = gtk4::Image::from_icon_name(&p.icon);
            icon.set_pixel_size(14);
            row.append(&icon);

            let label = Label::new(Some(&p.title));
            label.add_css_class("ext-info-value");
            label.set_hexpand(true);
            label.set_halign(gtk4::Align::Start);
            row.append(&label);

            let refresh_btn = gtk4::Button::new();
            refresh_btn.set_icon_name("view-refresh-symbolic");
            refresh_btn.add_css_class("flat");
            refresh_btn.set_tooltip_text(Some(&format!("Refresh: {}", p.title)));

            let ext_path = ext.path.clone();
            let script = p.script.clone();
            let p_title = p.title.clone();
            refresh_btn.connect_clicked(move |_| {
                let script_path = ext_path.join(&script);
                let fp = super::hooks::ACTIVE_FILE_PATH
                    .with(|f| {
                        f.borrow()
                            .as_ref()
                            .map(|p| p.to_string_lossy().into_owned())
                    })
                    .unwrap_or_default();
                match super::runner::run_script(&script_path, &["refresh", &fp], None) {
                    Ok(output) => {
                        crate::status_log::log_info(&format!(
                            "✓ {} refreshed ({}b)",
                            p_title,
                            output.len()
                        ));
                    }
                    Err(e) => {
                        crate::status_log::log_error(&format!("✗ {}: {}", p_title, e));
                    }
                }
            });
            row.append(&refresh_btn);
            vbox.append(&row);
        }
    }

    scrolled.set_child(Some(&vbox));
    scrolled
}

// ── Helpers ─────────────────────────────────────────────────────

/// Append a section header label
fn append_section_header(vbox: &gtk4::Box, text: &str) {
    let sep = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sep.set_margin_top(8);
    sep.set_margin_bottom(2);
    if vbox.first_child().is_some() {
        vbox.append(&sep);
    }
    let label = Label::new(Some(text));
    label.add_css_class("ext-section-header");
    label.set_halign(gtk4::Align::Start);
    label.set_margin_bottom(4);
    vbox.append(&label);
}

/// Append a key-value detail row
fn append_detail_row(vbox: &gtk4::Box, key: &str, value: &str) {
    let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    row.set_margin_top(2);
    row.set_margin_bottom(2);

    let key_label = Label::new(Some(key));
    key_label.add_css_class("ext-info-value");
    key_label.set_halign(gtk4::Align::Start);
    key_label.set_hexpand(true);
    key_label.set_ellipsize(gtk4::pango::EllipsizeMode::End);
    row.append(&key_label);

    let val_label = Label::new(Some(value));
    val_label.add_css_class("ext-info-dim");
    val_label.set_halign(gtk4::Align::End);
    val_label.set_ellipsize(gtk4::pango::EllipsizeMode::Middle);
    val_label.set_max_width_chars(20);
    row.append(&val_label);

    vbox.append(&row);
}
