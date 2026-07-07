use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, ToggleButtonExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    SimpleComponent,
};
use relm4::{adw, gtk};

use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_strategies::ConductorStrategy;

use crate::editor::run_solver::conductor_config;

/// The "how to solve" half of a solve request, handed off when the user validates this dialog.
/// It is deliberately independent of the problem [`Parameters`] (the "what to solve" half), which
/// travels alongside it. For now it is just the conductor strategy; model-refinement fields
/// (problem scoping, ...) will be added here later, which is why it also owns a [`sanitize`] seam
/// and the `params`-taking [`build_model`].
///
/// [`sanitize`]: SolveConfig::sanitize
/// [`build_model`]: SolveConfig::build_model
#[derive(Debug, Clone)]
pub struct SolveConfig {
    pub strategy: ConductorStrategy,
    // future: problem-scoping / model-refinement fields
}

impl Default for SolveConfig {
    fn default() -> Self {
        // Parallel full-optimisation is the default solve strategy (NOT ConductorStrategy's own
        // Default, which is the simple-search strategy).
        SolveConfig {
            strategy: ConductorStrategy::with_parallelism_defaults(),
        }
    }
}

impl SolveConfig {
    /// Reconcile this config against the parameters it will be solved against, dropping or
    /// adjusting any refinements that no longer apply. Currently a no-op; kept as the seam for
    /// future model-refinement fields.
    pub fn sanitize(&mut self, _params: &Parameters) {}

    /// Build the ILP model to be solved from `params`, streaming build log lines through `log`.
    /// The build is a pure function of the parameters (the colloscope is discarded by the model
    /// builder), so the current assignments are intentionally left at their default here.
    pub async fn build_model(
        &self,
        params: &Parameters,
        log: &mut (dyn FnMut(&str) + Send),
    ) -> Result<collomatique_constraints_colloscopes::ColloscopeModel, String> {
        let inner_data = collomatique_state_colloscopes::InnerData {
            params: params.clone(),
            colloscope:
                collomatique_state_colloscopes::colloscopes::Colloscope::new_empty_from_params(
                    params,
                ),
            ..Default::default()
        };
        let pool = sqlx::SqlitePool::connect(":memory:")
            .await
            .map_err(|e| e.to_string())?;
        collomatique_sqlite_state::create_schema(&pool)
            .await
            .map_err(|e| e.to_string())?;
        collomatique_sqlite_state::inner_data_to_sqlite(&pool, &inner_data)
            .await
            .map_err(|e| e.to_string())?;
        Ok(collomatique_constraints_colloscopes::build_model_with_log(&pool, log).await)
    }
}

pub struct Dialog {
    hidden: bool,
    /// The parameters the assembled [`SolveConfig`] will build its model from, set on `Show`.
    params: Parameters,
    /// The problem/solver configuration this window is assembling. For now only the conductor
    /// strategy is tracked; problem-scoping widgets will be added here later.
    strategy: ConductorStrategy,
    /// The advanced solver-configuration dialog, opened via "Paramètres avancés du résolveur".
    conductor_config_dialog: Controller<conductor_config::Dialog>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(SolveConfig, Parameters),
    Cancel,
    Accept,
    OpenAdvanced,
    UpdateStrategy(ConductorStrategy),
    IgnoreOrRefresh,
}

#[derive(Debug)]
pub enum DialogOutput {
    Cancelled,
    Accepted(SolveConfig, Parameters),
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
                    set_margin_all: 0,
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
            params: Parameters::default(),
            strategy: ConductorStrategy::with_parallelism_defaults(),
            conductor_config_dialog,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>) {
        match msg {
            DialogInput::Show(mut config, params) => {
                config.sanitize(&params);
                self.hidden = false;
                self.params = params;
                self.strategy = config.strategy;
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
                    .output(DialogOutput::Accepted(
                        SolveConfig {
                            strategy: self.strategy.clone(),
                        },
                        self.params.clone(),
                    ))
                    .unwrap();
            }
        }
    }
}
