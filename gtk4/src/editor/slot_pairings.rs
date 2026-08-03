use std::collections::BTreeSet;

use gtk::prelude::{BoxExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};
use relm4::{ComponentController, gtk};

use collomatique_ops::SlotPairingsUpdateOp;

use crate::tools::message_row::MessageSeverity;

pub mod slot_pairing_params;
pub mod slot_pairings_display;

/// One remark about a slot pairing rule.
///
/// Both the edition dialog — which shows them as full text rows — and the list
/// of recorded rules read the same variants, so a rule reads the same way
/// wherever it is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleMessage {
    /// Both parts name the same slot: the rule cannot be recorded.
    SameSlot,
    /// « créneau utilisé ⇒ créneau non utilisé » — the shape that may need a
    /// reified extra.
    HeavyShape,
    /// Nudge towards the symmetric « créneau non utilisé ⇒ créneau utilisé »
    /// shape.
    FavoredShape,
}

impl RuleMessage {
    pub fn severity(self) -> MessageSeverity {
        match self {
            RuleMessage::SameSlot => MessageSeverity::Error,
            RuleMessage::HeavyShape => MessageSeverity::Warning,
            RuleMessage::FavoredShape => MessageSeverity::Info,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            RuleMessage::SameSlot => {
                "L'antécédent et le conséquent doivent porter sur deux créneaux différents."
            }
            // « utilisé ⇒ non utilisé » is the one shape whose constraint needs
            // the antecedent *negated*, which only works directly when an
            // interrogation holds at most one group; otherwise
            // `constraints-colloscopes` has to reify an intermediate binary
            // variable and linearize it (see
            // `pairings::slot::emit_pairing_constraint`).
            //
            // The warning is shown whenever this shape is used, not only when
            // the precondition currently holds: the subject's group count can be
            // changed long after the rule is validated, and the user would then
            // never see the message. Naming the precondition in the text lets
            // them either pick another shape now, or keep this one and know what
            // to avoid later.
            RuleMessage::HeavyShape => {
                "La forme « créneau utilisé ⇒ créneau non utilisé » est coûteuse pour le solveur \
                 si une interrogation peut accueillir plusieurs groupes : elle nécessite alors des \
                 variables intermédiaires supplémentaires."
            }
            // « non utilisé ⇒ utilisé » compiles to `antécédent + conséquent ≥ 1`,
            // which is symmetric: the same single constraint also enforces the
            // converse. The wording stays conditional because this is *not* a
            // logical rewriting of the other shapes — it only helps when it
            // happens to express the user's need.
            RuleMessage::FavoredShape => {
                "Si votre besoin peut s'exprimer sous la forme « créneau non utilisé ⇒ créneau \
                 utilisé », préférez-la : c'est la plus efficace pour le solveur et elle impose \
                 aussi automatiquement la règle réciproque."
            }
        }
    }
}

/// The remarks a rule deserves, most severe first.
///
/// `shape` is the pair of `should_have` flags, in the order (antécédent,
/// conséquent). `slots_are_same` is only ever true in the edition dialog: a rule
/// that made it into the document always names two distinct slots.
pub fn rule_messages(shape: (bool, bool), slots_are_same: bool) -> Vec<RuleMessage> {
    let mut messages = Vec::new();
    if slots_are_same {
        messages.push(RuleMessage::SameSlot);
    }
    match shape {
        (true, false) => {
            messages.push(RuleMessage::HeavyShape);
            messages.push(RuleMessage::FavoredShape);
        }
        // Already the cheapest shape: nothing to nudge towards.
        (false, true) => {}
        _ => messages.push(RuleMessage::FavoredShape),
    }
    messages
}

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
    /// This whole tab is grouped by subject, so a slot is named without its
    /// own — exactly [collomatique_ops::rendering::render_slot_in_subject].
    fn build_slot_description(&self, slot_id: collomatique_state_colloscopes::SlotId) -> String {
        collomatique_ops::rendering::render_slot_in_subject(&self.teachers, &self.slots, slot_id)
            .expect("the slot comes from the document being displayed")
    }

    fn ordered_slots_for_subject(
        &self,
        subject_id: collomatique_state_colloscopes::SubjectId,
    ) -> Vec<(collomatique_state_colloscopes::SlotId, String)> {
        self.slots
            .slots_for_subject(subject_id)
            .map(|subject_slots| {
                subject_slots
                    .map(|(slot_id, _slot)| (*slot_id, self.build_slot_description(*slot_id)))
                    .collect()
            })
            .unwrap_or_default()
    }

    fn find_slot_subject(
        &self,
        slot_id: collomatique_state_colloscopes::SlotId,
    ) -> Option<collomatique_state_colloscopes::SubjectId> {
        self.slots
            .find_slot_subject_and_position(slot_id)
            .map(|(subject_id, _)| subject_id)
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
                    .iter()
                    .filter_map(|(id, desc)| {
                        let id = &id;
                        desc.parameters.interrogation_parameters.as_ref()?;

                        // Sparse slots ordering: a subject with interrogations
                        // but no slots yet has no row; render it with an empty
                        // slot list (matching the pre-sparse dense behavior).
                        let subject_slots =
                            self.slots.slots_vec_for_subject(*id).unwrap_or_default();

                        // Collect slot pairing rules for this subject
                        let rules: Vec<_> = self
                            .slot_pairings
                            .slot_pairing_rule_map
                            .iter()
                            .filter(|(_rule_id, rule)| {
                                // Check if antecedent slot belongs to this subject
                                subject_slots
                                    .iter()
                                    .any(|(slot_id, _)| *slot_id == rule.antecedent().slot_id)
                            })
                            .map(|(rule_id, rule)| (rule_id, rule.clone()))
                            .collect();

                        // Build slot descriptions for this subject
                        let slot_descriptions: Vec<_> = subject_slots
                            .iter()
                            .map(|(slot_id, _slot)| {
                                (*slot_id, self.build_slot_description(*slot_id))
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
                    .find_slot_subject(current_rule.antecedent().slot_id)
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
                let default_rule =
                    collomatique_state_colloscopes::slot_pairings::SlotPairingRule::new(
                        collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                            slot_id: first_slot_id,
                            should_have: true,
                        },
                        collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                            slot_id: second_slot_id,
                            should_have: true,
                        },
                        BTreeSet::new(),
                        false,
                    )
                    .expect("the Ajouter button is gated on len() >= 2");
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
