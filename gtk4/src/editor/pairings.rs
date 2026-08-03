use std::collections::BTreeSet;

use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};
use relm4::{ComponentController, adw, gtk};

use collomatique_ops::PairingsUpdateOp;

use crate::tools::messages::MessageSeverity;

mod pairing_params;
mod pairings_display;

/// One remark about a pairing rule.
///
/// Both the edition dialog — which shows them as full text rows — and the list
/// of recorded rules read the same variants, so a rule reads the same way
/// wherever it is displayed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleMessage {
    /// Both parts name the same subject: the rule cannot be recorded.
    SameSubject,
    /// « avoir ⇒ ne pas avoir » — the shape that may need a reified extra.
    HeavyShape,
    /// Nudge towards the symmetric « ne pas avoir ⇒ avoir » shape.
    FavoredShape,
}

impl RuleMessage {
    pub fn severity(self) -> MessageSeverity {
        match self {
            RuleMessage::SameSubject => MessageSeverity::Error,
            RuleMessage::HeavyShape => MessageSeverity::Warning,
            RuleMessage::FavoredShape => MessageSeverity::Info,
        }
    }

    pub fn text(self) -> &'static str {
        match self {
            RuleMessage::SameSubject => {
                "L'antécédent et le conséquent doivent porter sur deux matières différentes."
            }
            // « avoir ⇒ ne pas avoir » is the one shape whose constraint needs
            // the antecedent *negated*, which only works directly when a student
            // can have at most one interrogation of that subject per week;
            // otherwise `constraints-colloscopes` has to reify an intermediate
            // binary variable and linearize it (see
            // `pairings::subject::emit_pairing_constraint`).
            //
            // The warning is shown whenever this shape is used, not only when
            // the precondition currently holds: the antecedent subject's
            // periodicity can be changed long after the rule is validated, and
            // the user would then never see the message. Naming the precondition
            // in the text lets them either pick another shape now, or keep this
            // one and know what to avoid later.
            RuleMessage::HeavyShape => {
                "La forme « avoir ⇒ ne pas avoir » est coûteuse pour le solveur si la matière de \
                 l'antécédent peut avoir plusieurs interrogations la même semaine (séparation \
                 minimale de zéro semaine) : elle nécessite alors des variables intermédiaires \
                 supplémentaires."
            }
            // « ne pas avoir ⇒ avoir » compiles to `antécédent + conséquent ≥ 1`,
            // which is symmetric: the same single constraint also enforces the
            // converse. The wording stays conditional because this is *not* a
            // logical rewriting of the other shapes — it only helps when it
            // happens to express the user's need.
            RuleMessage::FavoredShape => {
                "Si votre besoin peut s'exprimer sous la forme « ne pas avoir ⇒ avoir », \
                 préférez-la : c'est la plus efficace pour le solveur et elle impose aussi \
                 automatiquement la règle réciproque."
            }
        }
    }
}

/// The `should_have` flags of a recorded rule, in the order (antécédent,
/// conséquent) — the same pair the dialog reads off its two condition combos.
pub fn rule_shape(rule: &collomatique_state_colloscopes::pairings::PairingRule) -> (bool, bool) {
    (rule.antecedent().should_have, rule.consequent().should_have)
}

/// The remarks a rule deserves, most severe first.
///
/// `shape` is the pair of `should_have` flags, in the order (antécédent,
/// conséquent). `subjects_are_same` is only ever true in the edition dialog: a
/// rule that made it into the document always names two distinct subjects.
pub fn rule_messages(shape: (bool, bool), subjects_are_same: bool) -> Vec<RuleMessage> {
    let mut messages = Vec::new();
    if subjects_are_same {
        messages.push(RuleMessage::SameSubject);
    }
    match shape {
        // The warning already names this shape, and it is symmetric anyway
        // (« A ⇒ ne pas B » is « B ⇒ ne pas A »), so the nudge would add nothing.
        (true, false) => messages.push(RuleMessage::HeavyShape),
        // Already the cheapest shape: nothing to nudge towards.
        (false, true) => {}
        _ => messages.push(RuleMessage::FavoredShape),
    }
    messages
}

#[derive(Debug)]
pub enum PairingsInput {
    Update(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::pairings::Pairings,
    ),

    PairingParamsSelected(collomatique_state_colloscopes::pairings::PairingRule),
    DeletePairing(collomatique_state_colloscopes::PairingRuleId),
    EditPairing(collomatique_state_colloscopes::PairingRuleId),
    AddPairing,
}

#[derive(Debug)]
enum PairingParamsSelectionReason {
    New,
    Edit(collomatique_state_colloscopes::PairingRuleId),
}

pub struct Pairings {
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    periods: collomatique_state_colloscopes::periods::Periods,
    pairings: collomatique_state_colloscopes::pairings::Pairings,
    pairings_list: FactoryVecDeque<pairings_display::Entry>,

    pairing_params_dialog: Controller<pairing_params::Dialog>,
    pairing_params_selection_reason: PairingParamsSelectionReason,
}

