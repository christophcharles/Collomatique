use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::prelude::{DynamicIndex, FactoryComponent};
use relm4::{ComponentParts, ComponentSender, FactorySender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use std::num::NonZeroU32;

use collomatique_strategies::{
    ConductorStrategy, ConductorWarning, FuzzyConfig, IncrementalConfig,
};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,

    worker_count: u32,
    enable_warm_start: bool,
    enable_default: bool,

    enable_incremental: bool,
    incremental_l1_weight: f64,
    incremental_tolerance: f64,

    enable_fuzzy: bool,
    fuzzy_sigma: f64,
    find_closest_tolerance: f64,

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
    UpdateIncremental(bool),
    UpdateIncrementalTolerance(f64),
    UpdateIncrementalL1Weight(f64),
    UpdateDefault(bool),
    UpdateFuzzyEnabled(bool),
    UpdateFuzzySigma(f64),
    UpdateTolerance(f64),
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
                            },
                            adw::PreferencesGroup {
                                set_title: "Résolution incrémentale",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer la résolution incrémentale",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_incremental,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateIncremental(value));
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
                                    #[watch]
                                    set_visible: model.enable_incremental,
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
                                    #[watch]
                                    set_visible: model.enable_incremental,
                                    #[track(self.should_redraw)]
                                    set_value: model.incremental_l1_weight,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateIncrementalL1Weight(value));
                                    },
                                },
                            },
                            adw::PreferencesGroup {
                                set_title: "Exploration aléatoire",
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_use_markup: false,
                                    set_title: "Activer l'exploration aléatoire",
                                    #[track(self.should_redraw)]
                                    set_active: model.enable_fuzzy,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::UpdateFuzzyEnabled(value));
                                    },
                                },
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
                                    #[watch]
                                    set_visible: model.enable_fuzzy,
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
                                    #[watch]
                                    set_visible: model.enable_fuzzy,
                                    #[track(self.should_redraw)]
                                    set_value: model.find_closest_tolerance,
                                    connect_value_notify[sender] => move |widget| {
                                        let value = widget.value();
                                        sender.input(DialogInput::UpdateTolerance(value));
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
            enable_warm_start: strategy.enable_warm_start,
            enable_default: strategy.enable_default,
            enable_incremental: strategy.incremental_config.is_some(),
            incremental_l1_weight: incremental_defaults.l1_weight,
            incremental_tolerance: incremental_defaults.distance_tolerance,
            enable_fuzzy: strategy.fuzzy_config.is_some(),
            fuzzy_sigma: fuzzy_defaults.fuzzy_sigma,
            find_closest_tolerance: fuzzy_defaults.find_closest_tolerance,
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
            DialogInput::UpdateDefault(value) => {
                if self.enable_default == value {
                    return;
                }
                self.enable_default = value;
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
    fn update_state_from_strategy(&mut self, strategy: ConductorStrategy) {
        self.worker_count = strategy.worker_count.get();
        self.enable_default = strategy.enable_default;
        self.enable_warm_start = strategy.enable_warm_start;
        match strategy.incremental_config {
            Some(cfg) => {
                self.enable_incremental = true;
                self.incremental_l1_weight = cfg.l1_weight;
                self.incremental_tolerance = cfg.distance_tolerance;
            }
            // Keep the last weight/tolerance so re-enabling incremental shows the previous values.
            None => {
                self.enable_incremental = false;
            }
        }
        match strategy.fuzzy_config {
            Some(cfg) => {
                self.enable_fuzzy = true;
                self.fuzzy_sigma = cfg.fuzzy_sigma;
                self.find_closest_tolerance = cfg.find_closest_tolerance;
            }
            // Keep the last sigma/tolerance so re-enabling fuzzy shows the previous values.
            None => {
                self.enable_fuzzy = false;
            }
        }
    }

    fn build_strategy(&self) -> ConductorStrategy {
        ConductorStrategy {
            worker_count: NonZeroU32::new(self.worker_count).unwrap_or(NonZeroU32::MIN),
            enable_default: self.enable_default,
            enable_warm_start: self.enable_warm_start,
            incremental_config: self.enable_incremental.then(|| IncrementalConfig {
                l1_weight: self.incremental_l1_weight,
                distance_tolerance: self.incremental_tolerance,
                epoch_time_limit: collomatique_time::TimeLimit::none(),
            }),
            fuzzy_config: self.enable_fuzzy.then(|| FuzzyConfig {
                fuzzy_sigma: self.fuzzy_sigma,
                find_closest_tolerance: self.find_closest_tolerance,
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
