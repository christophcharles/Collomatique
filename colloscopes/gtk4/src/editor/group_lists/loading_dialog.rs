use gtk::prelude::{BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::{
    Component, ComponentController, ComponentParts, ComponentSender, Controller, RelmWidgetExt,
    adw, gtk,
};

use collomatique_constraints_groups::{
    FrozenPlacements, GenerationPlan, GroupListsModel, ObjectiveWeights, build_model_with_log,
};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// Modal dialog shown while the group-list ILP model is built off-thread, on the way to the
/// optional polish of a greedy result. Cousin of `build_model::loading_dialog`, with no error
/// state: the model build cannot fail.
pub struct Dialog {
    hidden: bool,
    move_front: bool,
    /// Discards log lines and build results from a superseded `Show` (or from after a cancel).
    build_seq: u64,
    debug_view: Controller<DebugView>,
}

#[derive(Debug)]
pub enum DialogInput {
    /// Build the model for this plan, holding these seats fixed. An empty set pins nothing,
    /// which is what leaves the polish free to redo the whole assignment.
    Show(GenerationPlan, FrozenPlacements),
    /// One build-log line, streamed from the off-thread build that carries this sequence number.
    Echo(u64, String),
    Cancel,
}

#[derive(Debug)]
pub enum DialogOutput {
    /// The built model. The plan does not travel back: it came from the page, which still
    /// holds the copy the solution will be converted against.
    ModelReady(GroupListsModel),
    /// The build was abandoned through "Annuler": nothing will be handed back.
    Cancelled,
    /// The dialog just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[derive(Debug)]
pub enum DialogCommandOutput {
    Built(u64, GroupListsModel),
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
            DialogInput::Show(plan, frozen) => {
                self.hidden = false;
                self.move_front = true;
                // Any build still running from a previous opening is now stale.
                self.build_seq += 1;
                self.debug_view.emit(DebugViewInput::Clear);

                // The plan is the naming dialog's own — the one the greedy ran on, and the one
                // the frozen seats were read off — so seats and specs cannot drift apart. The
                // build is the heavy step and runs off the UI thread: each log line comes back
                // as `Echo` and streams live into the DebugView, and both the lines and the
                // result carry `seq`, so an abandoned build cannot write into a later one.
                let seq = self.build_seq;
                let input = sender.input_sender().clone();
                sender.spawn_oneshot_command(move || {
                    let mut log = move |line: &str| {
                        input.emit(DialogInput::Echo(seq, format!("{}\n", line)));
                    };
                    let model =
                        build_model_with_log(&plan, ObjectiveWeights::default(), &frozen, &mut log);
                    DialogCommandOutput::Built(seq, model)
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
        let DialogCommandOutput::Built(seq, model) = msg;
        // A stale result: the build was cancelled, or superseded by a later `Show`, while it was
        // running. Drop it.
        if seq != self.build_seq {
            return;
        }
        if !self.hidden {
            self.hidden = true;
            sender.output(DialogOutput::PresentParent).unwrap();
        }
        sender.output(DialogOutput::ModelReady(model)).unwrap();
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
    }
}
