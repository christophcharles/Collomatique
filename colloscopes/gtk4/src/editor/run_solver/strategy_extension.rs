use gtk::prelude::{BoxExt, ButtonExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::gtk;
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};

use collomatique_strategies::ConductorStrategy;

use crate::editor::build_model::{ConfigExtension, ExtensionOutput};
use crate::editor::run_solver::conductor_config;

/// Solver-strategy slot of the model-configuration dialog: the two usual strategies as a linked
/// pair of toggles, plus a button opening the full [`conductor_config`] dialog for anything else.
///
/// [`conductor_config`]: crate::editor::run_solver::conductor_config
pub struct Extension {
    strategy: ConductorStrategy,
    /// The advanced solver-configuration dialog, opened via the "Personnalisée" button.
    conductor_config_dialog: Controller<conductor_config::Dialog>,
}

#[derive(Debug)]
pub enum ExtensionInput {
    /// Seed the widgets from the strategy the configuration dialog was shown with. Does not
    /// announce a change: the dialog already holds this value.
    SetStrategy(ConductorStrategy),
    /// The user picked another strategy, here or in the advanced dialog.
    UpdateStrategy(ConductorStrategy),
    OpenAdvanced,
    IgnoreOrRefresh,
    /// The advanced dialog just closed: the configuration window must come back to the front.
    Present,
}

impl Extension {
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

#[relm4::component(pub)]
impl SimpleComponent for Extension {
    type Init = gtk::Window;

    type Input = ExtensionInput;
    type Output = ExtensionOutput<ConductorStrategy>;

    view! {
        #[root]
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
                                ExtensionInput::UpdateStrategy(ConductorStrategy::with_parallelism_defaults())
                            } else {
                                ExtensionInput::IgnoreOrRefresh
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
                                ExtensionInput::UpdateStrategy(ConductorStrategy::default())
                            } else {
                                ExtensionInput::IgnoreOrRefresh
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
                    connect_clicked => ExtensionInput::OpenAdvanced,
                },
            },
        }
    }

    fn init(
        window: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let conductor_config_dialog = conductor_config::Dialog::builder()
            .transient_for(&window)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                conductor_config::DialogOutput::Accepted(strategy) => {
                    ExtensionInput::UpdateStrategy(strategy)
                }
                conductor_config::DialogOutput::Cancelled => ExtensionInput::IgnoreOrRefresh,
                conductor_config::DialogOutput::PresentParent => ExtensionInput::Present,
            });

        let model = Extension {
            strategy: ConductorStrategy::with_parallelism_defaults(),
            conductor_config_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            ExtensionInput::SetStrategy(strategy) => {
                self.strategy = strategy;
            }
            ExtensionInput::UpdateStrategy(strategy) => {
                self.strategy = strategy.clone();
                sender
                    .output(ExtensionOutput::ValueChanged(strategy))
                    .unwrap();
            }
            ExtensionInput::OpenAdvanced => {
                self.conductor_config_dialog
                    .sender()
                    .send(conductor_config::DialogInput::Show {
                        strategy: self.strategy.clone(),
                        // A colloscope solve starts from nothing: the model goes to the engine
                        // without a solution to begin with.
                        external_warm_start: false,
                    })
                    .unwrap();
            }
            ExtensionInput::IgnoreOrRefresh => {}
            ExtensionInput::Present => {
                sender.output(ExtensionOutput::Present).unwrap();
            }
        }
    }
}

impl ConfigExtension for Extension {
    type Value = ConductorStrategy;

    const WINDOW_TITLE: &'static str = "Configuration de la résolution";

    fn set_value_msg(value: ConductorStrategy) -> ExtensionInput {
        ExtensionInput::SetStrategy(value)
    }
}
