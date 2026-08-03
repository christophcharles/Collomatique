use std::collections::BTreeSet;

use adw::prelude::{ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use adw::prelude::ActionRowExt;

use crate::tools::message_row::{MessageRow, MessageSeverity};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subject_name: String,
    ordered_slots: Vec<(collomatique_state_colloscopes::SlotId, String)>,
    periods: collomatique_state_colloscopes::periods::Periods,
    subject_excluded_periods: BTreeSet<collomatique_state_colloscopes::PeriodId>,

    antecedent_condition_selected: u32,
    antecedent_slot_selected: u32,
    consequent_condition_selected: u32,
    consequent_slot_selected: u32,
    soft: bool,

    period_data: Vec<PeriodData>,
    period_entries: FactoryVecDeque<PeriodEntry>,
    messages: FactoryVecDeque<MessageRow>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        String,
        Vec<(collomatique_state_colloscopes::SlotId, String)>,
        collomatique_state_colloscopes::periods::Periods,
        BTreeSet<collomatique_state_colloscopes::PeriodId>,
        collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    ),
    Cancel,
    Accept,

    UpdateAntecedentCondition(u32),
    UpdateAntecedentSlot(u32),
    UpdateConsequentCondition(u32),
    UpdateConsequentSlot(u32),
    UpdateSoft(bool),
    UpdatePeriodExclusion(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::slot_pairings::SlotPairingRule),
}

/// « utilisé ⇒ non utilisé » is the one shape whose constraint needs the
/// antecedent *negated*, which only works directly when an interrogation holds
/// at most one group; otherwise `constraints-colloscopes` has to reify an
/// intermediate binary variable and linearize it (see
/// `pairings::slot::emit_pairing_constraint`).
///
/// The warning is shown whenever this shape is selected, not only when the
/// precondition currently holds: the subject's group count can be changed long
/// after the rule is validated, and the user would then never see the message.
/// Naming the precondition in the text lets them either pick another shape now,
/// or keep this one and know what to avoid later.
const HEAVY_SHAPE_WARNING: &str = "La forme « créneau utilisé ⇒ créneau non utilisé » est \
    coûteuse pour le solveur si une interrogation peut accueillir plusieurs groupes : elle \
    nécessite alors des variables intermédiaires supplémentaires.";

/// « non utilisé ⇒ utilisé » compiles to `antécédent + conséquent ≥ 1`, which is
/// symmetric: the same single constraint also enforces the converse. The
/// wording stays conditional because this is *not* a logical rewriting of the
/// other shapes — it only helps when it happens to express the user's need.
const FAVORED_SHAPE_HINT: &str = "Si votre besoin peut s'exprimer sous la forme « créneau non \
    utilisé ⇒ créneau utilisé », préférez-la : c'est la plus efficace pour le solveur et elle \
    impose aussi automatiquement la règle réciproque.";

impl Dialog {
    fn generate_slots_model(&self) -> gtk::StringList {
        let slot_names: Vec<_> = self
            .ordered_slots
            .iter()
            .map(|(_id, name)| name.as_str())
            .collect();
        gtk::StringList::new(&slot_names[..])
    }

    fn generate_conditions_model() -> gtk::StringList {
        gtk::StringList::new(&["Créneau utilisé", "Créneau non utilisé"])
    }

    fn slot_id_to_selected(&self, slot_id: collomatique_state_colloscopes::SlotId) -> u32 {
        for (i, (id, _)) in self.ordered_slots.iter().enumerate() {
            if *id == slot_id {
                return i as u32;
            }
        }
        0
    }

    fn slot_selected_to_id(&self, selected: u32) -> collomatique_state_colloscopes::SlotId {
        self.ordered_slots[selected as usize].0
    }

