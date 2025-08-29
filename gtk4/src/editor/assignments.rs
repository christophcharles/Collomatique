use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_core::ops::AssignmentsUpdateOp;

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
            gtk::Box {
                set_hexpand: true,
                set_orientation: gtk::Orientation::Vertical,
                set_margin_all: 5,
                set_spacing: 5,
                gtk::Label {
                    set_hexpand: true,
                    set_label: "Placeholder",
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Assignments {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            assignments: collomatique_state_colloscopes::assignments::Assignments::default(),
        };
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
            }
        }
    }
}
