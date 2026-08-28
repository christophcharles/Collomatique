use collomatique_rpc::{CmdMsg, ResultMsg};
use collomatique_rpc_colloscopes::{
    AppAnswerMsg, AppCmdMsg, AppInitMsg, ColloInitMsg, ColloProtocol, ColloResultMsg,
    InternalDataStream,
};
use gtk::prelude::{
    BoxExt, ButtonExt, EditableExt, EntryBufferExt, EntryBufferExtManual, EntryExt, GtkWindowExt,
    OrientableExt, WidgetExt,
};
use relm4::{Component, ComponentController, ComponentParts, ComponentSender, Controller};
use relm4::{RelmWidgetExt, adw, gtk};

use collomatique_state_colloscopes::Data;
use collomatique_subprocesses::{EngineExe, SendError, Worker, WorkerEvent};

use crate::widgets::debug_view::{DebugView, DebugViewInput};

/// The interactive Python console, and the worker that runs it
///
/// Not modal: the document stays editable while the console is open, which is
/// why what the console sends back is checked against what the application
/// holds at that moment.
pub struct Window {
    hidden: bool,
    move_front: bool,
    focus_entry: bool,
    debug_view: Controller<DebugView>,
    worker: Option<Worker<ColloProtocol>>,
    /// Which session the worker belongs to
    ///
    /// Restarting or closing bumps it. Events and answers carry the generation
    /// they belong to, so what a dead worker asked for cannot be answered to
    /// the fresh one — the channel is strict request/response and one stray
    /// answer would desynchronize it for good.
    generation: u64,
    /// The prompt the worker is waiting on a line for
    pending_read: Option<String>,
    entry: String,
    /// Text to put in the entry, applied once the widgets are reachable
    entry_op: Option<String>,
    history: Vec<String>,
    /// Where Up/Down are in the history; `history.len()` is the line being typed.
    history_cursor: usize,
}

#[derive(Debug)]
pub enum WindowInput {
    Show,
    CloseRequest,
    Restart,

    UpdateEntry(String),
    Submit,
    HistoryPrev,
    HistoryNext,

    Worker {
        generation: u64,
        event: WorkerEvent<ColloProtocol>,
    },
    DataAnswer {
        generation: u64,
        data: Data,
    },
    ReplaceAnswer {
        generation: u64,
        outcome: ReplaceOutcome,
    },
}

/// What became of a document the console handed over
#[derive(Debug)]
pub enum ReplaceOutcome {
    Done { token: u64 },
    Refused,
    Failed(String),
}

#[derive(Debug)]
pub enum WindowOutput {
    DataRequest {
        generation: u64,
    },
    ReplaceRequest {
        generation: u64,
        data: InternalDataStream,
        token: Option<u64>,
    },
    /// This window just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[relm4::component(pub)]
impl Component for Window {
    type Init = ();

    type Input = WindowInput;
    type Output = WindowOutput;
    type CommandOutput = ();

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: false,
            set_default_size: (750, 500),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Console Python"),
            connect_close_request[sender] => move |_| {
                sender.input(WindowInput::CloseRequest);
                gtk::glib::Propagation::Stop
            },

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Redémarrer",
                        connect_clicked => WindowInput::Restart,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Fermer",
                        connect_clicked => WindowInput::CloseRequest,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,

                    append = model.debug_view.widget(),

                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_margin_all: 5,
                        set_spacing: 5,

                        gtk::Label {
                            add_css_class: "monospace",
                            #[watch]
                            set_label: model.pending_read.as_deref().unwrap_or(""),
                        },

