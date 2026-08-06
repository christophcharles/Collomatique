use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{ComponentParts, ComponentSender, FactorySender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use std::num::NonZeroU32;

use collomatique_strategies::{
    ConductorStrategy, ConductorWarning, DefaultConfig, FuzzyConfig, IncrementalConfig,
    WarmStartConfig,
};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,

    worker_count: u32,
    enable_warm_start: bool,
    warm_start_time_limit_enabled: bool,
    warm_start_time_limit_secs: u32,

    enable_default: bool,
    default_time_limit_enabled: bool,
    default_time_limit_secs: u32,
    default_incumbent_time_limit_enabled: bool,
    default_incumbent_time_limit_secs: u32,

    enable_incremental: bool,
    incremental_l1_weight: f64,
    incremental_tolerance: f64,
    incremental_time_limit_enabled: bool,
    incremental_time_limit_secs: u32,
    incremental_incumbent_time_limit_enabled: bool,
    incremental_incumbent_time_limit_secs: u32,

    enable_fuzzy: bool,
    fuzzy_sigma: f64,
    find_closest_tolerance: f64,
    fuzzy_time_limit_enabled: bool,
    fuzzy_time_limit_secs: u32,
    fuzzy_incumbent_time_limit_enabled: bool,
    fuzzy_incumbent_time_limit_secs: u32,

    /// The `ConductorStrategy` these widget states would produce, rebuilt after every update.
    strategy: ConductorStrategy,
    /// Live warnings for `strategy`, refreshed after every update.
    warnings: FactoryVecDeque<WarningItem>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(ConductorStrategy),
    Cancel,
    Accept,

    UpdateWorkerCount(u32),
    UpdateWarmStart(bool),
    UpdateWarmStartTimeLimitEnabled(bool),
    UpdateWarmStartTimeLimit(u32),
    UpdateIncremental(bool),
    UpdateIncrementalTolerance(f64),
    UpdateIncrementalL1Weight(f64),
    UpdateIncrementalTimeLimitEnabled(bool),
    UpdateIncrementalTimeLimit(u32),
    UpdateIncrementalIncumbentTimeLimitEnabled(bool),
    UpdateIncrementalIncumbentTimeLimit(u32),
    UpdateDefault(bool),
    UpdateDefaultTimeLimitEnabled(bool),
    UpdateDefaultTimeLimit(u32),
    UpdateDefaultIncumbentTimeLimitEnabled(bool),
    UpdateDefaultIncumbentTimeLimit(u32),
    UpdateFuzzyEnabled(bool),
    UpdateFuzzySigma(f64),
    UpdateTolerance(f64),
    UpdateFuzzyTimeLimitEnabled(bool),
    UpdateFuzzyTimeLimit(u32),
    UpdateFuzzyIncumbentTimeLimitEnabled(bool),
    UpdateFuzzyIncumbentTimeLimit(u32),
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    Accepted(ConductorStrategy),
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
            set_title: Some("Configuration du solveur"),
            set_default_size: (500, 500),
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
                    set_margin_all: 5,
                    set_spacing: 10,
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
                            gtk::Box {
                                set_hexpand: true,
                                set_margin_all: 0,
                                set_spacing: 0,
                                set_orientation: gtk::Orientation::Vertical,
                                adw::PreferencesGroup {
                                    set_title: "Parallélisme",
                                    set_margin_all: 5,
                                    set_hexpand: true,
                                    adw::SpinRow {
                                        set_hexpand: true,
                                        set_title: "Tâches en parallèle",
                                        #[wrap(Some)]
                                        set_adjustment = &gtk::Adjustment {
                                            set_lower: 1.,
                                            set_upper: u32::MAX as f64,
                                            set_step_increment: 1.,
                                            set_page_increment: 4.,
                                        },
                                        set_wrap: false,
                                        set_snap_to_ticks: true,
                                        set_numeric: true,
                                        #[track(self.should_redraw)]
                                        set_value: model.worker_count as f64,
                                        connect_value_notify[sender] => move |widget| {
                                            let value = widget.value() as u32;
                                            sender.input(DialogInput::UpdateWorkerCount(value));
                                        },
                                    },
                                },
                                gtk::Label {
                                    set_label: &Self::worker_count_recommendation(),
                                    add_css_class: "dim-label",
                                    set_wrap: true,
                                    set_xalign: 0.0,
                                    set_margin_start: 12,
                                    set_margin_end: 12,
                                    set_margin_top: 6,
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Stratégies",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Démarrage à chaud",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_warm_start,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateWarmStart(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Stratégie par défaut",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_default,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateDefault(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Résolution incrémentale",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_incremental,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateIncremental(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Exploration aléatoire",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_fuzzy,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateFuzzyEnabled(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Démarrage à chaud",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.enable_warm_start,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps (phase de recherche)",
                                    #[track(self.should_redraw)]
                                    set_active: model.warm_start_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateWarmStartTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.warm_start_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.warm_start_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateWarmStartTimeLimit(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Stratégie par défaut",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.enable_default,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps",
                                    #[track(self.should_redraw)]
                                    set_active: model.default_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateDefaultTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.default_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.default_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateDefaultTimeLimit(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps après la première solution",
                                    #[track(self.should_redraw)]
                                    set_active: model.default_incumbent_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateDefaultIncumbentTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.default_incumbent_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.default_incumbent_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateDefaultIncumbentTimeLimit(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Résolution incrémentale",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.enable_incremental,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Tolérance de recherche",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.incremental_tolerance,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateIncrementalTolerance(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Poids L1",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 100.,
                                        set_page_increment: 500.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.incremental_l1_weight,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateIncrementalL1Weight(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps (par époque)",
                                    #[track(self.should_redraw)]
                                    set_active: model.incremental_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateIncrementalTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.incremental_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.incremental_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateIncrementalTimeLimit(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps après la première solution (par époque)",
                                    #[track(self.should_redraw)]
                                    set_active: model.incremental_incumbent_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateIncrementalIncumbentTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.incremental_incumbent_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.incremental_incumbent_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateIncrementalIncumbentTimeLimit(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Exploration aléatoire",
                                set_margin_all: 5,
                                set_hexpand: true,
                                #[watch]
                                set_visible: model.enable_fuzzy,
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Sigma",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 0.1,
                                        set_page_increment: 1.,
                                    },
                                    set_digits: 2,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.fuzzy_sigma,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateFuzzySigma(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Tolérance de recherche",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 0.,
                                        set_upper: f64::MAX,
                                        set_step_increment: 1.,
                                        set_page_increment: 5.,
                                    },
                                    set_digits: 1,
                                    set_wrap: false,
                                    set_numeric: true,
                                    #[track(self.should_redraw)]
                                    set_value: model.find_closest_tolerance,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateTolerance(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps (phase de recherche)",
                                    #[track(self.should_redraw)]
                                    set_active: model.fuzzy_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateFuzzyTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.fuzzy_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.fuzzy_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateFuzzyTimeLimit(value));
                                    },
                                },
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Limite de temps après la première solution",
                                    #[track(self.should_redraw)]
                                    set_active: model.fuzzy_incumbent_time_limit_enabled,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateFuzzyIncumbentTimeLimitEnabled(value));
                                    },
                                },
                                adw::SpinRow {
                                    set_hexpand: true,
                                    set_title: "Durée (s)",
                                    #[wrap(Some)]
                                    set_adjustment = &gtk::Adjustment {
                                        set_lower: 1.,
                                        set_upper: u32::MAX as f64,
                                        set_step_increment: 10.,
                                        set_page_increment: 60.,
                                    },
                                    set_digits: 0,
                                    set_wrap: false,
                                    set_snap_to_ticks: true,
                                    set_numeric: true,
                                    #[watch]
                                    set_visible: model.fuzzy_incumbent_time_limit_enabled,
                                    #[track(self.should_redraw)]
                                    set_value: model.fuzzy_incumbent_time_limit_secs as f64,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value() as u32;
                                        sender.input(DialogInput::UpdateFuzzyIncumbentTimeLimit(value));
                                    },
                                },
                            },
                        },
                    },
                    gtk::Box {
                            set_orientation: gtk::Orientation::Vertical,
                            set_hexpand: true,
                            set_spacing: 5,
                            set_margin_all: 5,
                            #[watch]
                            set_visible: model.has_warnings(),
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "Avertissements",
                                set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold").unwrap()),
                            },
                            gtk::ScrolledWindow {
                                set_propagate_natural_height: true,
                                set_vexpand: false,
                                set_hscrollbar_policy: gtk::PolicyType::Never,
                                set_vscrollbar_policy: gtk::PolicyType::Automatic,
                                #[local_ref]
                                warnings_listbox -> gtk::ListBox {
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
        let strategy = ConductorStrategy::default();
        let fuzzy_defaults = FuzzyConfig::default();
        let incremental_defaults = IncrementalConfig::default();
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            worker_count: strategy.worker_count.get(),
            enable_warm_start: strategy.warm_start_config.is_some(),
            warm_start_time_limit_enabled: strategy
                .warm_start_config
                .as_ref()
                .is_some_and(|cfg| cfg.time_limit.is_some()),
            warm_start_time_limit_secs: Self::DEFAULT_TIME_LIMIT_SECS,
            enable_default: strategy.default_config.is_some(),
            default_time_limit_enabled: strategy
                .default_config
                .as_ref()
                .is_some_and(|cfg| cfg.time_limit.is_some()),
            default_time_limit_secs: Self::DEFAULT_TIME_LIMIT_SECS,
            default_incumbent_time_limit_enabled: strategy
                .default_config
                .as_ref()
                .is_some_and(|cfg| cfg.incumbent_time_limit.is_some()),
            default_incumbent_time_limit_secs: Self::DEFAULT_INCUMBENT_TIME_LIMIT_SECS,
            enable_incremental: strategy.incremental_config.is_some(),
            incremental_l1_weight: incremental_defaults.l1_weight,
            incremental_tolerance: incremental_defaults.distance_tolerance,
            incremental_time_limit_enabled: strategy
                .incremental_config
                .as_ref()
                .is_some_and(|cfg| cfg.epoch_time_limit.is_some()),
            incremental_time_limit_secs: Self::DEFAULT_TIME_LIMIT_SECS,
            incremental_incumbent_time_limit_enabled: strategy
                .incremental_config
                .as_ref()
                .is_some_and(|cfg| cfg.epoch_incumbent_time_limit.is_some()),
            incremental_incumbent_time_limit_secs: Self::DEFAULT_INCUMBENT_TIME_LIMIT_SECS,
            enable_fuzzy: strategy.fuzzy_config.is_some(),
            fuzzy_sigma: fuzzy_defaults.fuzzy_sigma,
            find_closest_tolerance: fuzzy_defaults.find_closest_tolerance,
            fuzzy_time_limit_enabled: strategy
                .fuzzy_config
                .as_ref()
                .is_some_and(|cfg| cfg.time_limit.is_some()),
            fuzzy_time_limit_secs: Self::DEFAULT_TIME_LIMIT_SECS,
            fuzzy_incumbent_time_limit_enabled: strategy
                .fuzzy_config
                .as_ref()
                .is_some_and(|cfg| cfg.incumbent_time_limit.is_some()),
            fuzzy_incumbent_time_limit_secs: Self::DEFAULT_INCUMBENT_TIME_LIMIT_SECS,
            strategy,
            warnings: FactoryVecDeque::builder()
                .launch(gtk::ListBox::default())
                .detach(),
        };

        let warnings_listbox = model.warnings.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        match msg {
            DialogInput::Show(strategy) => {
                self.hidden = false;
                self.should_redraw = true;
                self.update_state_from_strategy(strategy);
            }
            DialogInput::Cancel => {
                self.hidden = true;
                sender.output(DialogOutput::Cancelled).unwrap();
            }
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::Accepted(self.strategy.clone()))
                    .unwrap();
            }
            DialogInput::UpdateWorkerCount(value) => {
                if self.worker_count == value {
                    return;
                }
                self.worker_count = value;
            }
            DialogInput::UpdateWarmStart(value) => {
                if self.enable_warm_start == value {
                    return;
                }
                self.enable_warm_start = value;
            }
            DialogInput::UpdateWarmStartTimeLimitEnabled(value) => {
                if self.warm_start_time_limit_enabled == value {
                    return;
                }
                self.warm_start_time_limit_enabled = value;
            }
            DialogInput::UpdateWarmStartTimeLimit(value) => {
                if self.warm_start_time_limit_secs == value {
                    return;
                }
                self.warm_start_time_limit_secs = value;
            }
            DialogInput::UpdateIncremental(value) => {
                if self.enable_incremental == value {
                    return;
                }
                self.enable_incremental = value;
            }
            DialogInput::UpdateIncrementalTolerance(value) => {
                if self.incremental_tolerance == value {
                    return;
                }
                self.incremental_tolerance = value;
            }
            DialogInput::UpdateIncrementalL1Weight(value) => {
                if self.incremental_l1_weight == value {
                    return;
                }
                self.incremental_l1_weight = value;
            }
            DialogInput::UpdateIncrementalTimeLimitEnabled(value) => {
                if self.incremental_time_limit_enabled == value {
                    return;
                }
                self.incremental_time_limit_enabled = value;
            }
            DialogInput::UpdateIncrementalTimeLimit(value) => {
                if self.incremental_time_limit_secs == value {
                    return;
                }
                self.incremental_time_limit_secs = value;
            }
            DialogInput::UpdateIncrementalIncumbentTimeLimitEnabled(value) => {
                if self.incremental_incumbent_time_limit_enabled == value {
                    return;
                }
                self.incremental_incumbent_time_limit_enabled = value;
            }
            DialogInput::UpdateIncrementalIncumbentTimeLimit(value) => {
                if self.incremental_incumbent_time_limit_secs == value {
                    return;
                }
                self.incremental_incumbent_time_limit_secs = value;
            }
            DialogInput::UpdateDefault(value) => {
                if self.enable_default == value {
                    return;
                }
                self.enable_default = value;
            }
            DialogInput::UpdateDefaultTimeLimitEnabled(value) => {
                if self.default_time_limit_enabled == value {
                    return;
                }
                self.default_time_limit_enabled = value;
            }
            DialogInput::UpdateDefaultTimeLimit(value) => {
                if self.default_time_limit_secs == value {
                    return;
                }
                self.default_time_limit_secs = value;
            }
            DialogInput::UpdateDefaultIncumbentTimeLimitEnabled(value) => {
                if self.default_incumbent_time_limit_enabled == value {
                    return;
                }
                self.default_incumbent_time_limit_enabled = value;
            }
            DialogInput::UpdateDefaultIncumbentTimeLimit(value) => {
                if self.default_incumbent_time_limit_secs == value {
                    return;
                }
                self.default_incumbent_time_limit_secs = value;
            }
            DialogInput::UpdateFuzzyEnabled(value) => {
                if self.enable_fuzzy == value {
                    return;
                }
                self.enable_fuzzy = value;
            }
            DialogInput::UpdateFuzzySigma(value) => {
                if self.fuzzy_sigma == value {
                    return;
                }
                self.fuzzy_sigma = value;
            }
            DialogInput::UpdateTolerance(value) => {
                if self.find_closest_tolerance == value {
                    return;
                }
                self.find_closest_tolerance = value;
            }
            DialogInput::UpdateFuzzyTimeLimitEnabled(value) => {
                if self.fuzzy_time_limit_enabled == value {
                    return;
                }
                self.fuzzy_time_limit_enabled = value;
            }
            DialogInput::UpdateFuzzyTimeLimit(value) => {
                if self.fuzzy_time_limit_secs == value {
                    return;
                }
                self.fuzzy_time_limit_secs = value;
            }
            DialogInput::UpdateFuzzyIncumbentTimeLimitEnabled(value) => {
                if self.fuzzy_incumbent_time_limit_enabled == value {
                    return;
                }
                self.fuzzy_incumbent_time_limit_enabled = value;
            }
            DialogInput::UpdateFuzzyIncumbentTimeLimit(value) => {
                if self.fuzzy_incumbent_time_limit_secs == value {
                    return;
                }
                self.fuzzy_incumbent_time_limit_secs = value;
            }
        }
        self.strategy = self.build_strategy();
        self.update_warnings();
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.should_redraw {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(0.);
        }
    }
}