    fn update_data_from_rule(
        &mut self,
        rule: &collomatique_state_colloscopes::slot_pairings::SlotPairingRule,
    ) {
        self.antecedent_condition_selected = if rule.antecedent().should_have { 0 } else { 1 };
        self.antecedent_slot_selected = self.slot_id_to_selected(rule.antecedent().slot_id);
        self.consequent_condition_selected = if rule.consequent().should_have { 0 } else { 1 };
        self.consequent_slot_selected = self.slot_id_to_selected(rule.consequent().slot_id);
        self.soft = rule.soft();

        self.period_data = self
            .periods
            .period_ids()
            .enumerate()
            .map(|(i, period_id)| {
                let period_id = &period_id;
                let subject_excluded = self.subject_excluded_periods.contains(period_id);
                PeriodData {
                    period_index: i,
                    enabled: !rule.excluded_periods().contains(period_id),
                    subject_excluded,
                }
            })
            .collect();
    }

    fn rebuild_period_entries(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.period_entries,
            self.period_data.iter().cloned(),
            PeriodInput::UpdateData,
        );
    }

    fn build_rule_from_data(
        &self,
    ) -> collomatique_state_colloscopes::slot_pairings::SlotPairingRule {
        let excluded_periods: BTreeSet<_> = self
            .period_data
            .iter()
            .enumerate()
            .filter_map(|(i, pd)| {
                if !pd.enabled {
                    self.periods.period_id_at(i)
                } else {
                    None
                }
            })
            .collect();

        collomatique_state_colloscopes::slot_pairings::SlotPairingRule::new(
            collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                slot_id: self.slot_selected_to_id(self.antecedent_slot_selected),
                should_have: self.antecedent_condition_selected == 0,
            },
            collomatique_state_colloscopes::slot_pairings::SlotRulePart {
                slot_id: self.slot_selected_to_id(self.consequent_slot_selected),
                should_have: self.consequent_condition_selected == 0,
            },
            excluded_periods,
            self.soft,
        )
        .expect("the Valider button is insensitive while both parts share a slot")
    }

    fn slots_are_same(&self) -> bool {
        if self.ordered_slots.is_empty() {
            return true;
        }
        self.antecedent_slot_selected == self.consequent_slot_selected
    }

    /// Why « Valider » is greyed out, if it is. Doubles as the button's
    /// tooltip: [None] both hides the error row and clears the tooltip.
    fn error_message(&self) -> Option<&'static str> {
        self.slots_are_same()
            .then_some("L'antécédent et le conséquent doivent porter sur deux créneaux différents.")
    }

    /// The two `should_have` flags of the rule being edited, in the order
    /// (antécédent, conséquent) — index 0 of the conditions model is
    /// « Créneau utilisé ».
    fn selected_shape(&self) -> (bool, bool) {
        (
            self.antecedent_condition_selected == 0,
            self.consequent_condition_selected == 0,
        )
    }

    /// Refills the message area at the bottom of the dialog. Called after
    /// every input, so the rows always describe the current selection.
    fn update_messages(&mut self) {
        let mut messages = Vec::new();
        if let Some(error) = self.error_message() {
            messages.push((MessageSeverity::Error, error.to_string()));
        }
        match self.selected_shape() {
            (true, false) => {
                messages.push((MessageSeverity::Warning, HEAVY_SHAPE_WARNING.to_string()));
                messages.push((MessageSeverity::Info, FAVORED_SHAPE_HINT.to_string()));
            }
            // Already the cheapest shape: nothing to nudge towards.
            (false, true) => {}
            _ => messages.push((MessageSeverity::Info, FAVORED_SHAPE_HINT.to_string())),
        }

        let mut guard = self.messages.guard();
        guard.clear();
        for message in messages {
            guard.push_back(message);
        }
    }
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_title: Some("Configuration de l'appariement de créneaux"),
            set_default_size: (500, 400),
            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        connect_clicked => DialogInput::Cancel,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider",
                        add_css_class: "suggested-action",
                        #[watch]
                        set_sensitive: !model.slots_are_same(),
                        #[watch]
                        set_tooltip_text: model.error_message(),
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_orientation: gtk::Orientation::Vertical,
                    #[name(scrolled_window)]
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        gtk::Box {
                            set_hexpand: true,
                            set_margin_all: 5,
                            set_spacing: 10,
                            set_orientation: gtk::Orientation::Vertical,
                            adw::PreferencesGroup {
                                set_title: "Antécédent",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::ComboRow {
                                    set_title: "Condition",
                                    #[track(model.should_redraw)]
                                    set_model: Some(&Dialog::generate_conditions_model()),
                                    #[track(model.should_redraw)]
                                    set_selected: model.antecedent_condition_selected,
                                    connect_selected_notify[sender] => move |widget| {
                                        let selected = widget.selected();
                                        sender.input(DialogInput::UpdateAntecedentCondition(selected));
                                    },
                                },
                                adw::ComboRow {
                                    set_title: "Créneau",
                                    #[track(model.should_redraw)]
                                    set_model: Some(&model.generate_slots_model()),
                                    #[track(model.should_redraw)]
                                    set_selected: model.antecedent_slot_selected,
                                    connect_selected_notify[sender] => move |widget| {
                                        let selected = widget.selected();
                                        sender.input(DialogInput::UpdateAntecedentSlot(selected));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Conséquent",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::ComboRow {
                                    set_title: "Condition",
                                    #[track(model.should_redraw)]
                                    set_model: Some(&Dialog::generate_conditions_model()),
                                    #[track(model.should_redraw)]
                                    set_selected: model.consequent_condition_selected,
                                    connect_selected_notify[sender] => move |widget| {
                                        let selected = widget.selected();
                                        sender.input(DialogInput::UpdateConsequentCondition(selected));
                                    },
                                },
                                adw::ComboRow {
                                    set_title: "Créneau",
                                    #[track(model.should_redraw)]
                                    set_model: Some(&model.generate_slots_model()),
                                    #[track(model.should_redraw)]
                                    set_selected: model.consequent_slot_selected,
                                    connect_selected_notify[sender] => move |widget| {
                                        let selected = widget.selected();
                                        sender.input(DialogInput::UpdateConsequentSlot(selected));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Options",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_title: "Contrainte souple",
                                    set_subtitle: "Si activé, la contrainte sera satisfaite au mieux mais pourra être violée",
                                    #[track(model.should_redraw)]
                                    set_active: model.soft,
                                    connect_active_notify[sender] => move |widget| {
                                        let active = widget.is_active();
                                        sender.input(DialogInput::UpdateSoft(active));
                                    },
                                },
                            },
                            #[local_ref]
                            period_list -> adw::PreferencesGroup {
                                set_title: "Périodes concernées",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: !model.period_data.is_empty(),
                            },
                            gtk::Label {
                                set_margin_all: 5,
                                #[watch]
                                set_label: &format!("Matière concernée : {}", model.subject_name),
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                        },
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Vertical,
                        set_hexpand: true,
                        set_spacing: 5,
                        set_margin_all: 5,
                        #[watch]
                        set_visible: !model.messages.is_empty(),
                        gtk::ScrolledWindow {
                            set_propagate_natural_height: true,
                            set_vexpand: false,
                            set_hscrollbar_policy: gtk::PolicyType::Never,
                            set_vscrollbar_policy: gtk::PolicyType::Automatic,
                            #[local_ref]
                            messages_listbox -> gtk::ListBox {
                                set_hexpand: true,
                                add_css_class: "boxed-list",
                                set_selection_mode: gtk::SelectionMode::None,
                            },
                        },
                    },
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let period_entries = FactoryVecDeque::builder()
            .launch(adw::PreferencesGroup::default())
            .forward(sender.input_sender(), |msg| match msg {
                PeriodOutput::UpdateStatus(index, status) => {
                    DialogInput::UpdatePeriodExclusion(index, status)
                }
            });

        let model = Dialog {
            hidden: true,
            should_redraw: false,
            subject_name: String::new(),
            ordered_slots: Vec::new(),
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            subject_excluded_periods: BTreeSet::new(),
            antecedent_condition_selected: 0,
            antecedent_slot_selected: gtk::INVALID_LIST_POSITION,
            consequent_condition_selected: 0,
            consequent_slot_selected: gtk::INVALID_LIST_POSITION,
            soft: false,
            period_data: Vec::new(),
            period_entries,
            messages: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .detach(),
        };

        let period_list = model.period_entries.widget();
        let messages_listbox = model.messages.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(
                subject_name,
                ordered_slots,
                periods,
                subject_excluded_periods,
                rule,
            ) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subject_name = subject_name;
                self.ordered_slots = ordered_slots;
                self.periods = periods;
                self.subject_excluded_periods = subject_excluded_periods;
                self.update_data_from_rule(&rule);
                self.rebuild_period_entries();
            }
            DialogInput::Cancel => {
                self.hidden = true;
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.build_rule_from_data()))
                    .unwrap();
            }
            DialogInput::UpdateAntecedentCondition(selected) => {
                self.antecedent_condition_selected = selected;
            }
            DialogInput::UpdateAntecedentSlot(selected) => {
                self.antecedent_slot_selected = selected;
            }
            DialogInput::UpdateConsequentCondition(selected) => {
                self.consequent_condition_selected = selected;
            }
            DialogInput::UpdateConsequentSlot(selected) => {
                self.consequent_slot_selected = selected;
            }
            DialogInput::UpdateSoft(active) => {
                self.soft = active;
            }
            DialogInput::UpdatePeriodExclusion(index, enabled) => {
                if index < self.period_data.len() {
                    self.period_data[index].enabled = enabled;
                }
            }
        }
        self.update_messages();
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

