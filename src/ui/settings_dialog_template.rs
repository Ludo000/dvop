//! # Settings Dialog — GTK4 Composite Template
//!
//! Loads the settings dialog from `resources/settings_dialog.ui`. Template
//! children include theme dropdowns (light/dark), font-size spin buttons,
//! and an info label showing the current system theme.
//!
//! Used by `settings.rs` to populate and connect the settings UI.
//!
//! See FEATURES.md: Feature #128 — Settings Menu

use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{gio, glib, CompositeTemplate, Dialog, DropDown, Label, SpinButton};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/settings_dialog.ui")]
    pub struct SettingsDialog {
        #[template_child]
        pub theme_info: TemplateChild<Label>,
        #[template_child]
        pub light_theme_dropdown: TemplateChild<DropDown>,
        #[template_child]
        pub dark_theme_dropdown: TemplateChild<DropDown>,
        #[template_child]
        pub font_size_spin: TemplateChild<SpinButton>,
        #[template_child]
        pub terminal_font_size_spin: TemplateChild<SpinButton>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SettingsDialog {
        const NAME: &'static str = "DvopSettingsDialog";
        type Type = super::SettingsDialog;
        type ParentType = Dialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SettingsDialog {}
    impl WidgetImpl for SettingsDialog {}
    impl WindowImpl for SettingsDialog {}
    impl DialogImpl for SettingsDialog {}
}

glib::wrapper! {
    pub struct SettingsDialog(ObjectSubclass<imp::SettingsDialog>)
        @extends gtk4::Widget, gtk4::Window, Dialog,
        @implements gio::ActionGroup, gio::ActionMap, gtk4::Accessible, gtk4::Buildable,
                    gtk4::ConstraintTarget, gtk4::Native, gtk4::Root, gtk4::ShortcutManager;
}

impl SettingsDialog {
    pub fn new<P: IsA<gtk4::ApplicationWindow>>(parent: &P) -> Self {
        glib::Object::builder()
            .property(
                "transient-for",
                parent.as_ref().upcast_ref::<gtk4::Window>(),
            )
            .build()
    }

    pub fn theme_info(&self) -> Label {
        self.imp().theme_info.get()
    }

    pub fn light_theme_dropdown(&self) -> DropDown {
        self.imp().light_theme_dropdown.get()
    }

    pub fn dark_theme_dropdown(&self) -> DropDown {
        self.imp().dark_theme_dropdown.get()
    }

    pub fn font_size_spin(&self) -> SpinButton {
        self.imp().font_size_spin.get()
    }

    pub fn terminal_font_size_spin(&self) -> SpinButton {
        self.imp().terminal_font_size_spin.get()
    }
}
