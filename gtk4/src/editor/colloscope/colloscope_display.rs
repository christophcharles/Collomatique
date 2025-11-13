use gtk::prelude::{OrientableExt, WidgetExt};
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender};

#[derive(Debug)]
pub enum DisplayInput {
    Update(
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::slots::Slots,
        collomatique_state_colloscopes::students::Students,
        collomatique_state_colloscopes::group_lists::GroupLists,
        collomatique_state_colloscopes::colloscopes::Colloscope,
    ),
}

#[derive(Debug)]
pub enum DisplayOutput {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum DisplayIssue {
    NoPeriods,
    NoWeeks,
    NoSubjects,
    NoSlots,
}

pub struct Display {
    periods: collomatique_state_colloscopes::periods::Periods,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    slots: collomatique_state_colloscopes::slots::Slots,
    students: collomatique_state_colloscopes::students::Students,
    group_lists: collomatique_state_colloscopes::group_lists::GroupLists,
    colloscope: collomatique_state_colloscopes::colloscopes::Colloscope,

    issue: Option<DisplayIssue>,
}

#[relm4::component(pub)]
impl Component for Display {
    type Input = DisplayInput;
    type Output = DisplayOutput;
    type Init = ();
    type CommandOutput = ();

    view! {
        #[root]
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            gtk::Box {
                set_orientation: gtk::Orientation::Horizontal,
                add_css_class: "frame",
                add_css_class: "view",
                #[watch]
                set_visible: model.issue.is_none(),
                gtk::Label {
                    set_hexpand: true,
                    set_size_request: (-1,200),
                    set_label: "Interface en construction",
                },
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "<i>Aucune période à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: model.issue == Some(DisplayIssue::NoPeriods),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "<i>Aucune semaine de colle à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: model.issue == Some(DisplayIssue::NoWeeks),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "<i>Aucune matière à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: model.issue == Some(DisplayIssue::NoSubjects),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "<i>Aucun créneau de colles à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: model.issue == Some(DisplayIssue::NoSlots),
            },
        },
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Display {
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            slots: collomatique_state_colloscopes::slots::Slots::default(),
            students: collomatique_state_colloscopes::students::Students::default(),
            group_lists: collomatique_state_colloscopes::group_lists::GroupLists::default(),
            colloscope: collomatique_state_colloscopes::colloscopes::Colloscope::default(),
            issue: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            DisplayInput::Update(periods, subjects, slots, students, group_lists, colloscope) => {
                self.periods = periods;
                self.subjects = subjects;
                self.slots = slots;
                self.students = students;
                self.group_lists = group_lists;
                self.colloscope = colloscope;

                self.update_display_issue();
            }
        }
    }
}

impl Display {
    fn update_display_issue(&mut self) {
        self.issue = if self.periods.ordered_period_list.is_empty() {
            Some(DisplayIssue::NoPeriods)
        } else if self.periods.count_weeks() == 0 {
            Some(DisplayIssue::NoWeeks)
        } else if self.subjects.ordered_subject_list.is_empty() {
            Some(DisplayIssue::NoSubjects)
        } else if self
            .slots
            .subject_map
            .iter()
            .map(|(_id, subject_slots)| subject_slots.ordered_slots.len())
            .sum::<usize>()
            == 0
        {
            Some(DisplayIssue::NoSlots)
        } else {
            None
        };
    }
}
