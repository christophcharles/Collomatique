use std::collections::BTreeMap;

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{gtk, ComponentController};
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use collomatique_ops::SlotsUpdateOp;

mod slot_params;
mod slots_display;

#[derive(Debug)]
pub enum SlotsInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::teachers::Teachers,
        collomatique_state_colloscopes::week_patterns::WeekPatterns,
        collomatique_state_colloscopes::slots::Slots,
    ),

    SlotParamsSelected(collomatique_state_colloscopes::slots::Slot),
    MoveSlotUp(collomatique_state_colloscopes::SlotId),
    MoveSlotDown(collomatique_state_colloscopes::SlotId),
    DeleteSlot(collomatique_state_colloscopes::SlotId),
    EditSlot(collomatique_state_colloscopes::SlotId),
    AddSlot(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug)]
enum SlotParamsSelectionReason {
    New(collomatique_state_colloscopes::SubjectId),
    Edit(collomatique_state_colloscopes::SlotId),
}

pub struct Slots {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    teachers: collomatique_state_colloscopes::teachers::Teachers,
    week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns,
    slots: collomatique_state_colloscopes::slots::Slots,
    subjects_list: FactoryVecDeque<slots_display::Entry>,

    slot_params_dialog: Controller<slot_params::Dialog>,
    slot_params_selection_reason: Option<SlotParamsSelectionReason>,
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
                gtk::Label {
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.slots.subject_map.is_empty(),
                    set_halign: gtk::Align::Start,
                    set_label: "<big><b>Aucune matière à afficher</b></big>",
                    set_use_markup: true,
                },
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
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let subjects_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                slots_display::EntryOutput::MoveSlotUp(slot_id) => SlotsInput::MoveSlotUp(slot_id),
                slots_display::EntryOutput::MoveSlotDown(slot_id) => {
                    SlotsInput::MoveSlotDown(slot_id)
                }
                slots_display::EntryOutput::DeleteSlot(slot_id) => SlotsInput::DeleteSlot(slot_id),
                slots_display::EntryOutput::EditSlot(slot_id) => SlotsInput::EditSlot(slot_id),
                slots_display::EntryOutput::AddSlot(subject_id) => SlotsInput::AddSlot(subject_id),
            });

        let slot_params_dialog = slot_params::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                slot_params::DialogOutput::Accepted(params) => {
                    SlotsInput::SlotParamsSelected(params)
                }
            });

        let model = Slots {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            week_patterns: collomatique_state_colloscopes::week_patterns::WeekPatterns::default(),
            slots: collomatique_state_colloscopes::slots::Slots::default(),
            subjects_list,
            slot_params_dialog,
            slot_params_selection_reason: None,
        };

        let subjects_box = model.subjects_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SlotsInput::Update(subjects, teachers, week_patterns, slots) => {
                self.subjects = subjects;
                self.teachers = teachers;
                self.week_patterns = week_patterns;
                self.slots = slots;

                let new_data: Vec<_> = self
                    .subjects
                    .ordered_subject_list
                    .iter()
                    .filter_map(|(id, desc)| {
                        if desc.parameters.interrogation_parameters.is_none() {
                            return None;
                        }

                        let subject_slots = self
                            .slots
                            .subject_map
                            .get(id)
                            .expect("Subject should appear in slots if it can have interrogations")
                            .clone();
                        Some(slots_display::EntryData {
                            subject_params: desc.parameters.clone(),
                            subject_id: id.clone(),
                            teachers: self.filter_teachers(*id),
                            week_patterns: self.week_patterns.clone(),
                            subject_slots,
                        })
                    })
                    .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    new_data.into_iter(),
                    |data| slots_display::EntryInput::UpdateData(data),
                );
            }

            SlotsInput::MoveSlotUp(slot_id) => {
                sender.output(SlotsUpdateOp::MoveSlotUp(slot_id)).unwrap();
            }
            SlotsInput::MoveSlotDown(slot_id) => {
                sender.output(SlotsUpdateOp::MoveSlotDown(slot_id)).unwrap();
            }
            SlotsInput::DeleteSlot(slot_id) => {
                sender.output(SlotsUpdateOp::DeleteSlot(slot_id)).unwrap();
            }
            SlotsInput::EditSlot(slot_id) => {
                self.slot_params_selection_reason = Some(SlotParamsSelectionReason::Edit(slot_id));
                let current_slot = self
                    .slots
                    .find_slot(slot_id)
                    .expect("Slot ID should be valid")
                    .clone();
                let (subject_id, _pos) = self
                    .slots
                    .find_slot_subject_and_position(slot_id)
                    .expect("Slot ID should be valid");
                let subject_name = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .parameters
                    .name
                    .clone();
                let teachers = self.filter_teachers(subject_id);
                self.slot_params_dialog
                    .sender()
                    .send(slot_params::DialogInput::Show(
                        subject_name,
                        teachers,
                        self.week_patterns.clone(),
                        current_slot,
                    ))
                    .unwrap();
            }
            SlotsInput::AddSlot(subject_id) => {
                self.slot_params_selection_reason =
                    Some(SlotParamsSelectionReason::New(subject_id));
                let subject_name = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .parameters
                    .name
                    .clone();
                let teachers = self.filter_teachers(subject_id);
                let teacher_id = teachers
                    .iter()
                    .next()
                    .expect("There should be at least one teacher for the subject")
                    .0
                    .clone();
                let default_slot = collomatique_state_colloscopes::slots::Slot {
                    teacher_id,
                    start_time: collomatique_time::SlotStart {
                        weekday: collomatique_time::Weekday(chrono::Weekday::Mon),
                        start_time: collomatique_time::WholeMinuteTime::new(
                            chrono::NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
                        )
                        .unwrap(),
                    },
                    extra_info: String::new(),
                    week_pattern: None,
                    cost: 0,
                };
                self.slot_params_dialog
                    .sender()
                    .send(slot_params::DialogInput::Show(
                        subject_name,
                        teachers,
                        self.week_patterns.clone(),
                        default_slot,
                    ))
                    .unwrap();
            }
            SlotsInput::SlotParamsSelected(params) => {
                let reason = self
                    .slot_params_selection_reason
                    .take()
                    .expect("There should be a reason for slot parameter edition");

                match reason {
                    SlotParamsSelectionReason::Edit(slot_id) => {
                        sender
                            .output(SlotsUpdateOp::UpdateSlot(slot_id, params))
                            .unwrap();
                    }
                    SlotParamsSelectionReason::New(subject_id) => {
                        sender
                            .output(SlotsUpdateOp::AddNewSlot(subject_id, params))
                            .unwrap();
                    }
                }
            }
        }
    }
}

impl Slots {
    fn filter_teachers(
        &self,
        subject_id: collomatique_state_colloscopes::SubjectId,
    ) -> BTreeMap<
        collomatique_state_colloscopes::TeacherId,
        collomatique_state_colloscopes::teachers::Teacher,
    > {
        self.teachers
            .teacher_map
            .iter()
            .filter_map(|(teacher_id, teacher)| {
                if teacher.subjects.contains(&subject_id) {
                    Some((teacher_id.clone(), teacher.clone()))
                } else {
                    None
                }
            })
            .collect()
    }
}