#[relm4::component(pub)]
impl Component for Pairings {
    type Input = PairingsInput;
    type Output = PairingsUpdateOp;
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
                set_spacing: 10,
                gtk::Label {
                    set_halign: gtk::Align::Start,
                    set_label: "<i>Les règles d'appariement ne s'appliquent qu'aux élèves inscrits dans les deux matières concernées.</i>",
                    set_use_markup: true,
                    set_wrap: true,
                    set_margin_bottom: 5,
                },
                #[local_ref]
                pairings_list_box -> gtk::ListBox {
                    set_hexpand: true,
                    add_css_class: "boxed-list",
                    set_selection_mode: gtk::SelectionMode::None,
                    #[watch]
                    set_visible: !model.pairings.pairing_rule_map.is_empty(),
                },
                gtk::Button {
                    set_hexpand: true,
                    set_margin_top: 10,
                    #[watch]
                    set_visible: model.pairable_subjects().count() >= 2,
                    adw::ButtonContent {
                        set_icon_name: "list-add-symbolic",
                        set_label: "Ajouter une règle d'appariement",
                    },
                    connect_clicked => PairingsInput::AddPairing,
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let pairings_list = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.input_sender(), |msg| match msg {
                pairings_display::EntryOutput::DeletePairing(rule_id) => {
                    PairingsInput::DeletePairing(rule_id)
                }
                pairings_display::EntryOutput::EditPairing(rule_id) => {
                    PairingsInput::EditPairing(rule_id)
                }
            });

        let pairing_params_dialog = pairing_params::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                pairing_params::DialogOutput::Accepted(rule) => {
                    PairingsInput::PairingParamsSelected(rule)
                }
            });

        let model = Pairings {
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            pairings: collomatique_state_colloscopes::pairings::Pairings::default(),
            pairings_list,
            pairing_params_dialog,
            pairing_params_selection_reason: PairingParamsSelectionReason::New,
        };

        let pairings_list_box = model.pairings_list.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            PairingsInput::Update(subjects, periods, pairings) => {
                self.subjects = subjects;
                self.periods = periods;
                self.pairings = pairings;

                let new_data: Vec<_> = self
                    .pairings
                    .pairing_rule_map
                    .iter()
                    .map(|(rule_id, rule)| pairings_display::EntryData {
                        rule_id,
                        rule: rule.clone(),
                        summary: collomatique_ops::rendering::render_pairing_rule(
                            &self.subjects,
                            &self.pairings,
                            rule_id,
                        )
                        .expect("the rule comes from the document being displayed"),
                        periods: self.periods.clone(),
                    })
                    .collect();

                crate::tools::factories::update_vec_deque(
                    &mut self.pairings_list,
                    new_data.into_iter(),
                    pairings_display::EntryInput::UpdateData,
                );
            }

            PairingsInput::DeletePairing(rule_id) => {
                sender
                    .output(PairingsUpdateOp::DeletePairingRule(rule_id))
                    .unwrap();
            }
            PairingsInput::EditPairing(rule_id) => {
                self.pairing_params_selection_reason = PairingParamsSelectionReason::Edit(rule_id);
                let current_rule = self
                    .pairings
                    .pairing_rule_map
                    .get(&rule_id)
                    .expect("Rule ID should be valid")
                    .clone();
                self.pairing_params_dialog
                    .sender()
                    .send(pairing_params::DialogInput::Show(
                        self.subjects.clone(),
                        self.periods.clone(),
                        current_rule,
                    ))
                    .unwrap();
            }
            PairingsInput::AddPairing => {
                self.pairing_params_selection_reason = PairingParamsSelectionReason::New;
                let mut candidates = self.pairable_subjects();
                let (ant_id, con_id) = match (candidates.next(), candidates.next()) {
                    (Some(a), Some(b)) => (a, b),
                    _ => return, // Need at least 2 subjects with interrogations
                };
                let default_rule = collomatique_state_colloscopes::pairings::PairingRule::new(
                    collomatique_state_colloscopes::pairings::RulePart {
                        subject_id: ant_id,
                        should_have: true,
                    },
                    collomatique_state_colloscopes::pairings::RulePart {
                        subject_id: con_id,
                        should_have: true,
                    },
                    BTreeSet::new(),
                    false,
                )
                .expect("ant_id and con_id are the first two distinct pairable subjects");
                self.pairing_params_dialog
                    .sender()
                    .send(pairing_params::DialogInput::Show(
                        self.subjects.clone(),
                        self.periods.clone(),
                        default_rule,
                    ))
                    .unwrap();
            }
            PairingsInput::PairingParamsSelected(rule) => {
                match self.pairing_params_selection_reason {
                    PairingParamsSelectionReason::Edit(rule_id) => {
                        sender
                            .output(PairingsUpdateOp::UpdatePairingRule(rule_id, rule))
                            .unwrap();
                    }
                    PairingParamsSelectionReason::New => {
                        sender
                            .output(PairingsUpdateOp::AddNewPairingRule(rule))
                            .unwrap();
                    }
                }
            }
        }
    }
}

impl Pairings {
    /// The subjects a pairing rule may name, in document order.
    ///
    /// A rule is an implication between two subjects' interrogations, so a
    /// subject running none can only make it vacuous or impossible; the state
    /// layer refuses such a rule outright. Both the « Ajouter » button's
    /// visibility and the default rule that button opens read this, so the
    /// button is only offered when it can produce a rule the backend accepts.
    fn pairable_subjects(
        &self,
    ) -> impl Iterator<Item = collomatique_state_colloscopes::SubjectId> + '_ {
        self.subjects
            .ordered_subject_list
            .iter()
            .filter(|(_, subject)| subject.parameters.interrogation_parameters.is_some())
            .map(|(id, _)| id)
    }
}
