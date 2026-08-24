use collomatique_rpc::{CmdMsg, ResultMsg};
use collomatique_state::traits::Manager;
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{Component, ComponentController, adw, gtk};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use collomatique_ops::Desc;
use collomatique_state::{AppSession, AppState};
use collomatique_state_colloscopes::Data;

use crate::widgets::debug_view::{DebugView, DebugViewInput};
use collomatique_subprocesses::{EngineExe, SendError, Worker, WorkerEvent};
use std::path::PathBuf;

mod error_dialog;
mod error_display;
mod warning_running;

pub struct Dialog {
    hidden: bool,
    move_front: bool,
    path: PathBuf,
    script: String,
    end_with_error: bool,
    debug_view: Controller<DebugView>,
    worker: Option<Worker>,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    errors: FactoryVecDeque<error_display::Entry>,
    adjust_scrolling: bool,
    app_session: Option<AppSession<AppState<Data, Desc>, Desc>>,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(PathBuf, String, AppState<Data, Desc>),
    CancelRequest,
    Accept,

    Cancel,
    Echo(String),
    ProcessFinished,
    Cmd(Result<collomatique_rpc::CmdMsg, collomatique_rpc::RpcDecodeError>),
    Error(String),
    /// One of this window's own dialogs just closed: bring this window back to
    /// the front.
    Present,
}

#[derive(Debug)]
pub enum DialogCmdOutput {
    AdjustScrolling,
}

#[derive(Debug)]
pub enum DialogOutput {
    NewData(AppState<Data, Desc>),
    /// This window just closed: whoever owns the window underneath should bring
    /// it back to the front, because Windows will not do it on its own.
    PresentParent,
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCmdOutput;

    view! {
        #[root]
        root_window = adw::Window {
            set_modal: true,
            set_default_size: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            set_title: Some("Exécution du script Python"),
            add_css_class: "devel",

            adw::ToolbarView {
                add_top_bar = &adw::HeaderBar {
                    set_show_start_title_buttons: false,
                    set_show_end_title_buttons: false,
                    pack_start = &gtk::Button {
                        set_label: "Annuler",
                        set_sensitive: true,
                        connect_clicked => DialogInput::CancelRequest,
                    },
                    pack_end = &gtk::Button {
                        set_label: "Valider les modifications",
                        #[watch]
                        set_sensitive: model.worker.is_none() && model.has_modifications(),
                        add_css_class: "destructive-action",
                        connect_clicked => DialogInput::Accept,
                    },
                },
                #[wrap(Some)]
                set_content = &gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,
                    set_hexpand: true,
                    set_vexpand: true,
                    adw::Spinner {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_size_request: (50, 50),
                        #[watch]
                        set_visible: model.worker.is_some(),
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: model.worker.is_none() && !model.end_with_error && model.has_modifications(),
                        gtk::Image::from_icon_name("object-select-symbolic") {
                            set_size_request: (50, 50),
                            set_pixel_size: 50,
                        },
                        gtk::Label {
                            set_label: "Exécution terminée",
                        },
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: model.worker.is_none() && !model.end_with_error && !model.has_modifications(),
                        gtk::Image::from_icon_name("dialog-warning-symbolic") {
                            set_size_request: (50, 50),
                            set_pixel_size: 50,
                        },
                        gtk::Label {
                            set_label: "Aucune modification effectuée",
                        },
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: model.worker.is_none() && model.end_with_error,
                        gtk::Image::from_icon_name("dialog-error-symbolic") {
                            set_size_request: (50, 50),
                            set_pixel_size: 50,
                        },
                        gtk::Label {
                            set_label: "Erreur pendant l'exécution",
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        set_halign: gtk::Align::Start,
                        set_label: "Erreurs de communications avec le sous-processus :",
                        #[watch]
                        set_visible: !model.errors.is_empty(),
                    },
                    #[name(scrolled_window)]
                    gtk::ScrolledWindow {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                        set_margin_all: 5,
                        #[watch]
                        set_visible: !model.errors.is_empty(),
                        #[local_ref]
                        errors_listbox -> gtk::ListBox {
                            set_hexpand: true,
                            add_css_class: "boxed-list",
                            set_selection_mode: gtk::SelectionMode::None,
                        }
                    },
                    gtk::Box {
                        set_margin_all: 5,
                        set_hexpand: true,
                        set_vexpand: true,
                        set_orientation: gtk::Orientation::Vertical,
                        gtk::Label {
                            set_halign: gtk::Align::Start,
                            set_label: "Informations de débogage :",
                        },
                        append = model.debug_view.widget(),
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        add_css_class: "dimmed",
                        #[watch]
                        set_label: &model.path.to_string_lossy(),
                    },
                }
            }
        }
    }