#[derive(Debug, Clone)]
struct PeriodData {
    period_index: usize,
    enabled: bool,
    subject_excluded: bool,
}

#[derive(Debug)]
struct PeriodEntry {
    data: PeriodData,
    index: DynamicIndex,
    should_redraw: bool,
}

#[derive(Debug, Clone)]
enum PeriodInput {
    UpdateData(PeriodData),
    UpdateStatus(bool),
}

#[derive(Debug)]
enum PeriodOutput {
    UpdateStatus(usize, bool),
}

impl PeriodEntry {
    fn generate_title(&self) -> String {
        if self.data.subject_excluded {
            format!(
                "Période {} (exclue par la matière)",
                self.data.period_index + 1
            )
        } else {
            format!("Période {}", self.data.period_index + 1)
        }
    }
}

#[relm4::factory]
impl FactoryComponent for PeriodEntry {
    type Init = PeriodData;
    type Input = PeriodInput;
    type Output = PeriodOutput;
    type CommandOutput = ();
    type ParentWidget = adw::PreferencesGroup;

    view! {
        #[root]
        adw::SwitchRow {
            set_hexpand: true,
            set_use_markup: false,
            #[watch]
            set_title: &self.generate_title(),
            #[track(self.should_redraw)]
            set_active: self.data.enabled,
            connect_active_notify[sender] => move |widget| {
                let status = widget.is_active();
                sender.input(PeriodInput::UpdateStatus(status));
            },
        }
    }

    fn init_model(data: Self::Init, index: &DynamicIndex, _sender: FactorySender<Self>) -> Self {
        Self {
            data,
            index: index.clone(),
            should_redraw: false,
        }
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

    fn update(&mut self, msg: Self::Input, sender: FactorySender<Self>) {
        self.should_redraw = false;
        match msg {
            PeriodInput::UpdateData(new_data) => {
                self.data = new_data;
                self.should_redraw = true;
            }
            PeriodInput::UpdateStatus(new_status) => {
                if self.data.enabled == new_status {
                    return;
                }
                self.data.enabled = new_status;
                sender
                    .output(PeriodOutput::UpdateStatus(
                        self.index.current_index(),
                        new_status,
                    ))
                    .unwrap();
            }
        }
    }
}