                        #[name(entry)]
                        gtk::Entry {
                            set_hexpand: true,
                            add_css_class: "monospace",
                            // Only while the console is asking for a line: at
                            // any other moment there is nobody to read it.
                            #[watch]
                            set_sensitive: model.pending_read.is_some(),
                            set_buffer = &gtk::EntryBuffer {
                                connect_text_notify[sender] => move |widget| {
                                    let text: String = widget.text().into();
                                    sender.input(WindowInput::UpdateEntry(text));
                                },
                            },
                            connect_activate => WindowInput::Submit,
                            add_controller = gtk::EventControllerKey {
                                connect_key_pressed[sender] => move |_, key, _, _| {
                                    match key {
                                        gtk::gdk::Key::Up => {
                                            sender.input(WindowInput::HistoryPrev);
                                            gtk::glib::Propagation::Stop
                                        }
                                        gtk::gdk::Key::Down => {
                                            sender.input(WindowInput::HistoryNext);
                                            gtk::glib::Propagation::Stop
                                        }
                                        _ => gtk::glib::Propagation::Proceed,
                                    }
                                }
                            },
                        },
                    },
                }
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        // A long-lived window: the transcript is capped rather than kept whole.
        let debug_view = DebugView::builder().launch(Some(5000)).detach();

        let model = Window {
            hidden: true,
            move_front: false,
            focus_entry: false,
            debug_view,
            worker: None,
            generation: 0,
            pending_read: None,
            entry: String::new(),
            entry_op: None,
            history: Vec::new(),
            history_cursor: 0,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.move_front = false;
        self.focus_entry = false;
        match msg {
            WindowInput::Show => {
                self.hidden = false;
                self.move_front = true;
                self.focus_entry = true;
                if self.worker.is_none() {
                    self.debug_view.emit(DebugViewInput::Clear);
                    self.start_session(&sender);
                }
            }
            WindowInput::CloseRequest => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(WindowOutput::PresentParent).unwrap();
                }
                self.end_session();
            }
            WindowInput::Restart => {
                self.end_session();
                self.append("\n─── Session redémarrée ───\n");
                self.start_session(&sender);
            }
            WindowInput::UpdateEntry(text) => {
                self.entry = text;
            }
            WindowInput::Submit => {
                let Some(prompt) = self.pending_read.take() else {
                    return;
                };
                let line = std::mem::take(&mut self.entry);
                self.append(format!("{prompt}{line}\n"));
                if !line.trim().is_empty() && self.history.last() != Some(&line) {
                    self.history.push(line.clone());
                }
                self.history_cursor = self.history.len();
                self.entry_op = Some(String::new());
                self.send_response(ResultMsg::App(AppAnswerMsg::Line(line)));
            }
            WindowInput::HistoryPrev => {
                if self.history_cursor > 0 {
                    self.history_cursor -= 1;
                    self.entry_op = Some(self.history[self.history_cursor].clone());
                }
            }
            WindowInput::HistoryNext => {
                if self.history_cursor < self.history.len() {
                    self.history_cursor += 1;
                    self.entry_op = Some(
                        self.history
                            .get(self.history_cursor)
                            .cloned()
                            .unwrap_or_default(),
                    );
                }
            }
            WindowInput::Worker { generation, event } => {
                if generation != self.generation {
                    return;
                }
                self.worker_event(event, &sender);
            }
            WindowInput::DataAnswer { generation, data } => {
                if generation != self.generation {
                    return;
                }
                self.send_response(ResultMsg::App(AppAnswerMsg::generate_data_msg(&data)));
            }
            WindowInput::ReplaceAnswer {
                generation,
                outcome,
            } => {
                if generation != self.generation {
                    return;
                }
                self.send_response(match outcome {
                    ReplaceOutcome::Done { token } => {
                        ResultMsg::App(AppAnswerMsg::ReplaceDone { token })
                    }
                    ReplaceOutcome::Refused => ResultMsg::App(AppAnswerMsg::ReplaceRefused),
                    ReplaceOutcome::Failed(error) => ResultMsg::GlobalError(error),
                });
            }
        }
    }

    fn update_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::Input,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update(message, sender.clone(), root);
        self.update_view(widgets, sender);
        if let Some(text) = self.entry_op.take() {
            widgets.entry.set_text(&text);
            widgets.entry.set_position(-1);
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
        if self.focus_entry {
            widgets.entry.grab_focus();
        }
    }
}

impl Window {
    /// Spawns a worker for a new session
    fn start_session(&mut self, sender: &ComponentSender<Self>) {
        self.generation += 1;
        self.pending_read = None;

        let generation = self.generation;
        let input = sender.input_sender().clone();
        let callback = move |event: WorkerEvent<ColloProtocol>| {
            input.emit(WindowInput::Worker { generation, event });
        };

        match Worker::spawn(
            &EngineExe::Current,
            ColloInitMsg::App(AppInitMsg::StartPythonRepl),
            callback,
        ) {
            Ok(worker) => self.worker = Some(worker),
            Err(e) => self.append(format!("{e}\n")),
        }
    }

    /// Ends the session, if there is one: dropping the worker kills the
    /// subprocess, and the bumped generation drops what it still had in flight.
    fn end_session(&mut self) {
        self.worker = None;
        self.pending_read = None;
        self.generation += 1;
    }

    fn worker_event(&mut self, event: WorkerEvent<ColloProtocol>, sender: &ComponentSender<Self>) {
        match event {
            WorkerEvent::LogLine(line) => self.append(line),
            WorkerEvent::RpcCommand(Ok(cmd)) => match cmd {
                CmdMsg::App(AppCmdMsg::ReadLine { prompt }) => {
                    self.pending_read = Some(prompt);
                    self.focus_entry = true;
                }
                CmdMsg::App(AppCmdMsg::GetData) => {
                    sender
                        .output(WindowOutput::DataRequest {
                            generation: self.generation,
                        })
                        .unwrap();
                }
                CmdMsg::App(AppCmdMsg::ReplaceData { data, token }) => {
                    sender
                        .output(WindowOutput::ReplaceRequest {
                            generation: self.generation,
                            data,
                            token,
                        })
                        .unwrap();
                }
                CmdMsg::App(AppCmdMsg::SetData(_)) => {
                    self.send_response(ResultMsg::GlobalError(String::from(
                        "cette commande n'est disponible que pour un script Python",
                    )));
                }
                CmdMsg::Solver(_) | CmdMsg::Strategy(_) => {}
            },
            WorkerEvent::RpcCommand(Err(e)) => {
                if !e.payload().is_empty() {
                    self.append(format!("{e}\n"));
                }
                self.send_response(ResultMsg::InvalidMsg);
            }
            // Both arrive for one ending, so the line is written for whichever
            // comes first.
            WorkerEvent::GracefulExit | WorkerEvent::ProcessExited(_) => {
                if self.worker.is_some() {
                    self.worker = None;
                    self.pending_read = None;
                    self.append(
                        "\n─── Session terminée : cliquez sur « Redémarrer » pour en ouvrir \
                         une autre. ───\n",
                    );
                }
            }
            WorkerEvent::Error(e) => {
                self.worker = None;
                self.pending_read = None;
                self.append(format!("{e}\n"));
            }
        }
    }

    fn append(&self, text: impl Into<String>) {
        self.debug_view.emit(DebugViewInput::Append(text.into()));
    }

    fn send_response(&self, msg: ColloResultMsg) {
        if let Some(worker) = self.worker.as_ref() {
            match worker.send_rpc_message(msg) {
                // The worker already exited: nothing to respond to (also
                // reported as its own event). Harmless, so ignore it.
                Ok(()) | Err(SendError::Finished) => {}
                Err(SendError::Io(e)) => {
                    self.append(format!("Erreur d'envoi de la réponse : {e}\n"));
                }
            }
        }
    }
}
