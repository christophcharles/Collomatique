use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_core::ops::TeachersUpdateOp;

#[derive(Debug)]
pub enum TeachersInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::teachers::Teachers,
    ),
    AddTeacherClicked,
}

#[derive(Debug)]
enum TeacherModificationReason {
    New,
    Edit(collomatique_state_colloscopes::TeacherId),
}

pub struct Teachers {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    teachers: collomatique_state_colloscopes::teachers::Teachers,

    teacher_modification_reason: TeacherModificationReason,

    filter_dropdown: Controller<crate::widgets::droplist::Widget>,
}

#[relm4::component(pub)]
impl Component for Teachers {
    type Input = TeachersInput;
    type Output = TeachersUpdateOp;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Horizontal,
                    append: model.filter_dropdown.widget(),
                },
                adw::ToggleGroup {
                    set_halign: gtk::Align::Center,
                    add_css_class: "round",
                    add = adw::Toggle {
                        set_label: Some("Tous"),
                    },
                    add = adw::Toggle {
                        set_label: Some("Non-affectés"),
                    },
                    add = adw::Toggle {
                        set_label: Some("Mathématiques"),
                    },
                    add = adw::Toggle {
                        set_label: Some("Physique"),
                    },
                    add = adw::Toggle {
                        set_label: Some("Espagnol"),
                    },
                    add = adw::Toggle {
                        set_label: Some("Allemand"),
                    },
                },
                gtk::Label {
                    set_hexpand: true,
                    set_label: "<i>3 colleurs sur 5 affichés</i>",
                    set_use_markup: true,
                },
                gtk::ListBox {
                    set_hexpand: true,
                    set_margin_top: 20,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_label: "Thomas DURAND",
                            set_size_request: (150, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("contact-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_margin_end: 5,
                            set_label: "06 06 06 06 06",
                            set_size_request: (120, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("emblem-mail-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_end: 5,
                            set_label: "thomas.durand@gmail.com",
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::End,
                            set_margin_end: 5,
                            set_label: "Mathématiques, Physique",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete",
                            add_css_class: "flat",
                        },
                    },
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_label: "Érica DUMONT",
                            set_size_request: (150, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("contact-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_margin_end: 5,
                            set_label: "07 07 07 07 07",
                            set_size_request: (120, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("emblem-mail-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_end: 5,
                            set_label: "<i>Non renseigné</i>",
                            set_use_markup: true,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::End,
                            set_margin_end: 5,
                            set_label: "Espagnol",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete",
                            add_css_class: "flat",
                        },
                    },
                    append = &gtk::Box {
                        set_hexpand: true,
                        set_orientation: gtk::Orientation::Horizontal,
                        gtk::Button {
                            set_icon_name: "edit-symbolic",
                            add_css_class: "flat",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_label: "Gertrude DUPOND",
                            set_size_request: (150, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("contact-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_margin_end: 5,
                            set_label: "<i>Non renseigné</i>",
                            set_use_markup: true,
                            set_size_request: (120, -1),
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Image {
                            set_halign: gtk::Align::Start,
                            set_margin_start: 5,
                            set_margin_end: 5,
                            set_icon_name: Some("emblem-mail-symbolic"),
                        },
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_xalign: 0.,
                            set_margin_end: 5,
                            set_label: "<i>Non renseigné</i>",
                            set_use_markup: true,
                        },
                        gtk::Box {
                            set_hexpand: true,
                        },
                        gtk::Label {
                            set_halign: gtk::Align::End,
                            set_margin_end: 5,
                            set_label: "Espagnol",
                        },
                        gtk::Separator {
                            set_orientation: gtk::Orientation::Vertical,
                        },
                        gtk::Button {
                            set_icon_name: "edit-delete",
                            add_css_class: "flat",
                        },
                    },
                },
                gtk::Button {
                    set_margin_top: 10,
                    connect_clicked => TeachersInput::AddTeacherClicked,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter un colleur",
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let filter_dropdown = crate::widgets::droplist::Widget::builder()
            .launch(crate::widgets::droplist::WidgetParams {
                initial_list: vec!["Test1".into(), "Test2".into()],
                initial_selected: Some(0),
                enable_search: true,
                width_request: 100,
            })
            .detach();
        let model = Teachers {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            teacher_modification_reason: TeacherModificationReason::New,
            filter_dropdown,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            TeachersInput::Update(new_subjects, new_teachers) => {
                self.subjects = new_subjects;
                self.teachers = new_teachers;
            }
            TeachersInput::AddTeacherClicked => {
                self.teacher_modification_reason = TeacherModificationReason::New;
            }
        }
    }
}
