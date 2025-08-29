use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_core::ops::AssignmentsUpdateOp;

mod assignments_display;

#[derive(Debug)]
pub enum AssignmentsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::assignments::Assignments,
    ),
    UpdateStatus(
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::SubjectId,
        bool,
    ),
    CopyPreviousPeriod(collomatique_state_colloscopes::PeriodId),
}

pub struct Assignments {
    periods: collomatique_state_colloscopes::periods::Periods,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    students: collomatique_state_colloscopes::students::Students,
    assignments: collomatique_state_colloscopes::assignments::Assignments,

    period_factory: FactoryVecDeque<assignments_display::PeriodEntry>,
}

#[relm4::component(pub)]
impl Component for Assignments {
    type Input = AssignmentsInput;
    type Output = AssignmentsUpdateOp;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_margin_all: 5,
            set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.periods.ordered_period_list.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_label: "<big><b>Aucune période à afficher</b></big>",
                    set_use_markup: true,
                },
                #[local_ref]
                periods_widget -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_all: 5,
                    set_spacing: 20,
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let period_factory = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                assignments_display::PeriodEntryOutput::UpdateStatus(
                    period_id,
                    student_id,
                    subject_id,
                    new_status,
                ) => AssignmentsInput::UpdateStatus(period_id, student_id, subject_id, new_status),
                assignments_display::PeriodEntryOutput::CopyPreviousPeriod(period_id) => {
                    AssignmentsInput::CopyPreviousPeriod(period_id)
                }
            });

        let model = Assignments {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            assignments: collomatique_state_colloscopes::assignments::Assignments::default(),
            period_factory,
        };

        let periods_widget = model.period_factory.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            AssignmentsInput::Update(new_periods, new_subjects, new_students, new_assignments) => {
                self.periods = new_periods;
                self.subjects = new_subjects;
                self.students = new_students;
                self.assignments = new_assignments;

                self.update_period_factory();
            }
            AssignmentsInput::UpdateStatus(period_id, student_id, subject_id, new_status) => {
                sender
                    .output(AssignmentsUpdateOp::Assign(
                        period_id, student_id, subject_id, new_status,
                    ))
                    .unwrap();
            }
            AssignmentsInput::CopyPreviousPeriod(period_id) => {
                sender
                    .output(AssignmentsUpdateOp::DuplicatePreviousPeriod(period_id))
                    .unwrap();
            }
        }
    }
}

impl Assignments {
    fn update_period_factory(&mut self) {
        let new_data = self
            .periods
            .ordered_period_list
            .iter()
            .scan(0usize, |acc, (id, desc)| {
                let current_first_week = *acc;
                *acc += desc.len();

                let filtered_subjects = self
                    .subjects
                    .ordered_subject_list
                    .iter()
                    .filter(|(_subject_id, subject)| !subject.excluded_periods.contains(id))
                    .cloned()
                    .collect();

                let mut filtered_students: Vec<_> = self
                    .students
                    .student_map
                    .iter()
                    .filter_map(|(student_id, student)| {
                        if !student.excluded_periods.contains(id) {
                            Some((student_id.clone(), student.clone()))
                        } else {
                            None
                        }
                    })
                    .collect();

                filtered_students.sort_by(|a, b| {
                    let surname_cmp = a.1.desc.surname.cmp(&b.1.desc.surname);
                    if surname_cmp != std::cmp::Ordering::Equal {
                        return surname_cmp;
                    }

                    let firstname_cmp = a.1.desc.firstname.cmp(&b.1.desc.firstname);
                    if firstname_cmp != std::cmp::Ordering::Equal {
                        return firstname_cmp;
                    }

                    let id_cmp = a.0.cmp(&b.0);
                    id_cmp
                });

                Some(assignments_display::PeriodEntryData {
                    period_id: id.clone(),
                    global_first_week: self.periods.first_week.clone(),
                    first_week_num: current_first_week,
                    week_count: desc.len(),
                    filtered_subjects,
                    filtered_students,
                    period_assignments: self
                        .assignments
                        .period_map
                        .get(id)
                        .expect("Period id should be valid at this poind")
                        .clone(),
                })
            })
            .collect::<Vec<_>>();

        crate::tools::factories::update_vec_deque(
            &mut self.period_factory,
            new_data.into_iter(),
            |data| assignments_display::PeriodEntryInput::UpdateData(data),
        );
    }
}
