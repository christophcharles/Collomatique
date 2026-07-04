use adw::prelude::{PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt, SimpleComponent};
use relm4::{adw, gtk};

use std::num::NonZeroU32;

use collomatique_strategies::{ConductorStrategy, FuzzyConfig};

pub struct Dialog {
    hidden: bool,
    should_redraw: bool,

    worker_count: u32,
    enable_warm_start: bool,
    enable_default: bool,

    enable_fuzzy: bool,
    fuzzy_sigma: f64,
    find_closest_tolerance: f64,

    /// The `ConductorStrategy` these widget states would produce, rebuilt after every update.
    strategy: ConductorStrategy,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(ConductorStrategy),
    Cancel,
    Accept,

    UpdateWorkerCount(u32),
    UpdateWarmStart(bool),
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
        let model = Dialog {
            hidden: true,
            should_redraw: false,
            worker_count: strategy.worker_count.get(),
            enable_warm_start: strategy.enable_warm_start,
            enable_default: strategy.enable_default,
            enable_fuzzy: strategy.fuzzy_config.is_some(),
            fuzzy_sigma: fuzzy_defaults.fuzzy_sigma,
            find_closest_tolerance: fuzzy_defaults.find_closest_tolerance,
            strategy,
        };

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
            fuzzy_config: self.enable_fuzzy.then(|| FuzzyConfig {
                fuzzy_sigma: self.fuzzy_sigma,
                find_closest_tolerance: self.find_closest_tolerance,
            }),
        }
    }
}
