// Template-based search panel widget

use gtk4::prelude::*;
use gtk4::subclass::prelude::*;
use gtk4::{glib, CompositeTemplate, Box as GtkBox, TextView, TextBuffer, ListBox, Button, Label, ScrolledWindow};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/search_panel.ui")]
    pub struct SearchPanel {
        #[template_child]
        pub search_text_view: TemplateChild<TextView>,
        #[template_child]
        pub case_toggle: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub whole_word_toggle: TemplateChild<gtk4::ToggleButton>,
        #[template_child]
        pub replace_text_view: TemplateChild<TextView>,
        #[template_child]
        pub buttons_box: TemplateChild<GtkBox>,
        #[template_child]
        pub replace_btn: TemplateChild<Button>,
        #[template_child]
        pub replace_all_btn: TemplateChild<Button>,
        #[template_child]
        pub status: TemplateChild<Label>,
        #[template_child]
        pub results_list: TemplateChild<ListBox>,
        #[template_child]
        pub scroller: TemplateChild<ScrolledWindow>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SearchPanel {
        const NAME: &'static str = "DvopSearchPanel";
        type Type = super::SearchPanel;
        type ParentType = GtkBox;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for SearchPanel {}
    impl WidgetImpl for SearchPanel {}
    impl BoxImpl for SearchPanel {}
}

glib::wrapper! {
    pub struct SearchPanel(ObjectSubclass<imp::SearchPanel>)
        @extends gtk4::Widget, GtkBox,
        @implements gtk4::Accessible, gtk4::Buildable, gtk4::ConstraintTarget, gtk4::Orientable;
}

impl SearchPanel {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn search_buffer(&self) -> TextBuffer {
        self.imp().search_text_view.buffer()
    }

    pub fn replace_buffer(&self) -> TextBuffer {
        self.imp().replace_text_view.buffer()
    }

    pub fn case_toggle(&self) -> gtk4::ToggleButton {
        self.imp().case_toggle.get()
    }

    pub fn whole_word_toggle(&self) -> gtk4::ToggleButton {
        self.imp().whole_word_toggle.get()
    }

    pub fn buttons_box(&self) -> GtkBox {
        self.imp().buttons_box.get()
    }

    pub fn replace_btn(&self) -> Button {
        self.imp().replace_btn.get()
    }

    pub fn replace_all_btn(&self) -> Button {
        self.imp().replace_all_btn.get()
    }

    pub fn status_label(&self) -> Label {
        self.imp().status.get()
    }

    pub fn results_list(&self) -> ListBox {
        self.imp().results_list.get()
    }
}
