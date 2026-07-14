use std::collections::BTreeSet;

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};
use relm4::{ComponentController, gtk};

use collomatique_ops::SlotPairingsUpdateOp;

pub mod slot_pairing_params;
pub mod slot_pairings_display;

#[derive(Debug)]
pub enum SlotPairingsInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::teachers::Teachers,
        collomatique_state_colloscopes::slots::Slots,
        collomatique_state_colloscopes::slot_pairings::SlotPairings,
        collomatique_state_colloscopes::periods::Periods,
    ),

    SlotPairingParamsSelected(collomatique_state_colloscopes::slot_pairings::SlotPairingRule),
    DeleteSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
    EditSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
    AddSlotPairing(collomatique_state_colloscopes::SubjectId),
}

#[derive(Debug)]
enum SlotPairingParamsSelectionReason {
    New(collomatique_state_colloscopes::SubjectId),
    Edit(collomatique_state_colloscopes::SlotPairingRuleId),
}

pub struct SlotPairings {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    slots: collomatique_state_colloscopes::slots::Slots,
    slot_pairings: collomatique_state_colloscopes::slot_pairings::SlotPairings,
    periods: collomatique_state_colloscopes::periods::Periods,
    teachers: collomatique_state_colloscopes::teachers::Teachers,
    subjects_list: FactoryVecDeque<slot_pairings_display::Entry>,

    slot_pairing_params_dialog: Controller<slot_pairing_params::Dialog>,
    slot_pairing_params_selection_reason: Option<SlotPairingParamsSelectionReason>,
}

impl SlotPairings {
    fn build_slot_description(
        slot: &collomatique_state_colloscopes::slots::Slot,
        teachers: &collomatique_state_colloscopes::teachers::Teachers,
    ) -> String {
        let teacher_name = teachers
            .teacher_map
            .get(&slot.teacher_id)
            .map(|t| format!("{} {}", t.desc.firstname, t.desc.surname))
            .unwrap_or_else(|| "???".into());
        let time_text = slot.start_time.capitalize();
        format!("{} - {}", teacher_name, time_text)
    }

    fn ordered_slots_for_subject(
        &self,
        subject_id: collomatique_state_colloscopes::SubjectId,
    ) -> Vec<(collomatique_state_colloscopes::SlotId, String)> {
        self.slots
            .subject_map
            .get(&subject_id)
            .map(|subject_slots| {
                subject_slots
                    .ordered_slots
                    .iter()
                    .map(|(slot_id, slot)| {
                        (*slot_id, Self::build_slot_description(slot, &self.teachers))
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_slot_subject(
        &self,
        slot_id: collomatique_state_colloscopes::SlotId,
    ) -> Option<collomatique_state_colloscopes::SubjectId> {
        for (subject_id, subject_slots) in &self.slots.subject_map {
            for (sid, _) in &subject_slots.ordered_slots {
                if *sid == slot_id {
                    return Some(*subject_id);
                }
            }
        }
        None
    }
}

#[relm4::component(pub)]
impl Component for SlotPairings {
    type Input = SlotPairingsInput;
    type Output = SlotPairingsUpdateOp;
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
                slot_pairings_display::EntryOutput::DeleteSlotPairing(rule_id) => {
                    SlotPairingsInput::DeleteSlotPairing(rule_id)
                }
                slot_pairings_display::EntryOutput::EditSlotPairing(rule_id) => {
                    SlotPairingsInput::EditSlotPairing(rule_id)
                }
                slot_pairings_display::EntryOutput::AddSlotPairing(subject_id) => {
                    SlotPairingsInput::AddSlotPairing(subject_id)
                }
            });

        let slot_pairing_params_dialog = slot_pairing_params::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                slot_pairing_params::DialogOutput::Accepted(rule) => {
                    SlotPairingsInput::SlotPairingParamsSelected(rule)
                }
            });

        let model = SlotPairings {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            slots: collomatique_state_colloscopes::slots::Slots::default(),
            slot_pairings: collomatique_state_colloscopes::slot_pairings::SlotPairings::default(),
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            teachers: collomatique_state_colloscopes::teachers::Teachers::default(),
            subjects_list,
            slot_pairing_params_dialog,
            slot_pairing_params_selection_reason: None,
        };

        let subjects_box = model.subjects_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            SlotPairingsInput::Update(subjects, teachers, slots, slot_pairings, periods) => {
                self.subjects = subjects;
                self.teachers = teachers;
                self.slots = slots;
                self.slot_pairings = slot_pairings;
                self.periods = periods;

                let new_data: Vec<_> = self
                    .subjects
                    .ordered_subject_list
                    .entries()
                    .filter_map(|(id, desc)| {
                        let id = &id;
                        desc.parameters.interrogation_parameters.as_ref()?;

                        let subject_slots = self
                            .slots
                            .subject_map
                            .get(id)
                            .expect("Subject should appear in slots if it can have interrogations")
                            .clone();

                        // Collect slot pairing rules for this subject
                        let rules: Vec<_> = self
                            .slot_pairings
                            .slot_pairing_rule_map
                            .entries()
                            .filter(|(_rule_id, rule)| {
                                // Check if antecedent slot belongs to this subject
                                subject_slots
                                    .ordered_slots
                                    .iter()
                                    .any(|(slot_id, _)| *slot_id == rule.antecedent.slot_id)
                            })
                            .map(|(rule_id, rule)| (rule_id, rule.clone()))
                            .collect();

                        // Build slot descriptions for this subject
                        let slot_descriptions: Vec<_> = subject_slots
                            .ordered_slots
                            .iter()
                            .map(|(slot_id, slot)| {
                                (*slot_id, Self::build_slot_description(slot, &self.teachers))
                            })
                            .collect();

                        Some(slot_pairings_display::EntryData {
                            subject_id: *id,
                            subject_name: desc.parameters.name.clone(),
                            rules,
                            slot_descriptions,
                            periods: self.periods.clone(),
                        })
                    })
                    .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.subjects_list,
                    new_data.into_iter(),
                    slot_pairings_display::EntryInput::UpdateData,
                );
            }