impl Dialog {
    /// Seconds seeded into a time-limit spin the first time its switch is turned on (10 minutes).
    const DEFAULT_TIME_LIMIT_SECS: u32 = 600;
    /// Seconds seeded into an after-incumbent time-limit spin the first time its switch is turned
    /// on (5 minutes).
    const DEFAULT_INCUMBENT_TIME_LIMIT_SECS: u32 = 300;

    fn update_state_from_strategy(&mut self, strategy: ConductorStrategy) {
        self.worker_count = strategy.worker_count.get();
        match &strategy.warm_start_config {
            Some(cfg) => {
                self.enable_warm_start = true;
                (
                    self.warm_start_time_limit_enabled,
                    self.warm_start_time_limit_secs,
                ) = read_time_limit(cfg.time_limit, self.warm_start_time_limit_secs);
            }
            // Keep the last time limit so re-enabling warm-start shows the previous value.
            None => {
                self.enable_warm_start = false;
            }
        }
        match &strategy.default_config {
            Some(cfg) => {
                self.enable_default = true;
                (
                    self.default_time_limit_enabled,
                    self.default_time_limit_secs,
                ) = read_time_limit(cfg.time_limit, self.default_time_limit_secs);
                (
                    self.default_incumbent_time_limit_enabled,
                    self.default_incumbent_time_limit_secs,
                ) = read_time_limit(
                    cfg.incumbent_time_limit,
                    self.default_incumbent_time_limit_secs,
                );
            }
            // Keep the last time limit so re-enabling default shows the previous value.
            None => {
                self.enable_default = false;
            }
        }
        match &strategy.incremental_config {
            Some(cfg) => {
                self.enable_incremental = true;
                self.incremental_l1_weight = cfg.l1_weight;
                self.incremental_tolerance = cfg.distance_tolerance;
                (
                    self.incremental_time_limit_enabled,
                    self.incremental_time_limit_secs,
                ) = read_time_limit(cfg.epoch_time_limit, self.incremental_time_limit_secs);
                (
                    self.incremental_incumbent_time_limit_enabled,
                    self.incremental_incumbent_time_limit_secs,
                ) = read_time_limit(
                    cfg.epoch_incumbent_time_limit,
                    self.incremental_incumbent_time_limit_secs,
                );
            }
            // Keep the last weight/tolerance/limit so re-enabling incremental shows the previous values.
            None => {
                self.enable_incremental = false;
            }
        }
        match &strategy.fuzzy_config {
            Some(cfg) => {
                self.enable_fuzzy = true;
                self.fuzzy_sigma = cfg.fuzzy_sigma;
                self.find_closest_tolerance = cfg.find_closest_tolerance;
                (self.fuzzy_time_limit_enabled, self.fuzzy_time_limit_secs) =
                    read_time_limit(cfg.time_limit, self.fuzzy_time_limit_secs);
                (
                    self.fuzzy_incumbent_time_limit_enabled,
                    self.fuzzy_incumbent_time_limit_secs,
                ) = read_time_limit(
                    cfg.incumbent_time_limit,
                    self.fuzzy_incumbent_time_limit_secs,
                );
            }
            // Keep the last sigma/tolerance/limit so re-enabling fuzzy shows the previous values.
            None => {
                self.enable_fuzzy = false;
            }
        }
    }

