// Template-based git diff panel widget

use gtk4::subclass::prelude::*;
use gtk4::{glib, CompositeTemplate, Box as GtkBox, ListBox, Button, Label, ScrolledWindow};

mod imp {
    use super::*;

    #[derive(Debug, Default, CompositeTemplate)]
    #[template(resource = "/com/example/Dvop/git_diff_panel.ui")]
    pub struct GitDiffPanel {
        #[template_child]
        pub repo_label: TemplateChild<Label>,
        #[template_child]
        pub branch_label: TemplateChild<Label>,
        #[template_child]
        pub action_box: TemplateChild<GtkBox>,
        #[template_child]
        pub refresh_button: TemplateChild<Button>,
        #[template_child]
        pub stage_all_button: TemplateChild<Button>,
        #[template_child]
        pub staged_files_list: TemplateChild<ListBox>,
        #[template_child]
        pub files_list: TemplateChild<ListBox>,
        #[template_child]
        pub staged_scroller: TemplateChild<ScrolledWindow>,
        #[template_child]
        pub files_scroller: TemplateChild<ScrolledWindow>,
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

    pub fn repo_label(&self) -> Label {
        self.imp().repo_label.get()
    }

    pub fn branch_label(&self) -> Label {
        self.imp().branch_label.get()
    }

    pub fn action_box(&self) -> GtkBox {
        self.imp().action_box.get()
    }

    pub fn refresh_button(&self) -> Button {
        self.imp().refresh_button.get()
    }

    pub fn stage_all_button(&self) -> Button {
        self.imp().stage_all_button.get()
    }

    pub fn staged_files_list(&self) -> ListBox {
        self.imp().staged_files_list.get()
    }

    pub fn files_list(&self) -> ListBox {
        self.imp().files_list.get()
    }
}
