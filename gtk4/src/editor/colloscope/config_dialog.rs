mod advanced_dialog;
mod group_list_group;
mod period_group;

use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};
use relm4::{adw, gtk};

use collomatique_constraints_colloscopes::{
    GroupListRecompute, GroupListSolveData, PeriodSolveData, SolveConfig,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_strategies::ConductorStrategy;

use crate::editor::run_solver::conductor_config;

pub struct Dialog {
    hidden: bool,
    /// The parameters the assembled [`SolveConfig`] will build its model from, set on `Show`.
    params: Parameters,
    /// The problem/solver configuration this window is assembling. For now only the conductor
    /// strategy is tracked; problem-scoping widgets will be added here later.
    strategy: ConductorStrategy,
    /// The advanced solver-configuration dialog, opened via "Paramètres avancés du résolveur".
    conductor_config_dialog: Controller<conductor_config::Dialog>,
    /// The advanced model-parameter dialog (cross-period softening and L1 anchor weight), opened
    /// via the "Paramètres avancés" button.
    advanced_dialog: Controller<advanced_dialog::Dialog>,
    /// One titled [`adw::PreferencesGroup`] per period, shown in the left panel.
    periods_list: FactoryVecDeque<period_group::PeriodGroup>,
    /// One titled [`adw::PreferencesGroup`] per automatic group list, shown in the right panel.
    group_lists_list: FactoryVecDeque<group_list_group::GroupListGroup>,
    /// Per-period switch state backing `periods_list`, indexed by period position.
    periods_data: Vec<period_group::Data>,
    /// Per-automatic-group-list switch state backing `group_lists_list`, indexed by position.
    group_lists_data: Vec<group_list_group::Data>,
    /// Softening of the cross-fixed-period constraints: `Some(weight)` objectifies them, `None`
    /// keeps them hard. Configured through the "Paramètres avancés" dialog.
    objectify_cross_fixed_period: Option<f64>,
    /// L1 anchor penalty weight, configured through the "Paramètres avancés" dialog.
    l1_anchor_weight: f64,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(SolveConfig, ConductorStrategy, Parameters),
    Cancel,
    Accept,
    OpenAdvanced,
    OpenAdvancedParams,
    UpdateStrategy(ConductorStrategy),
    UpdateAdvancedParams(Option<f64>, f64),
    IgnoreOrRefresh,
    SetPeriodRecompute(usize, bool),
    SetPeriodUseCurrent(usize, bool),
    SetGroupListRecompute(usize, bool),
    SetGroupListObjective(usize, bool),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    Accepted(SolveConfig, ConductorStrategy, Parameters),
}

impl Dialog {
    fn is_opt_strategy(&self) -> bool {
        self.strategy == ConductorStrategy::with_parallelism_defaults()
    }

    fn is_search_strategy(&self) -> bool {
        self.strategy == ConductorStrategy::default()
    }

    fn is_other_strategy(&self) -> bool {
        !self.is_opt_strategy() && !self.is_search_strategy()
    }
}

impl Dialog {
    fn has_periods(&self) -> bool {
        !self.params.periods.is_empty()
    }

    fn has_automatic_groups(&self) -> bool {
        self.params
            .group_lists
            .group_list_map
            .iter()
            .any(|(_, group_list)| !group_list.is_prefilled())
    }

    /// One human-readable title per period, in order.
    fn period_titles(&self) -> Vec<String> {
        let global_first_week = self.params.periods.first_week.clone();
        self.params
            .periods
            .period_ids()
            .enumerate()
            .scan(0usize, |first_week_num, (index, id)| {
                let week_count = self.params.weeks.week_count_for_period(id).unwrap_or(0);
                let title = crate::editor::generate_period_title(
                    &global_first_week,
                    index,
                    *first_week_num,
                    week_count,
                );
                *first_week_num += week_count;
                Some(title)
            })
            .collect()
    }

    /// The name of each automatic (non-prefilled) group list, in map order.
    fn group_list_names(&self) -> Vec<String> {
        self.params
            .group_lists
            .group_list_map
            .values()
            .filter(|group_list| !group_list.is_prefilled())
            .map(|group_list| group_list.params.name.clone())
            .collect()
    }

    /// Rebuild the per-period and per-group-list switch state from `config`, reconciled against the
    /// current parameters. Periods and group lists absent from the incoming [`SolveConfig`] (e.g. on
    /// a fresh document) fall back to their defaults via [`SolveConfig::sanitize`].
    fn set_data_from_config(&mut self, config: SolveConfig) {
        let sanitized_config = config.sanitize(&self.params);
        // Look up each period/list by id rather than zipping against `.values()`: the titles are
        // produced in period display order, whereas the config maps iterate in
        // `PeriodId`/`GroupListId` order, which need not match. `sanitize` guarantees an entry for
        // every current period and non-prefilled group list, so the indexing is total.
        self.periods_data = self
            .params
            .periods
            .period_ids()
            .zip(self.period_titles())
            .map(|(id, title)| {
                let data = &sanitized_config.periods[&id];
                period_group::Data {
                    title,
                    recompute: data.recompute,
                    use_current_values: data.use_current_values,
                }
            })
            .collect();
        self.group_lists_data = self
            .params
            .group_lists
            .group_list_map
            .iter()
            .filter(|(_id, group_list)| !group_list.is_prefilled())
            .zip(self.group_list_names())
            .map(|((id, _), title)| {
                let data = &sanitized_config.group_lists[&id];
                group_list_group::Data {
                    title,
                    recompute: data.recompute.is_some(),
                    previous_values_as_objective: data
                        .recompute
                        .as_ref()
                        .is_some_and(|r| r.previous_values_as_objective),
                }
            })
            .collect();
        self.objectify_cross_fixed_period = sanitized_config.objectify_cross_fixed_period;
        self.l1_anchor_weight = sanitized_config.l1_anchor_weight;
    }

    fn config_from_data(&self) -> SolveConfig {
        SolveConfig {
            periods: self
                .params
                .periods
                .period_ids()
                .zip(self.periods_data.iter())
                .map(|(id, data)| {
                    (
                        id,
                        PeriodSolveData {
                            recompute: data.recompute,
                            use_current_values: data.use_current_values,
                        },
                    )
                })
                .collect(),
            group_lists: self
                .params
                .group_lists
                .group_list_map
                .iter()
                .filter(|(_id, group_list)| !group_list.is_prefilled())
                .zip(self.group_lists_data.iter())
                .map(|((id, _), data)| {
                    (
                        id.clone(),
                        GroupListSolveData {
                            recompute: data.recompute.then(|| GroupListRecompute {
                                previous_values_as_objective: data.previous_values_as_objective,
                            }),
                        },
                    )
                })
                .collect(),
            objectify_cross_fixed_period: self.objectify_cross_fixed_period,
            l1_anchor_weight: self.l1_anchor_weight,
        }
    }

    /// Push the current `periods_data` into the left-hand factory list.
    fn refresh_periods_list(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.periods_list,
            self.periods_data.iter().cloned(),
            period_group::PeriodGroupInput::UpdateData,
        );
    }

    /// Push the current `group_lists_data` into the right-hand factory list.
    fn refresh_group_lists_list(&mut self) {
        crate::tools::factories::update_vec_deque(
            &mut self.group_lists_list,
            self.group_lists_data.iter().cloned(),
            group_list_group::GroupListGroupInput::UpdateData,
        );
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
            set_title: Some("Configuration de la résolution"),
            set_default_size: (1024, 576),
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
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    set_margin_all: 0,
                    set_spacing: 0,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Frame {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 5,
                        gtk::Paned {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_all: 0,
                            set_orientation: gtk::Orientation::Horizontal,
                            set_position: 510,
                            #[wrap(Some)]
                            set_start_child = &gtk::Box {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_margin_all: 0,
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 5,
                                gtk::Label {
                                    set_halign: gtk::Align::Center,
                                    set_margin_all: 10,
                                    set_label: "<b><big>Périodes</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.has_periods(),
                                },
                                gtk::ScrolledWindow {
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                                    #[watch]
                                    set_visible: model.has_periods(),
                                    #[local_ref]
                                    periods_box -> gtk::Box {
                                        set_hexpand: true,
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_all: 5,
                                        set_spacing: 10,
                                    },
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Center,
                                    set_vexpand: true,
                                    set_hexpand: true,
                                    set_justify: gtk::Justification::Center,
                                    set_label: "<b><big>Aucune période</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: !model.has_periods(),
                                },
                            },
                            #[wrap(Some)]
                            set_end_child = &gtk::Box {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_margin_all: 0,
                                set_orientation: gtk::Orientation::Vertical,
                                set_spacing: 5,
                                gtk::Label {
                                    set_halign: gtk::Align::Center,
                                    set_margin_all: 10,
                                    set_label: "<b><big>Listes automatiques</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.has_automatic_groups(),
                                },
                                gtk::ScrolledWindow {
                                    set_hexpand: true,
                                    set_vexpand: true,
                                    set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                                    #[watch]
                                    set_visible: model.has_automatic_groups(),
                                    #[local_ref]
                                    group_lists_box -> gtk::Box {
                                        set_hexpand: true,
                                        set_orientation: gtk::Orientation::Vertical,
                                        set_margin_all: 5,
                                        set_spacing: 10,
                                    },
                                },
                                gtk::Label {
                                    set_valign: gtk::Align::Center,
                                    set_vexpand: true,
                                    set_hexpand: true,
                                    set_justify: gtk::Justification::Center,
                                    set_label: "<b><big>Aucune liste automatique</big></b>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: !model.has_automatic_groups(),
                                },
                            },
                        },
                    },
                    gtk::Box {
                        set_margin_all: 0,
                        set_orientation: gtk::Orientation::Horizontal,
                        set_hexpand: true,
                        gtk::Frame {
                            set_hexpand: true,
                            set_margin_all: 5,
                            gtk::Box {
                                set_orientation: gtk::Orientation::Horizontal,
                                set_spacing: 5,
                                gtk::Label {
                                    set_margin_start: 10,
                                    set_margin_all: 5,
                                    set_label: "<b>Configuration du résolveur :</b>",
                                    set_use_markup: true,
                                },
                                gtk::Box {
                                    set_spacing: 0,
                                    add_css_class: "linked",
                                    #[name(opt_toggle_btn)]
                                    gtk::ToggleButton {
                                        set_margin_top: 5,
                                        set_margin_bottom: 5,
                                        set_label: "Optimisation complète",
                                        #[track(opt_toggle_btn.is_active() != model.is_opt_strategy())]
                                        set_active: model.is_opt_strategy(),
                                        connect_toggled[sender] => move |widget| {
                                            let new_state = widget.is_active();
                                            sender.input(if new_state {
                                                DialogInput::UpdateStrategy(ConductorStrategy::with_parallelism_defaults())
                                            } else {
                                                DialogInput::IgnoreOrRefresh
                                            });
                                        }
                                    },
                                    #[name(search_toggle_btn)]
                                    gtk::ToggleButton {
                                        set_margin_top: 5,
                                        set_margin_bottom: 5,
                                        set_label: "Recherche simple",
                                        #[track(search_toggle_btn.is_active() != model.is_search_strategy())]
                                        set_active: model.is_search_strategy(),
                                        connect_toggled[sender] => move |widget| {
                                            let new_state = widget.is_active();
                                            sender.input(if new_state {
                                                DialogInput::UpdateStrategy(ConductorStrategy::default())
                                            } else {
                                                DialogInput::IgnoreOrRefresh
                                            });
                                        }
                                    },
                                },
                                gtk::Label {
                                    set_margin_all: 5,
                                    set_label: "<i><small>Personnalisée</small></i>",
                                    set_use_markup: true,
                                    #[watch]
                                    set_visible: model.is_other_strategy(),
                                },
                                gtk::Box {
                                    set_hexpand: true,
                                },
                                gtk::Button {
                                    add_css_class: "frame",
                                    set_margin_all: 5,
                                    set_label: "Personnalisée",
                                    connect_clicked => DialogInput::OpenAdvanced,
                                },
                            },
                        },
                        gtk::Button {
                            add_css_class: "frame",
                            add_css_class: "warning",
                            set_margin_all: 5,
                            adw::ButtonContent {
                                set_icon_name: "configure-symbolic",
                                set_label: "Paramètres avancés",
                            },
                            connect_clicked => DialogInput::OpenAdvancedParams,
                        },
                    },
                },
            },
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let conductor_config_dialog = conductor_config::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                conductor_config::DialogOutput::Accepted(strategy) => {
                    DialogInput::UpdateStrategy(strategy)
                }
                conductor_config::DialogOutput::Cancelled => DialogInput::IgnoreOrRefresh,
            });

        let advanced_dialog = advanced_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                advanced_dialog::DialogOutput::Accepted(objectify, l1_anchor_weight) => {
                    DialogInput::UpdateAdvancedParams(objectify, l1_anchor_weight)
                }
                advanced_dialog::DialogOutput::Cancelled => DialogInput::IgnoreOrRefresh,
            });

        let periods_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                period_group::PeriodGroupOutput::RecomputeToggled(index, value) => {
                    DialogInput::SetPeriodRecompute(index, value)
                }
                period_group::PeriodGroupOutput::UseCurrentToggled(index, value) => {
                    DialogInput::SetPeriodUseCurrent(index, value)
                }
            });
        let group_lists_list = FactoryVecDeque::builder()
            .launch(gtk::Box::default())
            .forward(sender.input_sender(), |msg| match msg {
                group_list_group::GroupListGroupOutput::RecomputeToggled(index, value) => {
                    DialogInput::SetGroupListRecompute(index, value)
                }
                group_list_group::GroupListGroupOutput::ObjectiveToggled(index, value) => {
                    DialogInput::SetGroupListObjective(index, value)
                }
            });

        let model = Dialog {
            hidden: true,
            params: Parameters::default(),
            strategy: ConductorStrategy::with_parallelism_defaults(),
            conductor_config_dialog,
            advanced_dialog,
            periods_list,
            group_lists_list,
            periods_data: Vec::new(),
            group_lists_data: Vec::new(),
            objectify_cross_fixed_period: SolveConfig::default().objectify_cross_fixed_period,
            l1_anchor_weight: SolveConfig::default().l1_anchor_weight,
        };

        let periods_box = model.periods_list.widget();
        let group_lists_box = model.group_lists_list.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(config, strategy, params) => {
                self.hidden = false;
                self.params = params;
                self.strategy = strategy;
                self.set_data_from_config(config);

                self.refresh_periods_list();
                self.refresh_group_lists_list();
            }
            DialogInput::OpenAdvanced => {
                self.conductor_config_dialog
                    .sender()
                    .send(conductor_config::DialogInput::Show(self.strategy.clone()))
                    .unwrap();
            }
            DialogInput::UpdateStrategy(strategy) => {
                self.strategy = strategy;
            }
            DialogInput::IgnoreOrRefresh => {}
            DialogInput::SetPeriodRecompute(index, value) => {
                if let Some(data) = self.periods_data.get_mut(index) {
                    data.recompute = value;
                }
                self.refresh_periods_list();
            }
            DialogInput::SetPeriodUseCurrent(index, value) => {
                if let Some(data) = self.periods_data.get_mut(index) {
                    data.use_current_values = value;
                }
                self.refresh_periods_list();
            }
            DialogInput::SetGroupListRecompute(index, value) => {
                if let Some(data) = self.group_lists_data.get_mut(index) {
                    data.recompute = value;
                }
                self.refresh_group_lists_list();
            }
            DialogInput::SetGroupListObjective(index, value) => {
                if let Some(data) = self.group_lists_data.get_mut(index) {
                    data.previous_values_as_objective = value;
                }
                self.refresh_group_lists_list();
            }
            DialogInput::OpenAdvancedParams => {
                self.advanced_dialog
                    .sender()
                    .send(advanced_dialog::DialogInput::Show(
                        self.objectify_cross_fixed_period,
                        self.l1_anchor_weight,
                    ))
                    .unwrap();
            }
            DialogInput::UpdateAdvancedParams(objectify, l1_anchor_weight) => {
                self.objectify_cross_fixed_period = objectify;
                self.l1_anchor_weight = l1_anchor_weight;
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(
                        self.config_from_data(),
                        self.strategy.clone(),
                        self.params.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}
