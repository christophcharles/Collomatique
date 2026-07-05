use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
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
    ConductorConfigAccepted(ConductorStrategy),
    ConductorConfigCancelled,
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
            set_title: Some("Configuration du colloscope"),
            set_default_size: (800, 600),
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
                    gtk::Button {
                        add_css_class: "frame",
                        add_css_class: "warning",
                        set_margin_all: 5,
                        set_halign: gtk::Align::End,
                        adw::ButtonContent {
                            set_icon_name: "configure-symbolic",
                            set_label: "Paramètres avancés du résolveur",
                        },
                        connect_clicked => DialogInput::OpenAdvanced,
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
                    DialogInput::ConductorConfigAccepted(strategy)
                }
                conductor_config::DialogOutput::Cancelled => DialogInput::ConductorConfigCancelled,
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
            DialogInput::ConductorConfigAccepted(strategy) => {
                self.strategy = strategy;
            }
            DialogInput::ConductorConfigCancelled => {}
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
