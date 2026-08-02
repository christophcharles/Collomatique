use std::collections::BTreeMap;

use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::RelmWidgetExt;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{adw, gtk};

#[derive(Debug, Clone)]
pub struct EntryData {
    pub subject_id: collomatique_state_colloscopes::SubjectId,
    pub subject_name: String,
    pub rules: Vec<(
        collomatique_state_colloscopes::SlotPairingRuleId,
        collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    )>,
    pub slot_descriptions: Vec<(collomatique_state_colloscopes::SlotId, String)>,
    pub periods: collomatique_state_colloscopes::periods::Periods,
}

#[derive(Debug)]
pub struct Entry {
    subject_id: collomatique_state_colloscopes::SubjectId,
    subject_name: String,
    slot_descriptions: Vec<(collomatique_state_colloscopes::SlotId, String)>,
    periods: collomatique_state_colloscopes::periods::Periods,
    rules: FactoryVecDeque<Rule>,
}

#[derive(Debug, Clone)]
pub enum EntryInput {
    UpdateData(EntryData),

    AddSlotPairingClicked,
}

#[derive(Debug)]
pub enum EntryOutput {
    DeleteSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
    EditSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
    AddSlotPairing(collomatique_state_colloscopes::SubjectId),
}

impl Entry {
    fn build_rule_data(
        &self,
        rule_id: collomatique_state_colloscopes::SlotPairingRuleId,
        rule: &collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    ) -> RuleData {
        let slot_desc_map: BTreeMap<_, _> = self
            .slot_descriptions
            .iter()
            .map(|(id, desc)| (*id, desc.clone()))
            .collect();
        RuleData {
            rule_id,
            rule: rule.clone(),
            slot_desc_map,
            periods: self.periods.clone(),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Entry {
    type Init = EntryData;
    type Input = EntryInput;
    type Output = EntryOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::Box;

    view! {
        #[root]
        gtk::Box {
            set_hexpand: true,
            set_orientation: gtk::Orientation::Vertical,
            set_spacing: 10,
            gtk::Label {
                set_halign: gtk::Align::Start,
                #[watch]
                set_label: &self.subject_name,
                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_label: "<i>Aucun appariement de créneaux à afficher</i>",
                set_use_markup: true,
                #[watch]
                set_visible: self.rules.is_empty(),
            },
            #[local_ref]
            rules_list -> gtk::ListBox {
                set_hexpand: true,
                add_css_class: "boxed-list",
                set_selection_mode: gtk::SelectionMode::None,
                #[watch]
                set_visible: !self.rules.is_empty(),
            },
            gtk::Button {
                set_margin_top: 10,
                adw::ButtonContent {
                    set_icon_name: "list-add-symbolic",
                    set_label: "Ajouter un appariement de créneaux",
                },
                #[watch]
                set_sensitive: self.slot_descriptions.len() >= 2,
                connect_clicked => EntryInput::AddSlotPairingClicked,
            }
        },
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, sender: FactorySender<Self>) -> Self {
        let rules = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .forward(sender.output_sender(), |msg| match msg {
                RuleOutput::EditSlotPairing(rule_id) => EntryOutput::EditSlotPairing(rule_id),
                RuleOutput::DeleteSlotPairing(rule_id) => EntryOutput::DeleteSlotPairing(rule_id),
            });

        let mut model = Self {
            subject_id: data.subject_id,
            subject_name: data.subject_name,
            slot_descriptions: data.slot_descriptions,
            periods: data.periods,
            rules,
        };

        let rules_vec: Vec<_> = data
            .rules
            .iter()
            .map(|(rule_id, rule)| model.build_rule_data(*rule_id, rule))
            .collect();
        crate::tools::factories::update_vec_deque(&mut model.rules, rules_vec.into_iter(), |x| {
            RuleInput::UpdateData(x)
        });

        model
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let rules_list = self.rules.widget();
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        match msg {
            EntryInput::UpdateData(new_data) => {
                self.subject_id = new_data.subject_id;
                self.subject_name = new_data.subject_name;
                self.slot_descriptions = new_data.slot_descriptions;
                self.periods = new_data.periods;

                let rules_vec: Vec<_> = new_data
                    .rules
                    .iter()
                    .map(|(rule_id, rule)| self.build_rule_data(*rule_id, rule))
                    .collect();
                crate::tools::factories::update_vec_deque(
                    &mut self.rules,
                    rules_vec.into_iter(),
                    RuleInput::UpdateData,
                );
            }
            EntryInput::AddSlotPairingClicked => {
                sender
                    .output(EntryOutput::AddSlotPairing(self.subject_id))
                    .unwrap();
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuleData {
    pub rule_id: collomatique_state_colloscopes::SlotPairingRuleId,
    pub rule: collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    pub slot_desc_map: BTreeMap<collomatique_state_colloscopes::SlotId, String>,
    pub periods: collomatique_state_colloscopes::periods::Periods,
}

#[derive(Debug)]
pub struct Rule {
    data: RuleData,
}

#[derive(Debug, Clone)]
pub enum RuleInput {
    UpdateData(RuleData),
}

#[derive(Debug)]
pub enum RuleOutput {
    EditSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
    DeleteSlotPairing(collomatique_state_colloscopes::SlotPairingRuleId),
}

impl Rule {
    fn generate_summary(&self) -> String {
        let ant_desc = self.slot_desc(&self.data.rule.antecedent().slot_id);
        let con_desc = self.slot_desc(&self.data.rule.consequent().slot_id);
        let ant_cond = if self.data.rule.antecedent().should_have {
            "utilisé"
        } else {
            "non utilisé"
        };
        let con_cond = if self.data.rule.consequent().should_have {
            "utilisé"
        } else {
            "non utilisé"
        };
        let soft_text = if self.data.rule.soft() {
            " (souple)"
        } else {
            ""
        };
        format!(
            "[{}] {} \u{27F9} [{}] {}{}",
            ant_cond, ant_desc, con_cond, con_desc, soft_text
        )
    }

    fn slot_desc(&self, slot_id: &collomatique_state_colloscopes::SlotId) -> String {
        self.data
            .slot_desc_map
            .get(slot_id)
            .cloned()
            .expect("the rule's slots are slots of the subject this row was built from")
    }

    fn generate_excluded_periods_info(&self) -> String {
        let mut excluded_period_list: Vec<_> = self
            .data
            .rule
            .excluded_periods()
            .iter()
            .map(|period_id| {
                self.data
                    .periods
                    .find_period_position(*period_id)
                    .expect("Period referenced by slot pairing rule should be valid")
                    + 1
            })
            .collect();

        excluded_period_list.sort();

        let excluded_period_list: Vec<_> = excluded_period_list
            .into_iter()
            .map(|x| x.to_string())
            .collect();

        match excluded_period_list.len() {
            0 => String::new(),
            1 => format!("Désactivée sur la période {}", excluded_period_list[0]),
            _ => format!(
                "Désactivée sur les périodes {}",
                collomatique_ops::rendering::join_french(&excluded_period_list)
            ),
        }
    }
}

#[relm4::factory(pub)]
impl FactoryComponent for Rule {
    type Init = RuleData;
    type Input = RuleInput;
    type Output = RuleOutput;
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        root_widget = gtk::Box {
            set_hexpand: true,
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 5,
            gtk::Button {
                set_icon_name: "edit-symbolic",
                add_css_class: "flat",
                connect_clicked[sender, rule_id = self.data.rule_id] => move |_| {
                    sender
                        .output(RuleOutput::EditSlotPairing(rule_id))
                        .unwrap();
                },
                set_tooltip_text: Some("Modifier l'appariement de créneaux"),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_xalign: 0.,
                set_margin_start: 5,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_summary(),
            },
            gtk::Box {
                set_hexpand: true,
            },
            gtk::Label {
                set_halign: gtk::Align::End,
                set_margin_end: 5,
                #[watch]
                set_label: &self.generate_excluded_periods_info(),
                set_attributes: Some(&gtk::pango::AttrList::from_string("style italic, scale 0.8").unwrap()),
                #[watch]
                set_visible: !self.data.rule.excluded_periods().is_empty(),
            },
            gtk::Separator {
                set_orientation: gtk::Orientation::Vertical,
            },
            gtk::Button {
                set_icon_name: "edit-delete-symbolic",
                add_css_class: "flat",
                connect_clicked[sender, rule_id = self.data.rule_id] => move |_| {
                    sender
                        .output(RuleOutput::DeleteSlotPairing(rule_id))
                        .unwrap();
                },
                set_tooltip_text: Some("Supprimer l'appariement de créneaux"),
            },
        }
    }

    fn init_model(data: Self::Init, _index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self { data }
    }

    fn init_widgets(
        &mut self,
        _index: &DynamicIndex,
        root: Self::Root,
        _returned_widget: &<Self::ParentWidget as FactoryView>::ReturnedWidget,
        sender: FactorySender<Self>,
    ) -> Self::Widgets {
        let widgets = view_output!();

        widgets
    }

    fn update(&mut self, msg: Self::Input, _sender: FactorySender<Self>) {
        match msg {
            RuleInput::UpdateData(new_data) => {
                self.data = new_data;
            }
        }
    }
}
