//! # Git Diff Panel — GTK4 Composite Template
//!
//! Loads the widget tree defined in `resources/git_diff_panel.ui` at runtime.
//! Each `#[template_child]` field binds to a named widget in the `.ui` XML file,
//! providing type-safe access from Rust code (see `git_diff.rs`).
//!
//! ## How Composite Templates Work in GTK4-rs
//!
//! 1. The inner `mod imp` struct derives `CompositeTemplate` and annotates
//!    each field with `#[template_child]`.  The `#[template(resource = "...")]`
//!    attribute points to the GResource path compiled by `build.rs`.
//! 2. The outer `glib::wrapper!` struct wraps the inner implementation and
//!    exposes public accessor methods (e.g., `fn branch_button() -> MenuButton`).
//! 3. At construction time, `init_template()` inflates the XML and wires up
//!    the template children automatically.
//!
//! See FEATURES.md: Feature #60 — Git Status Panel

use gtk4::subclass::prelude::*;
use gtk4::{glib, Box as GtkBox, Button, CompositeTemplate, ListBox, MenuButton, ScrolledWindow, Revealer};

mod imp {
    use super::*;

    // #[derive(...)] asks the compiler to automatically generate basic trait implementations.
    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/git_diff_panel.ui")]
    pub struct GitDiffPanel {
        #[template_child]
        pub branch_button: TemplateChild<MenuButton>,
        #[template_child]
        pub git_menu_button: TemplateChild<MenuButton>,
        #[template_child]
        pub revert_all_button: TemplateChild<Button>,
        #[template_child]
        pub refresh_button: TemplateChild<Button>,
        #[template_child]
        pub stage_all_button: TemplateChild<Button>,
        #[template_child]
        pub unstage_all_button: TemplateChild<Button>,
        #[template_child]
        pub staged_files_list: TemplateChild<ListBox>,
        #[template_child]
        pub files_list: TemplateChild<ListBox>,
        #[template_child]
        pub staged_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub files_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub commit_message_view: TemplateChild<gtk4::TextView>,
        #[template_child]
        pub commit_button: TemplateChild<Button>,
        #[template_child]
        pub staged_revealer: TemplateChild<Revealer>,
    }

    #[glib::object_subclass]
    // "impl" blocks define methods and behavior for a struct or enum.
    impl ObjectSubclass for GitDiffPanel {
        const NAME: &'static str = "DvopGitDiffPanel";
        type Type = super::GitDiffPanel;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    // "impl" blocks define methods and behavior for a struct or enum.
    impl ObjectImpl for GitDiffPanel {}
    // "impl" blocks define methods and behavior for a struct or enum.
    impl WidgetImpl for GitDiffPanel {}
    // "impl" blocks define methods and behavior for a struct or enum.
    impl BoxImpl for GitDiffPanel {}
}

glib::wrapper! {
    pub struct GitDiffPanel(ObjectSubclass<imp::GitDiffPanel>)
        @extends gtk4::Widget, GtkBox,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl Default for GitDiffPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl GitDiffPanel {
    // pub makes this function public, allowing it to be used from outside this module.
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    // pub makes this function public, allowing it to be used from outside this module.
    pub fn branch_button(&self) -> MenuButton {
        self.imp().branch_button.get()
    }

    // pub makes this function public, allowing it to be used from outside this module.
    pub fn git_menu_button(&self) -> MenuButton {
        self.imp().git_menu_button.get()
    }

    // pub makes this function public, allowing it to be used from outside this module.
    pub fn revert_all_button(&self) -> Button {
        self.imp().revert_all_button.get()
    }

    pub fn refresh_button(&self) -> Button {
        self.imp().refresh_button.get()
    }

    pub fn stage_all_button(&self) -> Button {
        self.imp().stage_all_button.get()
    }

    pub fn unstage_all_button(&self) -> Button {
        self.imp().unstage_all_button.get()
    }

    pub fn staged_files_list(&self) -> ListBox {
        self.imp().staged_files_list.get()
    }

    pub fn files_list(&self) -> ListBox {
        self.imp().files_list.get()
    }

    pub fn commit_message_view(&self) -> gtk4::TextView {
        self.imp().commit_message_view.get()
    }

    pub fn commit_button(&self) -> Button {
        self.imp().commit_button.get()
    }

    pub fn staged_revealer(&self) -> Revealer {
        self.imp().staged_revealer.get()
    }
}
