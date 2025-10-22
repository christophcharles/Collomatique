use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::GroupListsUpdateOp;

#[derive(Debug)]
pub enum GroupListsInput {
    Update(
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::students::Students<
            collomatique_state_colloscopes::StudentId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::group_lists::GroupLists<
            collomatique_state_colloscopes::GroupListId,
            collomatique_state_colloscopes::PeriodId,
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::StudentId,
        >,
    ),
}

pub struct GroupLists {
    periods:
        collomatique_state_colloscopes::periods::Periods<collomatique_state_colloscopes::PeriodId>,
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    students: collomatique_state_colloscopes::students::Students<
        collomatique_state_colloscopes::StudentId,
        collomatique_state_colloscopes::PeriodId,
    >,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists<
        collomatique_state_colloscopes::GroupListId,
        collomatique_state_colloscopes::PeriodId,
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::StudentId,
    >,
}

#[relm4::component(pub)]
impl Component for GroupLists {
    type Input = GroupListsInput;
    type Output = GroupListsUpdateOp;
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
                    set_margin_top: 10,
                    set_halign: gtk::Align::Start,
                    set_label: "<big><b>En développement...</b></big>",
                    set_use_markup: true,
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = GroupLists {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            GroupListsInput::Update(periods, subjects, students, group_lists) => {
                self.periods = periods;
                self.subjects = subjects;
                self.students = students;
                self.group_lists = group_lists;
            }
        }
    }
}
