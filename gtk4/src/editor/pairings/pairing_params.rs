use std::collections::BTreeSet;

use adw::prelude::{ComboRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::FactorySender;
use relm4::factory::FactoryView;
use relm4::prelude::{DynamicIndex, FactoryComponent, FactoryVecDeque};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use adw::prelude::ActionRowExt;

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,
    subjects: collomatique_state_colloscopes::subjects::Subjects,
    periods: collomatique_state_colloscopes::periods::Periods,
    ordered_subjects: Vec<(collomatique_state_colloscopes::SubjectId, String)>,

    antecedent_condition_selected: u32,
    antecedent_subject_selected: u32,
    consequent_condition_selected: u32,
    consequent_subject_selected: u32,
    soft: bool,

    period_data: Vec<PeriodData>,
    period_entries: FactoryVecDeque<PeriodEntry>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(
        collomatique_state_colloscopes::subjects::Subjects,
        collomatique_state_colloscopes::periods::Periods,
        collomatique_state_colloscopes::pairings::PairingRule,
    ),
    Cancel,
    Accept,

    UpdateAntecedentCondition(u32),
    UpdateAntecedentSubject(u32),
    UpdateConsequentCondition(u32),
    UpdateConsequentSubject(u32),
    UpdateSoft(bool),
    UpdatePeriodExclusion(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Accepted(collomatique_state_colloscopes::pairings::PairingRule),
}

impl Dialog {
    fn generate_subjects_model(&self) -> gtk::StringList {
        let subject_names: Vec<_> = self
            .ordered_subjects
            .iter()
            .map(|(_id, name)| name.as_str())
            .collect();
        gtk::StringList::new(&subject_names[..])
    }

    fn generate_conditions_model() -> gtk::StringList {
        gtk::StringList::new(&["Avoir une interrogation", "Ne pas avoir d'interrogation"])
    }

    fn subject_id_to_selected(&self, subject_id: collomatique_state_colloscopes::SubjectId) -> u32 {
        for (i, (id, _)) in self.ordered_subjects.iter().enumerate() {
            if *id == subject_id {
                return i as u32;
            }
        }
        0
    }

    fn subject_selected_to_id(&self, selected: u32) -> collomatique_state_colloscopes::SubjectId {
        self.ordered_subjects[selected as usize].0
    }

    fn build_ordered_subjects(&mut self) {
        let mut subjects: Vec<_> = self
            .subjects
            .ordered_subject_list
            .iter()
            .map(|(subject_id, subject)| (subject_id, subject.parameters.name.clone()))
            .collect();
        subjects.sort_by_key(|(id, name)| (name.clone(), *id));
        self.ordered_subjects = subjects;
    }

    fn update_data_from_rule(
        &mut self,
        rule: &collomatique_state_colloscopes::pairings::PairingRule,
    ) {
        self.antecedent_condition_selected = if rule.antecedent.should_have { 0 } else { 1 };
        self.antecedent_subject_selected = self.subject_id_to_selected(rule.antecedent.subject_id);
        self.consequent_condition_selected = if rule.consequent.should_have { 0 } else { 1 };
        self.consequent_subject_selected = self.subject_id_to_selected(rule.consequent.subject_id);
        self.soft = rule.soft;

        self.period_data = self
            .periods
            .ordered_period_list
            .iter()
            .enumerate()
            .map(|(i, (period_id, _desc))| PeriodData {
                period_index: i,
                enabled: !rule.excluded_periods.contains(&period_id),
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

    fn build_rule_from_data(&self) -> collomatique_state_colloscopes::pairings::PairingRule {
        let excluded_periods: BTreeSet<_> = self
            .period_data
            .iter()
            .enumerate()
            .filter_map(|(i, pd)| {
                if !pd.enabled {
                    self.periods.ordered_period_list.get_at(i).map(|(id, _)| id)
                } else {
                    None
                }
            })
            .collect();

        collomatique_state_colloscopes::pairings::PairingRule {
            antecedent: collomatique_state_colloscopes::pairings::RulePart {
                subject_id: self.subject_selected_to_id(self.antecedent_subject_selected),
                should_have: self.antecedent_condition_selected == 0,
            },
            consequent: collomatique_state_colloscopes::pairings::RulePart {
                subject_id: self.subject_selected_to_id(self.consequent_subject_selected),
                should_have: self.consequent_condition_selected == 0,
            },
            excluded_periods,
            soft: self.soft,
        }
    }

    fn subjects_are_same(&self) -> bool {
        if self.ordered_subjects.is_empty() {
            return true;
        }
        self.antecedent_subject_selected == self.consequent_subject_selected
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
            set_title: Some("Configuration de l'appariement"),
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
                        set_sensitive: !model.subjects_are_same(),
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[name(scrolled_window)]
                #[wrap(Some)]
                set_content = &gtk::ScrolledWindow {
                    set_hexpand: true,
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
                                set_title: "Matière",
                                #[track(model.should_redraw)]
                                set_model: Some(&model.generate_subjects_model()),
                                #[track(model.should_redraw)]
                                set_selected: model.antecedent_subject_selected,
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected();
                                    sender.input(DialogInput::UpdateAntecedentSubject(selected));
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
                                set_title: "Matière",
                                #[track(model.should_redraw)]
                                set_model: Some(&model.generate_subjects_model()),
                                #[track(model.should_redraw)]
                                set_selected: model.consequent_subject_selected,
                                connect_selected_notify[sender] => move |widget| {
                                    let selected = widget.selected();
                                    sender.input(DialogInput::UpdateConsequentSubject(selected));
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
            subjects: collomatique_state_colloscopes::subjects::Subjects::default(),
            periods: collomatique_state_colloscopes::periods::Periods::default(),
            ordered_subjects: Vec::new(),
            antecedent_condition_selected: 0,
            antecedent_subject_selected: gtk::INVALID_LIST_POSITION,
            consequent_condition_selected: 0,
            consequent_subject_selected: gtk::INVALID_LIST_POSITION,
            soft: false,
            period_data: Vec::new(),
            period_entries,
        };

        let period_list = model.period_entries.widget();
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(subjects, periods, rule) => {
                self.hidden = false;
                self.should_redraw = true;
                self.subjects = subjects;
                self.periods = periods;
                self.build_ordered_subjects();
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
            DialogInput::UpdateAntecedentSubject(selected) => {
                self.antecedent_subject_selected = selected;
            }
            DialogInput::UpdateConsequentCondition(selected) => {
                self.consequent_condition_selected = selected;
            }
            DialogInput::UpdateConsequentSubject(selected) => {
                self.consequent_subject_selected = selected;
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
        format!("Période {}", self.data.period_index + 1)
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
