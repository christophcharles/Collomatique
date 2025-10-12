use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::TeachersUpdateOp;

mod dialog;

#[derive(Debug)]
pub enum TeachersInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::teachers::Teachers<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::SubjectId,
        >,
    ),
    EditTeacherClicked(collomatique_state_colloscopes::TeacherId),
    DeleteTeacherClicked(collomatique_state_colloscopes::TeacherId),
    AddTeacherClicked,
    FilterChanged(Option<usize>),
    TeacherEditResult(
        collomatique_state_colloscopes::teachers::Teacher<
            collomatique_state_colloscopes::SubjectId,
        >,
    ),
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

use crate::widgets::contact_list::ContactInfo;

pub struct Teachers {
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    teachers: collomatique_state_colloscopes::teachers::Teachers<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    >,

    teacher_modification_reason: TeacherModificationReason,
    current_filter: TeacherFilter,
    current_list: Vec<ContactInfo<collomatique_state_colloscopes::TeacherId>>,

    contact_list:
        Controller<crate::widgets::contact_list::Widget<collomatique_state_colloscopes::TeacherId>>,

    filter_dropdown: Controller<crate::widgets::droplist::Widget>,

    dialog: Controller<dialog::Dialog>,
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
                #[local_ref]
                contact_list_widget -> gtk::Box {
                    set_hexpand: true,
                    set_margin_top: 20,
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
        let contact_list = crate::widgets::contact_list::Widget::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::contact_list::WidgetOutput::EditContact(id) => {
                    TeachersInput::EditTeacherClicked(id)
                }
                crate::widgets::contact_list::WidgetOutput::DeleteContact(id) => {
                    TeachersInput::DeleteTeacherClicked(id)
                }
            });
        let filter_dropdown = crate::widgets::droplist::Widget::builder()
            .launch(crate::widgets::droplist::WidgetParams {
                initial_list: vec!["Toutes les matières".into(), "Aucune matière".into()],
                initial_selected: Some(0),
                enable_search: false,
                width_request: 100,
            })
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::droplist::WidgetOutput::SelectionChanged(num) => {
                    TeachersInput::FilterChanged(num)
                }
            });
        let dialog = dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                dialog::DialogOutput::Accepted(teacher_data) => {
                    TeachersInput::TeacherEditResult(teacher_data)
                }
            });
        let model = Teachers {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            teacher_modification_reason: TeacherModificationReason::New,
            current_filter: TeacherFilter::NoFilter,
            current_list: vec![],
            contact_list,
            filter_dropdown,
            dialog,
        };
        let contact_list_widget = model.contact_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            TeachersInput::Update(new_subjects, new_teachers) => {
                self.subjects = new_subjects;
                self.teachers = new_teachers;
                self.fix_current_filter_if_necessary();
                self.update_filter_droplist();
                self.update_current_list();
            }
            TeachersInput::DeleteTeacherClicked(id) => {
                sender.output(TeachersUpdateOp::DeleteTeacher(id)).unwrap();
            }
            TeachersInput::EditTeacherClicked(id) => {
                self.teacher_modification_reason = TeacherModificationReason::Edit(id);
                let teacher_data = self
                    .teachers
                    .teacher_map
                    .get(&id)
                    .expect("Teacher id should be valid on edit");
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.subjects.clone(),
                        teacher_data.clone(),
                    ))
                    .unwrap();
            }
            TeachersInput::AddTeacherClicked => {
                self.teacher_modification_reason = TeacherModificationReason::New;
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.subjects.clone(),
                        collomatique_state_colloscopes::teachers::Teacher::default(),
                    ))
                    .unwrap();
            }
            TeachersInput::FilterChanged(num) => {
                self.current_filter = match num {
                    None => TeacherFilter::NoFilter,
                    Some(0) => TeacherFilter::NoFilter,
                    Some(1) => TeacherFilter::NoSubjectLinked,
                    Some(x) => {
                        let index = x - 2;
                        TeacherFilter::Subject(
                            self.filtered_list_position_to_subject_id(index)
                                .expect("Index in subject list should be valid"),
                        )
                    }
                };
                self.update_current_list();
            }
            TeachersInput::TeacherEditResult(teacher_data) => {
                sender
                    .output(match self.teacher_modification_reason {
                        TeacherModificationReason::New => {
                            TeachersUpdateOp::AddNewTeacher(teacher_data)
                        }
                        TeacherModificationReason::Edit(teacher_id) => {
                            TeachersUpdateOp::UpdateTeacher(teacher_id, teacher_data)
                        }
                    })
                    .unwrap();
            }
        }
    }
}

impl Teachers {
    fn fix_current_filter_if_necessary(&mut self) {
        let TeacherFilter::Subject(subject_id) = self.current_filter else {
            return;
        };

        if let Some(subject) = self.subjects.find_subject(subject_id) {
            if subject.parameters.interrogation_parameters.is_some() {
                return;
            }
        }

        self.current_filter = TeacherFilter::NoFilter;
    }

    fn find_subject_position_in_filtered_list(
        &self,
        subject_id: collomatique_state_colloscopes::SubjectId,
    ) -> Option<usize> {
        let mut pos = 0usize;
        for (id, subject) in &self.subjects.ordered_subject_list {
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }
            if subject_id == *id {
                return Some(pos);
            }
            pos += 1;
        }
        None
    }

    fn filtered_list_position_to_subject_id(
        &self,
        pos: usize,
    ) -> Option<collomatique_state_colloscopes::SubjectId> {
        let mut current_pos = 0usize;
        for (subject_id, subject) in &self.subjects.ordered_subject_list {
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }
            if current_pos == pos {
                return Some(*subject_id);
            }
            current_pos += 1;
        }
        None
    }

    fn update_filter_droplist(&mut self) {
        let mut list = vec!["Toutes les matières".into(), "Aucune matière".into()];

        for (_subject_id, subject) in &self.subjects.ordered_subject_list {
            if subject.parameters.interrogation_parameters.is_none() {
                continue;
            }
            list.push(subject.parameters.name.clone());
        }

        let num = match self.current_filter {
            TeacherFilter::NoFilter => 0usize,
            TeacherFilter::NoSubjectLinked => 1usize,
            TeacherFilter::Subject(subject_id) => {
                let pos = self
                    .find_subject_position_in_filtered_list(subject_id)
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

        self.current_list.sort();

        self.contact_list
            .sender()
            .send(crate::widgets::contact_list::WidgetInput::UpdateList(
                self.current_list.clone(),
            ))
            .unwrap();
    }
}
