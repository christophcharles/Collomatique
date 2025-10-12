use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::{adw, gtk};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
};

use collomatique_ops::StudentsUpdateOp;

mod dialog;

#[derive(Debug)]
pub enum StudentsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
    ),
    EditStudentClicked(collomatique_state_colloscopes::StudentId),
    DeleteStudentClicked(collomatique_state_colloscopes::StudentId),
    AddStudentClicked,
    FilterChanged(Option<usize>),
    StudentEditResult(
        collomatique_state_colloscopes::students::Student<collomatique_state_colloscopes::PeriodId>,
    ),
}

#[derive(Debug)]
enum StudentModificationReason {
    New,
    Edit(collomatique_state_colloscopes::StudentId),
}

#[derive(Debug, PartialEq, Eq)]
enum StudentFilter {
    NoFilter,
    NoSubjectLinked,
    Period(collomatique_state_colloscopes::PeriodId),
}

use crate::widgets::contact_list::ContactInfo;

pub struct Students {
    periods:
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,

    student_modification_reason: StudentModificationReason,
    current_filter: StudentFilter,
    current_list: Vec<ContactInfo<collomatique_state_colloscopes::StudentId>>,

    contact_list:
        Controller<crate::widgets::contact_list::Widget<collomatique_state_colloscopes::StudentId>>,

    filter_dropdown: Controller<crate::widgets::droplist::Widget>,

    dialog: Controller<dialog::Dialog>,
}

impl Students {
    fn generate_students_count_text(&self) -> String {
        if self.current_list.len() == 0 || self.current_list.len() == 1 {
            format!(
                "<i>{} élève sur {} affiché</i>",
                self.current_list.len(),
                self.students.student_map.len(),
            )
        } else {
            format!(
                "<i>{} élèves sur {} affichés</i>",
                self.current_list.len(),
                self.students.student_map.len(),
            )
        }
    }
}

#[relm4::component(pub)]
impl Component for Students {
    type Input = StudentsInput;
    type Output = StudentsUpdateOp;
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
                        set_label: "Afficher les élèves de :",
                    },
                    append: model.filter_dropdown.widget(),
                    append = &gtk::Box {
                        set_hexpand: true,
                    },
                    append = &gtk::Label {
                        #[watch]
                        set_label: &model.generate_students_count_text(),
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
                    connect_clicked => StudentsInput::AddStudentClicked,
                    adw::ButtonContent {
                        set_icon_name: "edit-add",
                        set_label: "Ajouter un élève",
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
                    StudentsInput::EditStudentClicked(id)
                }
                crate::widgets::contact_list::WidgetOutput::DeleteContact(id) => {
                    StudentsInput::DeleteStudentClicked(id)
                }
            });
        let filter_dropdown = crate::widgets::droplist::Widget::builder()
            .launch(crate::widgets::droplist::WidgetParams {
                initial_list: vec!["Toutes les périodes".into(), "Aucune période".into()],
                initial_selected: Some(0),
                enable_search: false,
                width_request: 100,
            })
            .forward(sender.input_sender(), |msg| match msg {
                crate::widgets::droplist::WidgetOutput::SelectionChanged(num) => {
                    StudentsInput::FilterChanged(num)
                }
            });
        let dialog = dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                dialog::DialogOutput::Accepted(student_data) => {
                    StudentsInput::StudentEditResult(student_data)
                }
            });
        let model = Students {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            student_modification_reason: StudentModificationReason::New,
            current_filter: StudentFilter::NoFilter,
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
            StudentsInput::Update(new_periods, new_students) => {
                self.periods = new_periods;
                self.students = new_students;
                self.fix_current_filter_if_necessary();
                self.update_filter_droplist();
                self.update_current_list();
            }
            StudentsInput::DeleteStudentClicked(id) => {
                sender.output(StudentsUpdateOp::DeleteStudent(id)).unwrap();
            }
            StudentsInput::EditStudentClicked(id) => {
                self.student_modification_reason = StudentModificationReason::Edit(id);
                let student_data = self
                    .students
                    .student_map
                    .get(&id)
                    .expect("Student id should be valid on edit");
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.periods.clone(),
                        student_data.clone(),
                    ))
                    .unwrap();
            }
            StudentsInput::AddStudentClicked => {
                self.student_modification_reason = StudentModificationReason::New;
                self.dialog
                    .sender()
                    .send(dialog::DialogInput::Show(
                        self.periods.clone(),
                        collomatique_state_colloscopes::students::Student::default(),
                    ))
                    .unwrap();
            }
            StudentsInput::FilterChanged(num) => {
                self.current_filter = match num {
                    None => StudentFilter::NoFilter,
                    Some(0) => StudentFilter::NoFilter,
                    Some(1) => StudentFilter::NoSubjectLinked,
                    Some(x) => {
                        let index = x - 2;
                        assert!(index < self.periods.ordered_period_list.len());

                        StudentFilter::Period(self.periods.ordered_period_list[index].0.clone())
                    }
                };
                self.update_current_list();
            }
            StudentsInput::StudentEditResult(student_data) => {
                sender
                    .output(match self.student_modification_reason {
                        StudentModificationReason::New => {
                            StudentsUpdateOp::AddNewStudent(student_data)
                        }
                        StudentModificationReason::Edit(student_id) => {
                            StudentsUpdateOp::UpdateStudent(student_id, student_data)
                        }
                    })
                    .unwrap();
            }
        }
    }
}