    fn build_strategy(&self) -> ConductorStrategy {
        ConductorStrategy {
            worker_count: NonZeroU32::new(self.worker_count).unwrap_or(NonZeroU32::MIN),
            default_config: self.enable_default.then(|| DefaultConfig {
                time_limit: make_time_limit(
                    self.default_time_limit_enabled,
                    self.default_time_limit_secs,
                ),
                incumbent_time_limit: make_time_limit(
                    self.default_incumbent_time_limit_enabled,
                    self.default_incumbent_time_limit_secs,
                ),
            }),
            warm_start_config: self.enable_warm_start.then(|| WarmStartConfig {
                time_limit: make_time_limit(
                    self.warm_start_time_limit_enabled,
                    self.warm_start_time_limit_secs,
                ),
            }),
            incremental_config: self.enable_incremental.then(|| IncrementalConfig {
                l1_weight: self.incremental_l1_weight,
                distance_tolerance: self.incremental_tolerance,
                epoch_time_limit: make_time_limit(
                    self.incremental_time_limit_enabled,
                    self.incremental_time_limit_secs,
                ),
                epoch_incumbent_time_limit: make_time_limit(
                    self.incremental_incumbent_time_limit_enabled,
                    self.incremental_incumbent_time_limit_secs,
                ),
            }),
            fuzzy_config: self.enable_fuzzy.then(|| FuzzyConfig {
                fuzzy_sigma: self.fuzzy_sigma,
                find_closest_tolerance: self.find_closest_tolerance,
                time_limit: make_time_limit(
                    self.fuzzy_time_limit_enabled,
                    self.fuzzy_time_limit_secs,
                ),
                incumbent_time_limit: make_time_limit(
                    self.fuzzy_incumbent_time_limit_enabled,
                    self.fuzzy_incumbent_time_limit_secs,
                ),
            }),
        }
    }

