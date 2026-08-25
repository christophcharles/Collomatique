use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    adw, gtk,
};

use collomatique_constraints_colloscopes::{ConfiguredColloscopeModel, SolveConfig};
use collomatique_state_colloscopes::colloscope_params::Parameters;
use collomatique_state_colloscopes::colloscopes::Colloscope;

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// Modal dialog shown while the ILP model is (re)built off-thread from a [`SolveConfig`]. It
/// streams the builder's log lines into a [`DebugView`] and, on success, hands the built model
/// back to the parent. While the build runs, "Annuler" abandons it; on failure the dialog
/// switches to an in-place error state, dismissed with "Fermer".
pub struct Dialog {
    hidden: bool,
    move_front: bool,
    /// `None` while building; `Some(message)` once the build has failed (error state).
    error: Option<String>,
    /// Discards log lines and build results from a superseded `Show` (or from after a cancel).
    build_seq: u64,
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    Show(SolveConfig, Parameters, Colloscope),
    /// One build-log line, streamed from the off-thread build that carries this sequence number.
    Echo(u64, String),
    Cancel,
    Close,
}

#[derive(Debug)]
pub enum DialogOutput {
    ModelReady(ConfiguredColloscopeModel),
    /// The build was abandoned through "Annuler": nothing will be handed back.
    Cancelled,
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    Built(u64, Result<ConfiguredColloscopeModel, String>),
}

/// Build the configured model from the current `params` and `colloscope`. `build_model` both
/// builds the base model and reads the current assignments back, so the full `InnerData` is
/// assembled here in the caller.
fn build_configured_model(
    config: &SolveConfig,
    params: Parameters,
    colloscope: Colloscope,
    log: &mut (dyn FnMut(&str) + Send),
) -> Result<ConfiguredColloscopeModel, String> {
    let inner_data = collomatique_state_colloscopes::InnerData {
        params,
        colloscope,
        ..Default::default()
    };
    config.build_model(&inner_data, log)
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCommandOutput;

    view! {
        #[root]
        root_window = gtk::Window {
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
                    set_visible: model.error.is_none(),
                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Annuler",
                        set_tooltip: "Abandonner la construction du modèle",
                        connect_clicked => DialogInput::Cancel,
                    },
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
            move_front: false,
            error: None,
            build_seq: 0,
            debug_view,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.move_front = false;
        match msg {
            DialogInput::Show(config, params, colloscope) => {
                self.hidden = false;
                self.move_front = true;
                self.error = None;
                // Any build still running from a previous opening is now stale.
                self.build_seq += 1;
                self.debug_view.emit(DebugViewInput::Clear);

                // Building the model is heavy work. Run it off the UI thread; each log line is
                // emitted back as `Echo` and streams live into the DebugView while the build
                // runs. `config`, `params` and `colloscope` are all consumed by the build; only
                // the built model is handed back. Both the log lines and the result carry `seq`,
                // so an abandoned build cannot write into a later one.
                let seq = self.build_seq;
                let input = sender.input_sender().clone();
                sender.spawn_oneshot_command(move || {
                    let mut log = move |line: &str| {
                        input.emit(DialogInput::Echo(seq, format!("{}\n", line)));
                    };
                    let result = build_configured_model(&config, params, colloscope, &mut log);
                    DialogCommandOutput::Built(seq, result)
                });
            }
            DialogInput::Echo(seq, line) => {
                if seq == self.build_seq {
                    self.debug_view.emit(DebugViewInput::Append(line));
                }
            }
            DialogInput::Cancel => {
                // Abandon the build in flight: bumping the sequence number makes its remaining
                // log lines and its eventual result no-ops. The worker thread itself cannot be
                // interrupted — it runs to completion off-screen and its model is dropped.
                self.build_seq += 1;
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                    sender.output(DialogOutput::Cancelled).unwrap();
                }
            }
            DialogInput::Close => {
                // Only dismissable once the build has failed; ignored while a build is in flight.
                if self.error.is_some() && !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
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
        self.move_front = false;
        let DialogCommandOutput::Built(seq, result) = msg;
        // A stale result: the build was cancelled, or superseded by a later `Show`, while it was
        // running. Drop it.
        if seq != self.build_seq {
            return;
        }
        match result {
            Ok(model) => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                sender.output(DialogOutput::ModelReady(model)).unwrap();
            }
            Err(e) => {
                self.error = Some(e);
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
