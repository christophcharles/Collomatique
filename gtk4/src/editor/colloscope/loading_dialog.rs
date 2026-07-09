use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    adw, gtk,
};

use collomatique_constraints_colloscopes::{ColloscopeModel, ProblemInternalVar, SolveConfig};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;
use collomatique_strategies::{ConductorPayload, IncrementalPayload};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// Modal, button-less dialog shown while the ILP model is (re)built off-thread from a
/// [`SolveConfig`]. It streams the builder's log lines into a [`DebugView`] and, on success,
/// hands the built model back to the parent. On failure it switches to an in-place error state
/// (the only state with a dismiss button).
pub struct Dialog {
    hidden: bool,
    /// `None` while building; `Some(message)` once the build has failed (error state).
    error: Option<String>,
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(SolveConfig, Parameters, Colloscope),
    Echo(String),
    Close,
}

#[derive(Debug)]
pub enum DialogOutput {
    ModelReady(ColloscopeModel, ConductorPayload<ProblemInternalVar>),
}

/// Build the incremental epoch payload from the freshly-built model: every `StudentGroup` base
/// variable is solved first (epoch 0), then each `GroupInInterrogation` variable is solved in the
/// epoch matching its week (week + 1), so the schedule fills in week by week on top of the fixed
/// group assignment. Base variables absent from the map fall into the strategy's final epoch.
fn build_incremental_payload(model: &ColloscopeModel) -> ConductorPayload<ProblemInternalVar> {
    let epochs = collomatique_constraints_colloscopes::build_incremental_epochs(model);
    ConductorPayload {
        incremental: IncrementalPayload { epochs },
    }
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    Built(Result<ColloscopeModel, String>),
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCommandOutput;

    view! {
        #[root]
        gtk::Window {
            set_modal: true,
            set_default_size: (600, 450),
            #[watch]
            set_visible: !model.hidden,
            // The window has no close button (hidden titlebar); a window-manager close request is
            // routed to `Close`, which only acts once the build has failed.
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Close);
                gtk::glib::Propagation::Stop
            },
            #[wrap(Some)]
            set_titlebar = &gtk::Box {
                set_visible: false,
            },

            gtk::Box {
                set_orientation: gtk::Orientation::Vertical,
                set_spacing: 10,
                set_margin_all: 15,

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_spacing: 5,
                    #[watch]
                    set_visible: model.error.is_none(),
                    adw::Spinner {
                        set_size_request: (64, 64),
                    },
                    gtk::Label {
                        set_margin_top: 15,
                        set_label: "Construction du modèle...",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    },
                },

                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_halign: gtk::Align::Center,
                    set_spacing: 5,
                    #[watch]
                    set_visible: model.error.is_some(),
                    gtk::Image::from_icon_name("dialog-error-symbolic") {
                        set_size_request: (64, 64),
                        set_pixel_size: 64,
                    },
                    gtk::Label {
                        set_margin_top: 15,
                        set_label: "Échec de la construction du modèle",
                        set_attributes: Some(&gtk::pango::AttrList::from_string("weight bold, scale 1.2").unwrap()),
                    },
                    gtk::Label {
                        set_wrap: true,
                        set_justify: gtk::Justification::Center,
                        #[watch]
                        set_label: model.error.as_deref().unwrap_or(""),
                    },
                },

                gtk::Box {
                    set_hexpand: true,
                    set_vexpand: true,
                    append: model.debug_view.widget(),
                },

                gtk::Box {
                    set_halign: gtk::Align::Center,
                    #[watch]
                    set_visible: model.error.is_some(),
                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Fermer",
                        connect_clicked => DialogInput::Close,
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let debug_view = DebugView::builder().launch(()).detach();

        let model = Dialog {
            hidden: true,
            error: None,
            debug_view,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match msg {
            DialogInput::Show(config, params, colloscope) => {
                self.hidden = false;
                self.error = None;
                self.debug_view.emit(DebugViewInput::Clear);

                // Building the model is heavy, async (in-memory sqlite) work. Run it off the UI
                // thread; each log line is emitted back as `Echo` and streams live into the
                // DebugView while the build runs. `config`, `params` and `colloscope` are all
                // consumed by the build; only the built model is handed back.
                let input = sender.input_sender().clone();
                sender.oneshot_command(async move {
                    let mut log = move |line: &str| {
                        input.emit(DialogInput::Echo(format!("{}\n", line)));
                    };
                    let result = config.build_model(&params, &colloscope, &mut log).await;
                    DialogCommandOutput::Built(result)
                });
            }
            DialogInput::Echo(line) => {
                self.debug_view.emit(DebugViewInput::Append(line));
            }
            DialogInput::Close => {
                // Only dismissable once the build has failed; ignored while a build is in flight.
                if self.error.is_some() {
                    self.hidden = true;
                }
            }
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        let DialogCommandOutput::Built(result) = msg;
        match result {
            Ok(model) => {
                self.hidden = true;
                let payload = build_incremental_payload(&model);
                sender
                    .output(DialogOutput::ModelReady(model, payload))
                    .unwrap();
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }
}