    fn update_warnings(&mut self) {
        let mut guard = self.warnings.guard();
        guard.clear();
        for warning in self.strategy.warnings() {
            guard.push_back(warning_message(warning).to_string());
        }
    }

    fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    fn worker_count_recommendation() -> String {
        format!(
            "Nombre de tâches recommandé pour cet ordinateur : {}",
            ConductorStrategy::with_parallelism_defaults()
                .worker_count
                .get()
        )
    }
}

/// Reads a backend [`collomatique_time::TimeLimit`] into the dialog's `(enabled, secs)` pair.
/// When the limit is
/// unbounded, keeps `current` so a later re-enable restores the last shown value.
fn read_time_limit(tl: collomatique_time::TimeLimit, current: u32) -> (bool, u32) {
    match tl.get_seconds() {
        Some(s) => (true, s.get()),
        None => (false, current),
    }
}

/// Builds a backend [`collomatique_time::TimeLimit`] from the dialog's `(enabled, secs)` pair.
/// `0 s` (or disabled)
/// maps to no limit.
fn make_time_limit(enabled: bool, secs: u32) -> collomatique_time::TimeLimit {
    if enabled {
        NonZeroU32::new(secs)
            .map(collomatique_time::TimeLimit::seconds)
            .unwrap_or_default()
    } else {
        collomatique_time::TimeLimit::none()
    }
}

