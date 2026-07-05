use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};
use relm4::{adw, gtk};

use collomatique_strategies::ConductorStrategy;

use crate::editor::run_solver::conductor_config;

pub struct Dialog {
    hidden: bool,
    /// The problem/solver configuration this window is assembling. For now only the conductor
    /// strategy is tracked; problem-scoping widgets will be added here later.
    strategy: ConductorStrategy,
    /// The advanced solver-configuration dialog, opened via "Paramètres avancés du résolveur".
    conductor_config_dialog: Controller<conductor_config::Dialog>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show,
    Cancel,
    Accept,
    OpenAdvanced,
    UpdateStrategy(ConductorStrategy),
    IgnoreOrRefresh,
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    Accepted(ConductorStrategy),
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
            set_title: Some("Configuration du colloscope"),
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
                    set_margin_all: 5,
                    set_spacing: 0,
                    set_orientation: gtk::Orientation::Vertical,
                    gtk::Box {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 0,
                        gtk::Frame {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_all: 5,
                        },
                        gtk::Frame {
                            set_hexpand: true,
                            set_vexpand: true,
                            set_margin_all: 5,
                        },
                    },
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
                                add_css_class: "warning",
                                set_margin_all: 5,
                                adw::ButtonContent {
                                    set_icon_name: "configure-symbolic",
                                    set_label: "Paramètres avancés",
                                },
                                connect_clicked => DialogInput::OpenAdvanced,
                            },
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

        let model = Dialog {
            hidden: true,
            strategy: ConductorStrategy::with_parallelism_defaults(),
            conductor_config_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show => {
                self.hidden = false;
                self.strategy = ConductorStrategy::with_parallelism_defaults();
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
        }
    }
}