impl Students {
    fn fix_current_filter_if_necessary(&mut self) {
        let StudentFilter::Period(period_id) = self.current_filter else {
            return;
        };

        if self.periods.find_period_position(period_id).is_some() {
            return;
        }

        self.current_filter = StudentFilter::NoFilter;
    }

    fn update_filter_droplist(&mut self) {
        let mut list = vec!["Toutes les périodes".into(), "Aucune période".into()];

        let mut first_week_num = 0usize;
        for (index, (_id, period)) in self.periods.ordered_period_list.iter().enumerate() {
            list.push(super::generate_week_succession_title(
                "La période",
                &self.periods.first_week,
                index,
                first_week_num,
                period.len(),
            ));

            first_week_num += period.len();
        }

        let num = match self.current_filter {
            StudentFilter::NoFilter => 0usize,
            StudentFilter::NoSubjectLinked => 1usize,
            StudentFilter::Period(period_id) => {
                let pos = self
                    .periods
                    .find_period_position(period_id)
                    .expect("Current filter should always point to a valid period");
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

        for (student_id, student) in &self.students.student_map {
            let keep_student = match self.current_filter {
                StudentFilter::NoFilter => true,
                StudentFilter::NoSubjectLinked => {
                    student.excluded_periods.len() == self.periods.ordered_period_list.len()
                }
                StudentFilter::Period(period_id) => !student.excluded_periods.contains(&period_id),
            };

            if keep_student {
                self.current_list.push(ContactInfo {
                    id: student_id.clone(),
                    contact: student.desc.clone(),
                    extra: {
                        let mut excluded_period_list: Vec<_> = student
                            .excluded_periods
                            .iter()
                            .map(|period_id| {
                                self.periods
                                    .find_period_position(*period_id)
                                    .expect("Period referenced by students should be valid")
                                    + 1
                            })
                            .collect();

                        excluded_period_list.sort();

                        let excluded_period_list: Vec<_> = excluded_period_list
                            .into_iter()
                            .map(|x| x.to_string())
                            .collect();

                        match excluded_period_list.len() {
                            0 => String::new(),
                            1 => format!("Exclu de la période {}", excluded_period_list[0]),
                            _ => format!(
                                "Exclu des périodes {} et {}",
                                excluded_period_list[..excluded_period_list.len() - 1].join(", "),
                                excluded_period_list.last().unwrap()
                            ),
                        }
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
