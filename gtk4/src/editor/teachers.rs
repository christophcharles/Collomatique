use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
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
    FilterChanged(usize),
}

#[derive(Debug)]
enum TeacherModificationReason {
    New,
    Edit(collomatique_state_colloscopes::TeacherId),
}

#[derive(Debug, PartialEq, Eq)]
enum TeacherFilter {
    NoFilter,
    NoSubjectLinked,
    Subject(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug)]
pub struct ContactInfo {
    pub id: collomatique_state_colloscopes::TeacherId,
    pub contact: collomatique_state_colloscopes::PersonWithContact,
    pub extra: String,
}
pub struct Teachers {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    teachers: collomatique_state_colloscopes::teachers::Teachers,

    teacher_modification_reason: TeacherModificationReason,
    current_filter: TeacherFilter,
    current_list: Vec<ContactInfo>,

    filter_dropdown: Controller<crate::widgets::droplist::Widget>,
}

impl Teachers {
    fn generate_teachers_count_text(&self) -> String {
        if self.current_list.len() == 0 || self.current_list.len() == 1 {
            format!(
                "<i>{} colleur sur {} affiché</i>",
                self.current_list.len(),
                self.teachers.teacher_map.len(),
            )
        } else {
            format!(
                "<i>{} colleurs sur {} affichés</i>",
                self.current_list.len(),
                self.teachers.teacher_map.len(),
            )
        }
    }
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
                    set_spacing: 5,
                    append = &gtk::Label {
                        set_label: "Afficher les colleurs de :",
                    },
                    append: model.filter_dropdown.widget(),
                    append = &gtk::Box {
                        set_hexpand: true,
                    },
                    append = &gtk::Label {
                        #[watch]
                        set_label: &model.generate_teachers_count_text(),
                        set_use_markup: true,
                    },
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
                initial_list: vec!["Toutes les matières".into(), "Aucune matière".into()],
                initial_selected: Some(0),
                enable_search: true,
                width_request: 100,
            })
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::droplist::WidgetOutput::SelectionChanged(num) => {
                    TeachersInput::FilterChanged(num)
                }
            });
        let model = Teachers {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            teacher_modification_reason: TeacherModificationReason::New,
            current_filter: TeacherFilter::NoFilter,
            current_list: vec![],
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
                self.fix_current_filter_if_necessary();
                self.update_filter_droplist();
                self.update_current_list();
            }
            TeachersInput::AddTeacherClicked => {
                self.teacher_modification_reason = TeacherModificationReason::New;
            }
            TeachersInput::FilterChanged(num) => {
                self.current_filter = match num {
                    0 => TeacherFilter::NoFilter,
                    1 => TeacherFilter::NoSubjectLinked,
                    x => {
                        let index = x - 2;
                        assert!(index < self.subjects.ordered_subject_list.len());

                        TeacherFilter::Subject(self.subjects.ordered_subject_list[index].0.clone())
                    }
                };
                self.update_current_list();
            }
        }
    }
}

impl Teachers {
    fn fix_current_filter_if_necessary(&mut self) {
        let TeacherFilter::Subject(subject_id) = self.current_filter else {
            return;
        };

        if self.subjects.find_subject_position(subject_id).is_some() {
            return;
        }

        self.current_filter = TeacherFilter::NoFilter;
    }

    fn update_filter_droplist(&mut self) {
        let mut list = vec!["Toutes les matières".into(), "Aucune matière".into()];

        for (_subject_id, subject) in &self.subjects.ordered_subject_list {
            list.push(subject.parameters.name.clone());
        }

        let num = match self.current_filter {
            TeacherFilter::NoFilter => 0usize,
            TeacherFilter::NoSubjectLinked => 1usize,
            TeacherFilter::Subject(subject_id) => {
                let pos = self
                    .subjects
                    .find_subject_position(subject_id)
                    .expect("Current filter should always point to a valid subject");
                pos + 2
            }
        };

        self.filter_dropdown
            .sender()
            .send(crate::widgets::droplist::WidgetInput::UpdateList(
                list,
                Some(num),
            ))
            .unwrap();
    }

    fn update_current_list(&mut self) {
        self.current_list = vec![];

        for (teacher_id, teacher) in &self.teachers.teacher_map {
            let keep_teacher = match self.current_filter {
                TeacherFilter::NoFilter => true,
                TeacherFilter::NoSubjectLinked => teacher.subjects.is_empty(),
                TeacherFilter::Subject(subject_id) => teacher.subjects.contains(&subject_id),
            };

            if keep_teacher {
                self.current_list.push(ContactInfo {
                    id: teacher_id.clone(),
                    contact: teacher.desc.clone(),
                    extra: {
                        let subject_list: Vec<_> = teacher
                            .subjects
                            .iter()
                            .map(|subject_id| {
                                self.subjects
                                    .find_subject(*subject_id)
                                    .expect("Subject referenced by teachers should be valid")
                                    .parameters
                                    .name
                                    .clone()
                            })
                            .collect();

                        subject_list.join(", ")
                    },
                });
            }
        }
    }
}
