// Template-based git diff panel widget

use gtk4::subclass::prelude::*;
use gtk4::{glib, CompositeTemplate, Box as GtkBox, ListBox, Button, MenuButton, ScrolledWindow};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/git_diff_panel.ui")]
    pub struct GitDiffPanel {
        #[template_child]
        pub branch_button: TemplateChild<MenuButton>,
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
    }

    #[glib::object_subclass]
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

    impl ObjectImpl for GitDiffPanel {}
    impl WidgetImpl for GitDiffPanel {}
    impl BoxImpl for GitDiffPanel {}
}

glib::wrapper! {
    pub struct GitDiffPanel(ObjectSubclass<imp::GitDiffPanel>)
        @extends gtk4::Widget, GtkBox;
}

impl GitDiffPanel {
    pub fn new() -> Self {
        glib::Object::builder().build()
    }

    pub fn branch_button(&self) -> MenuButton {
        self.imp().branch_button.get()
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
}
