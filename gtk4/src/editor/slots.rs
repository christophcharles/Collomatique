use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::gtk;
use relm4::{Component, ComponentParts, ComponentSender, RelmWidgetExt};

use collomatique_ops::SlotsUpdateOp;

mod slots_display;

#[derive(Debug)]
pub enum SlotsInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::PeriodId,
        >,
        collomatique_state_colloscopes::teachers::Teachers<
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::SubjectId,
        >,
        collomatique_state_colloscopes::week_patterns::WeekPatterns<
            collomatique_state_colloscopes::WeekPatternId,
        >,
        collomatique_state_colloscopes::slots::Slots<
            collomatique_state_colloscopes::SubjectId,
            collomatique_state_colloscopes::SlotId,
            collomatique_state_colloscopes::TeacherId,
            collomatique_state_colloscopes::WeekPatternId,
        >,
    ),
}

pub struct Slots {
    subjects: collomatique_state_colloscopes::subjects::Subjects<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::PeriodId,
    >,
    teachers: collomatique_state_colloscopes::teachers::Teachers<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::SubjectId,
    >,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns<
        collomatique_state_colloscopes::WeekPatternId,
    >,
    slots: collomatique_state_colloscopes::slots::Slots<
        collomatique_state_colloscopes::SubjectId,
        collomatique_state_colloscopes::SlotId,
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::WeekPatternId,
    >,
    subjects_list: FactoryVecDeque<slots_display::Entry>,
}

#[relm4::component(pub)]
impl Component for Slots {
    type Input = SlotsInput;
    type Output = SlotsUpdateOp;
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
                #[local_ref]
                subjects_box -> gtk::Box {
                    set_hexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    set_margin_top: 20,
                    set_spacing: 30,
                    #[watch]
                    set_visible: !model.slots.subject_map.is_empty(),
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let subjects_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .detach();

        let model = Slots {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            slots: collomatique_state_colloscopes::slots::Slots::default(),
            subjects_list,
        };

        let subjects_box = model.subjects_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SlotsInput::Update(subjects, teachers, week_patterns, slots) => {
                self.subjects = subjects;
                self.teachers = teachers;
                self.week_patterns = week_patterns;
                self.slots = slots;

                let new_data: Vec<_> =
                    self.subjects
                        .ordered_subject_list
                        .iter()
                        .filter_map(|(id, desc)| {
                            if desc.parameters.interrogation_parameters.is_none() {
                                return None;
                            }

                            let _subject_slots = self.slots.subject_map.get(id).expect(
                                "Subject should appear in slots if it can have interrogations",
                            );
                            Some(slots_display::EntryData {
                                subject_params: desc.parameters.clone(),
                                subject_id: id.clone(),
                            })
                        })
                        .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    new_data.into_iter(),
                    |data| slots_display::EntryInput::UpdateData(data),
                );
            }
        }
    }
}
