use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    adw, gtk,
};

use collomatique_constraints_groups::{
    FrozenPlacements, GenerationPlan, GenerationRequest, GroupListsModel, ObjectiveWeights,
    build_generation_plan, build_model_with_log,
};
use collomatique_state_colloscopes::colloscope_params::Parameters;

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// Modal dialog shown while the group-list ILP model is built off-thread, on the way to the
/// optional polish of a greedy result. Cousin of `build_model::loading_dialog`, with no error
/// state: neither the plan nor the model can fail here (see `Show`).
pub struct Dialog {
    hidden: bool,
    move_front: bool,
    /// Discards log lines and build results from a superseded `Show` (or from after a cancel).
    build_seq: u64,
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    /// Build the plan and the model for this request — canonical range included — against
    /// these parameters, with these objective weights, holding these seats fixed. An empty
    /// set pins nothing, which is what leaves the polish free to redo the whole assignment.
    Show(
        GenerationRequest,
        ObjectiveWeights,
        FrozenPlacements,
        Parameters,
    ),
    /// One build-log line, streamed from the off-thread build that carries this sequence number.
    Echo(u64, String),
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    /// The plan the model was built from, and the model. The plan travels along because the
    /// solve's result is converted back against exactly it.
    ModelReady(GenerationPlan, GroupListsModel),
    /// The build was abandoned through "Annuler": nothing will be handed back.
    Cancelled,
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    Built(u64, GenerationPlan, GroupListsModel),
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
            // The window has no close button (hidden titlebar); a window-manager close request
            // abandons the build, exactly like "Annuler".
            connect_close_request[sender] => move |_| {
                sender.input(DialogInput::Cancel);
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
                    set_hexpand: true,
                    set_vexpand: true,
                    append: model.debug_view.widget(),
                },

                gtk::Box {
                    set_halign: gtk::Align::Center,
                    gtk::Button {
                        set_size_request: (200, 40),
                        set_label: "Annuler",
                        set_tooltip: "Abandonner la construction du modèle",
                        connect_clicked => DialogInput::Cancel,
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
        let debug_view = DebugView::builder().launch(None).detach();

        let model = Dialog {
            hidden: true,
            move_front: false,
            build_seq: 0,
            debug_view,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.move_front = false;
        match msg {
            DialogInput::Show(request, weights, frozen, params) => {
                self.hidden = false;
                self.move_front = true;
                // Any build still running from a previous opening is now stale.
                self.build_seq += 1;
                self.debug_view.emit(DebugViewInput::Clear);

                // The plan is rebuilt rather than carried over, because the optimize window may
                // have set a canonical range the naming dialog's plan did not have. Rebuilding
                // keeps spec identity and order (the range is elected or overridden *after* the
                // specs are assembled), so the names stay aligned with the specs.
                //
                // Both steps run off the UI thread: the model build is the heavy one, and the
                // plan build does the template clustering. Each log line comes back as `Echo`
                // and streams live into the DebugView; both the lines and the result carry
                // `seq`, so an abandoned build cannot write into a later one.
                let seq = self.build_seq;
                let input = sender.input_sender().clone();
                sender.spawn_oneshot_command(move || {
                    let mut log = move |line: &str| {
                        input.emit(DialogInput::Echo(seq, format!("{}\n", line)));
                    };
                    // The naming dialog already built this very plan from this very request
                    // against these parameters, and no plan error depends on the canonical
                    // range: a failure here is a caller bug, like it is there.
                    let plan = build_generation_plan(&params, &request)
                        .expect("the naming dialog already built a plan from this request");
                    // The seats were read off the plan the naming dialog built, and this one
                    // is rebuilt from the same request and parameters — the very assumption the
                    // `expect` above states. `build_model_with_log` asserts on a seat this plan
                    // does not have, so a drift here is loud rather than silent.
                    let model = build_model_with_log(&plan, weights, &frozen, &mut log);
                    DialogCommandOutput::Built(seq, plan, model)
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
        }
    }

    fn update_cmd(
        &mut self,
        msg: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.move_front = false;
        let DialogCommandOutput::Built(seq, plan, model) = msg;
        // A stale result: the build was cancelled, or superseded by a later `Show`, while it was
        // running. Drop it.
        if seq != self.build_seq {
            return;
        }
        if !self.hidden {
            self.hidden = true;
            sender.output(DialogOutput::PresentParent).unwrap();
        }
        sender
            .output(DialogOutput::ModelReady(plan, model))
            .unwrap();
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
