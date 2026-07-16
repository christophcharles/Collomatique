use std::collections::BTreeSet;

use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentParts, ComponentSender, Controller, RelmWidgetExt};
use relm4::{ComponentController, adw, gtk};

use collomatique_ops::PairingsUpdateOp;

mod pairing_params;
mod pairings_display;

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
                    set_visible: model.subjects.ordered_subject_list.len() >= 2,
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
                        subjects: self.subjects.clone(),
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
                let first_subject = self
                    .subjects
                    .ordered_subject_list
                    .get_at(0)
                    .map(|(id, _)| id);
                let second_subject = self
                    .subjects
                    .ordered_subject_list
                    .get_at(1)
                    .map(|(id, _)| id);
                let (ant_id, con_id) = match (first_subject, second_subject) {
                    (Some(a), Some(b)) => (a, b),
                    _ => return, // Need at least 2 subjects
                };
                let default_rule = collomatique_state_colloscopes::pairings::PairingRule {
                    antecedent: collomatique_state_colloscopes::pairings::RulePart {
                        subject_id: ant_id,
                        should_have: true,
                    },
                    consequent: collomatique_state_colloscopes::pairings::RulePart {
                        subject_id: con_id,
                        should_have: true,
                    },
                    excluded_periods: BTreeSet::new(),
                    soft: false,
                };
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