    fn init(
        _: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                error_dialog::DialogOutput::PresentParent => DialogInput::Present,
            });

        let warning_running = warning_running::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_running::DialogOutput::Accept => DialogInput::Cancel,
                warning_running::DialogOutput::PresentParent => DialogInput::Present,
            });

        let debug_view = DebugView::builder().launch(()).detach();

        let errors = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let model = Dialog {
            hidden: true,
            move_front: false,
            path: PathBuf::new(),
            script: String::new(),
            end_with_error: false,
            debug_view,
            worker: None,
            error_dialog,
            warning_running,
            errors,
            adjust_scrolling: false,
            app_session: None,
        };

        let errors_listbox = model.errors.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.adjust_scrolling = false;
        self.move_front = false;
        match msg {
            DialogInput::Run(path, script, app_state) => {
                self.hidden = false;
                self.move_front = true;
                self.path = path;
                self.script = script;
                self.app_session = Some(AppSession::new(app_state));
                self.errors.guard().clear();
                self.end_with_error = false;
                self.debug_view.emit(DebugViewInput::Clear);

                let input = sender.input_sender().clone();
                let callback = move |event: WorkerEvent| match event {
                    WorkerEvent::LogLine(line) => {
                        input.emit(DialogInput::Echo(line));
                    }
                    WorkerEvent::RpcCommand(cmd) => {
                        input.emit(DialogInput::Cmd(cmd));
                    }
                    WorkerEvent::GracefulExit | WorkerEvent::ProcessExited(_) => {
                        input.emit(DialogInput::ProcessFinished);
                    }
                    WorkerEvent::Error(e) => {
                        input.emit(DialogInput::Error(e.to_string()));
                    }
                };

                let spawn_result = Worker::spawn(
                    &EngineExe::Current,
                    collomatique_rpc::InitMsg::RunPythonScript(self.script.clone()),
                    callback,
                );

                match spawn_result {
                    Ok(worker) => self.worker = Some(worker),
                    Err(e) => {
                        self.end_with_error = true;
                        self.error_dialog
                            .sender()
                            .send(error_dialog::DialogInput::Show(e.to_string()))
                            .unwrap();
                    }
                }
            }
            DialogInput::CancelRequest => {
                if self.worker.is_some() {
                    self.warning_running
                        .sender()
                        .send(warning_running::DialogInput::Show)
                        .unwrap();
                } else {
                    sender.input(DialogInput::Cancel);
                }
            }
            DialogInput::Cancel => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                // Dropping the worker kills the subprocess if it is still running.
                self.worker = None;
            }
            DialogInput::Echo(line) => {
                self.debug_view.emit(DebugViewInput::Append(line));
            }
            DialogInput::Cmd(cmd) => match cmd {
                Ok(cmd_msg) => match cmd_msg {
                    CmdMsg::GetData => {
                        let data = self
                            .app_session
                            .as_ref()
                            .expect("there should be some current state to accept")
                            .get_data();
                        self.send_response(ResultMsg::generate_data_msg(data));
                    }
                    CmdMsg::SetData(data_stream) => {
                        let app_session = self
                            .app_session
                            .as_mut()
                            .expect("there should be some current state to accept");
                        let data: collomatique_state_colloscopes::Data = data_stream.into();
                        let op = collomatique_state_colloscopes::Op::GlobalUpdate(
                            data.into_inner_data(),
                        );
                        let desc = (
                            collomatique_ops::OpCategory::None,
                            String::from("Mise à jour globale"),
                        );
                        match collomatique_state::traits::Manager::apply(app_session, op, desc) {
                            Ok(_) => {
                                self.send_response(ResultMsg::Ack(None));
                            }
                            Err(e) => {
                                self.send_response(ResultMsg::GlobalError(e.to_string()));
                            }
                        }
                    }
                    CmdMsg::Solver(_) | CmdMsg::Strategy(_) => {}
                },
                Err(e) => {
                    if !e.payload().is_empty() {
                        self.add_error(sender, e.to_string());
                    }
                    self.send_response(ResultMsg::InvalidMsg);
                }
            },
            DialogInput::Accept => {
                if !self.hidden {
                    self.hidden = true;
                    sender.output(DialogOutput::PresentParent).unwrap();
                }
                let app_session = self
                    .app_session
                    .take()
                    .expect("there should be some current state to accept");
                let last_op_cat = match app_session.get_undo_name() {
                    Some((cat, _desc)) => cat.clone(),
                    None => collomatique_ops::OpCategory::None,
                };
                sender
                    .output(DialogOutput::NewData(app_session.commit((
                        last_op_cat,
                        format!("Exécution de {}", self.path.to_string_lossy()),
                    ))))
                    .unwrap();
            }
            DialogInput::ProcessFinished => {
                self.worker = None;
            }
            DialogInput::Error(error) => {
                self.end_with_error = true;
                self.error_dialog
                    .sender()
                    .send(error_dialog::DialogInput::Show(error))
                    .unwrap();
            }
            DialogInput::Present => {
                self.move_front = true;
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
        if self.move_front {
            widgets.root_window.present();
        }
        if self.adjust_scrolling {
            let adj = widgets.scrolled_window.vadjustment();
            adj.set_value(adj.upper());
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        _sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        self.move_front = false;
        match message {
            DialogCmdOutput::AdjustScrolling => {
                self.adjust_scrolling = true;
            }
        }
    }
}

impl Dialog {
    fn has_modifications(&self) -> bool {
        self.app_session.as_ref().map_or(false, |s| s.can_undo())
    }

    fn send_response(&self, msg: ResultMsg) {
        if let Some(worker) = self.worker.as_ref() {
            match worker.send_rpc_message(msg) {
                // The worker already exited: nothing to respond to (also handled by the
                // separate `ProcessFinished` event). Harmless, so ignore it.
                Ok(()) | Err(SendError::Finished) => {}
                Err(SendError::Io(e)) => {
                    eprintln!("Erreur d'envoi de la réponse RPC au sous-processus : {e}");
                }
            }
        }
    }

    fn add_error(&mut self, sender: ComponentSender<Self>, data: String) {
        self.errors.guard().push_back(data);
        sender.oneshot_command(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            DialogCmdOutput::AdjustScrolling
        });
    }
}