fn warning_message(warning: ConductorWarning) -> &'static str {
    match warning {
        ConductorWarning::NoStrategyEnabled => {
            "Aucune stratégie n'est activée : rien ne sera exécuté."
        }
        ConductorWarning::NoOptimizing => {
            "Aucune stratégie d'optimisation n'est activée : le solveur cherchera une solution \
             réalisable sans tenter de l'améliorer."
        }
        ConductorWarning::NoSeed => {
            "L'exploration aléatoire est activée mais aucune stratégie ne produit de solution \
             initiale (démarrage à chaud ou résolution incrémentale) : elle ne démarrera jamais et \
             le solveur s'arrêtera immédiatement."
        }
        ConductorWarning::StarvedFuzzy => {
            "L'exploration aléatoire est activée mais l'unique tâche est occupée par la stratégie \
             par défaut : elle n'aura jamais de créneau libre. Augmentez le nombre de tâches en \
             parallèle."
        }
        ConductorWarning::WontFinish => {
            "La stratégie par défaut est désactivée : sans elle, aucune borne ne prouve \
             l'optimalité et le solveur tournera indéfiniment."
        }
        ConductorWarning::ColdFuzzy => {
            "L'exploration aléatoire est activée sans solution initiale (démarrage à chaud ou \
             résolution incrémentale) : elle ne se déclenchera qu'une fois la stratégie par défaut \
             bien avancée et sera donc souvent inutile."
        }
        ConductorWarning::RedundantWarmStart => {
            "Le démarrage à chaud et la résolution incrémentale sont tous deux activés : la \
             résolution incrémentale fournit généralement un meilleur point de départ ; le \
             démarrage à chaud n'est utile que pour obtenir rapidement une solution."
        }
        ConductorWarning::OverwhelmedCpu => {
            "Le nombre de tâches en parallèle dépasse le nombre de cœurs du processeur."
        }
    }
}

#[derive(Debug)]
struct WarningItem {
    message: String,
}

#[relm4::factory]
impl FactoryComponent for WarningItem {
    type Init = String;
    type Input = ();
    type Output = ();
    type CommandOutput = ();
    type ParentWidget = gtk::ListBox;

    view! {
        #[root]
        gtk::Box {
            set_margin_all: 5,
            set_orientation: gtk::Orientation::Horizontal,
            add_css_class: "warning",
            gtk::Image {
                set_margin_end: 5,
                set_icon_name: Some("dialog-warning-symbolic"),
            },
            gtk::Label {
                set_halign: gtk::Align::Start,
                set_wrap: true,
                set_label: &self.message,
            },
        },
    }

    fn init_model(
        message: Self::Init,
        _index: &DynamicIndex,
        _sender: FactorySender<Self>,
    ) -> Self {
        Self { message }
    }

    fn update(&mut self, _msg: Self::Input, _sender: FactorySender<Self>) {}
}
