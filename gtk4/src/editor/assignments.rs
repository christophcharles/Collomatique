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
            set_policy: (gtk::PolicyType::Automatic, gtk::PolicyType::Automatic),
            #[local_ref]
            periods_widget -> gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let period_factory = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

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

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            AssignmentsInput::Update(new_periods, new_subjects, new_students, new_assignments) => {
                self.periods = new_periods;
                self.subjects = new_subjects;
                self.students = new_students;
                self.assignments = new_assignments;

                self.update_period_factory();
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
                    .cloned()
                    .filter(|(_subject_id, subject)| !subject.excluded_periods.contains(id))
                    .collect();

                Some(assignments_display::PeriodEntryData {
                    global_first_week: self.periods.first_week.clone(),
                    first_week_num: current_first_week,
                    week_count: desc.len(),
                    filtered_subjects,
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
