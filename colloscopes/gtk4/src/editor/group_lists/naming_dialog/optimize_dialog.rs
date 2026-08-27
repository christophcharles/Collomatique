use adw::prelude::{ActionRowExt, PreferencesGroupExt, PreferencesRowExt};
use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};
use relm4::{adw, gtk};

use collomatique_strategies::ConductorStrategy;

use crate::editor::run_solver::conductor_config;

/// The door to the ILP polish: what the optional optimization run needs that the greedy did not
/// — whether the prefill is held fixed, and the solver configuration. The objective itself is
/// not a choice: the model maximizes the very score the greedy maximized, so the solve is a
/// refinement of the result already on screen and has nothing to weigh against it. Validating
/// starts the solve from that result; cancelling leaves the naming dialog with it untouched.
pub struct Dialog {
    hidden: bool,
    move_front: bool,
    should_redraw: bool,
    /// Whether the ILP keeps the greedy's prefill instead of re-deciding it.
    fix_prefill: bool,
    /// The solver configuration this window carries, seeded on `Show` and edited through the
    /// child `conductor_config` dialog.
    strategy: ConductorStrategy,
    /// That child dialog, opened by the frame's edit button.
    conductor_config_dialog: Controller<conductor_config::Dialog>,
}

impl Dialog {
    /// Whether the carried configuration is still the preset this flow defaults to. Drives the
    /// frame's status label — the configuration itself is never rendered in detail here.
    fn is_default_strategy(&self) -> bool {
        self.strategy == ConductorStrategy::with_parallelism_optimize_only()
    }

    fn strategy_status(&self) -> &'static str {
        if self.is_default_strategy() {
            "par défaut"
        } else {
            "personnalisée"
        }
    }
}

#[derive(Debug)]
pub enum DialogInput {
    /// Open with the page's current solver configuration and prefill choice.
    Show(ConductorStrategy, bool),
    Cancel,
    Accept,
    SetFixPrefill(bool),
    /// The frame's edit button: open the solver-configuration dialog.
    OpenSolverConfig,
    UpdateStrategy(ConductorStrategy),
    /// The frame's reset button: back to the preset of this flow.
    ResetStrategy,
    /// The child dialog closed without a new configuration: nothing to change, but this window
    /// goes back to the front.
    IgnoreOrRefresh,
    /// The child dialog just closed: bring this window back to the front.
    Present,
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    /// The solver configuration and whether the prefill is to be held fixed.
    Accepted(ConductorStrategy, bool),
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[relm4::component(pub)]
impl SimpleComponent for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Optimiser les listes de groupes"),
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
                        set_label: "Optimiser",
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
                                set_title: "Pré-remplissage",
                                set_description: Some("La première phase du calcul remplit des groupes entiers avec des élèves qui suivent exactement les mêmes matières"),
                                set_margin_all: 5,
                                set_hexpand: true,
                                adw::SwitchRow {
                                    set_hexpand: true,
                                    set_title: "Figer le pré-remplissage",
                                    set_subtitle: "Ces élèves gardent leur groupe et l'optimisation ne déplace que les autres. Le calcul est bien plus rapide, mais les meilleures solutions peuvent devenir inatteignables",
                                    #[track(model.should_redraw)]
                                    set_active: model.fix_prefill,
                                    connect_active_notify[sender] => move |widget| {
                                        let value = widget.is_active();
                                        sender.input(DialogInput::SetFixPrefill(value));
                                    },
                                },
                            },
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
                            gtk::Label {
                                set_margin_all: 5,
                                #[watch]
                                set_label: model.strategy_status(),
                            },
                            gtk::Box {
                                set_hexpand: true,
                            },
                            gtk::Button {
                                add_css_class: "flat",
                                set_margin_all: 5,
                                set_icon_name: "document-edit-symbolic",
                                set_tooltip: "Paramètres du résolveur personnalisés",
                                connect_clicked => DialogInput::OpenSolverConfig,
                            },
                            gtk::Button {
                                add_css_class: "flat",
                                set_margin_all: 5,
                                set_icon_name: "view-refresh-symbolic",
                                set_tooltip: "Réinitialiser la configuration du résolveur par défaut",
                                connect_clicked => DialogInput::ResetStrategy,
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
        let conductor_config_dialog = conductor_config::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                conductor_config::DialogOutput::Accepted(strategy) => {
                    DialogInput::UpdateStrategy(strategy)
                }
                conductor_config::DialogOutput::Cancelled => DialogInput::IgnoreOrRefresh,
                conductor_config::DialogOutput::PresentParent => DialogInput::Present,
            });

        let model = Dialog {
            hidden: true,
            move_front: false,
            should_redraw: false,
            fix_prefill: false,
            strategy: ConductorStrategy::with_parallelism_optimize_only(),
            conductor_config_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        self.should_redraw = false;
        self.move_front = false;
        match msg {
            DialogInput::Show(strategy, fix_prefill) => {
                self.hidden = false;
                self.move_front = true;
                self.should_redraw = true;
                self.strategy = strategy;
                self.fix_prefill = fix_prefill;
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Cancelled).unwrap();
                }
            }
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender
                        .output(DialogOutput::Accepted(
                            self.strategy.clone(),
                            self.fix_prefill,
                        ))
                        .unwrap();
                }
            }
            DialogInput::SetFixPrefill(value) => {
                if self.fix_prefill == value {
                    return;
                }
                self.fix_prefill = value;
            }
            DialogInput::OpenSolverConfig => {
                self.conductor_config_dialog
                    .sender()
                    .send(conductor_config::DialogInput::Show {
                        strategy: self.strategy.clone(),
                        // This flow always hands the solve the greedy result as warm start, so
                        // the seeding warnings and the "Solution initiale fournie" row are read
                        // against a supplied solution.
                        external_warm_start: true,
                    })
                    .unwrap();
            }
            DialogInput::UpdateStrategy(strategy) => {
                self.strategy = strategy;
            }
            DialogInput::ResetStrategy => {
                self.strategy = ConductorStrategy::with_parallelism_optimize_only();
            }
            DialogInput::IgnoreOrRefresh => {}
            DialogInput::Present => {
                self.move_front = true;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