            SlotPairingsInput::DeleteSlotPairing(rule_id) => {
                sender
                    .output(SlotPairingsUpdateOp::DeleteSlotPairingRule(rule_id))
                    .unwrap();
            }
            SlotPairingsInput::EditSlotPairing(rule_id) => {
                self.slot_pairing_params_selection_reason =
                    Some(SlotPairingParamsSelectionReason::Edit(rule_id));
                let current_rule = self
                    .slot_pairings
                    .slot_pairing_rule_map
                    .get(&rule_id)
                    .expect("Rule ID should be valid")
                    .clone();
                let subject_id = self
                    .find_slot_subject(current_rule.antecedent.slot_id)
                    .expect("Antecedent slot should belong to a subject");
                let subject_name = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .parameters
                    .name
                    .clone();
                let ordered_slots = self.ordered_slots_for_subject(subject_id);
                let subject_excluded_periods = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .excluded_periods
                    .clone();
                self.slot_pairing_params_dialog
                    .sender()
                    .send(slot_pairing_params::DialogInput::Show(
                        subject_name,
                        ordered_slots,
                        self.periods.clone(),
                        subject_excluded_periods,
                        current_rule,
                    ))
                    .unwrap();
            }
            SlotPairingsInput::AddSlotPairing(subject_id) => {
                self.slot_pairing_params_selection_reason =
                    Some(SlotPairingParamsSelectionReason::New(subject_id));
                let subject_name = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .parameters
                    .name
                    .clone();
                let ordered_slots = self.ordered_slots_for_subject(subject_id);
                let subject_excluded_periods = self
                    .subjects
                    .find_subject(subject_id)
                    .expect("Subject ID should be valid")
                    .excluded_periods
                    .clone();
                let first_slot_id = ordered_slots
                    .first()
                    .expect("There should be at least one slot for the subject")
                    .0;
                let second_slot_id = ordered_slots
                    .get(1)
                    .map(|(id, _)| *id)
                    .unwrap_or(first_slot_id);
                let default_rule = collomatique_state_colloscopes::slot_pairings::SlotPairingRule {
                    antecedent: collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                        slot_id: first_slot_id,
                        should_have: true,
                    },
                    consequent: collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                        slot_id: second_slot_id,
                        should_have: true,
                    },
                    excluded_periods: BTreeSet::new(),
                    soft: false,
                };
                self.slot_pairing_params_dialog
                    .sender()
                    .send(slot_pairing_params::DialogInput::Show(
                        subject_name,
                        ordered_slots,
                        self.periods.clone(),
                        subject_excluded_periods,
                        default_rule,
                    ))
                    .unwrap();
            }
            SlotPairingsInput::SlotPairingParamsSelected(rule) => {
                let reason = self
                    .slot_pairing_params_selection_reason
                    .take()
                    .expect("There should be a reason for slot pairing parameter edition");

                match reason {
                    SlotPairingParamsSelectionReason::Edit(rule_id) => {
                        sender
                            .output(SlotPairingsUpdateOp::UpdateSlotPairingRule(rule_id, rule))
                            .unwrap();
                    }
                    SlotPairingParamsSelectionReason::New(_subject_id) => {
                        sender
                            .output(SlotPairingsUpdateOp::AddNewSlotPairingRule(rule))
                            .unwrap();
                    }
                }
            }
        }
    }
}
