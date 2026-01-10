use collomatique_rpc::gui_answer::OpenFileDialogAnswer;
use collomatique_rpc::{CmdMsg, ResultMsg};
use collomatique_state::traits::Manager;
use gtk::prelude::{AdjustmentExt, BoxExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk, Component, ComponentController};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use collomatique_ops::Desc;
use collomatique_state::{AppSession, AppState};
use collomatique_state_colloscopes::Data;

use crate::widgets::rpc_server;
use std::path::PathBuf;

mod confirm_dialog;
mod error_dialog;
mod input_dialog;
mod msg_display;
mod ok_dialog;
mod warning_running;

pub struct Dialog {
    hidden: bool,
    run_type: RunType,
    is_running: bool,
    end_with_error: bool,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    ok_dialog: Controller<ok_dialog::Dialog>,
    confirm_dialog: Controller<confirm_dialog::Dialog>,
    input_dialog: Controller<input_dialog::Dialog>,
    rpc_logger: Controller<rpc_server::RpcLogger>,
    commands: FactoryVecDeque<msg_display::Entry>,
    adjust_scrolling: bool,
    app_session: Option<AppSession<AppState<Data, Desc>, Desc>>,
}

#[derive(Debug)]
pub enum RunType {
    Script(PathBuf, String),
    SolveColloscope,
}

impl RunType {
    fn get_path(&self) -> Option<std::path::PathBuf> {
        match self {
            RunType::Script(path, _script) => Some(path.clone()),
            RunType::SolveColloscope => None,
        }
    }

    fn get_init_msg(&self) -> collomatique_rpc::InitMsg {
        match self {
            RunType::Script(_path, script) => {
                collomatique_rpc::InitMsg::RunPythonScript(script.clone())
            }
            RunType::SolveColloscope => collomatique_rpc::InitMsg::SolveColloscope,
        }
    }

    fn get_under_text(&self) -> String {
        match self {
            RunType::Script(path, _script) => path.to_string_lossy().to_string(),
            RunType::SolveColloscope => "Résolution du colloscope".into(),
        }
    }

    fn get_op_name(&self) -> String {
        match self {
            RunType::Script(path, _script) => {
                format!("Exécution de {}", path.to_string_lossy())
            }
            RunType::SolveColloscope => "Résolution du colloscope".into(),
        }
    }

    fn get_title(&self) -> String {
        match self {
            RunType::Script(_, _) => "Exécution du script Python".to_string(),
            RunType::SolveColloscope => "Résolution du colloscope".into(),
        }
    }
}

#[derive(Debug)]
pub enum DialogInput {
    Run(RunType, AppState<Data, Desc>),
    CancelRequest,
    Accept,

    Cancel,
    ProcessFinished,
    Cmd(Result<collomatique_rpc::CmdMsg, String>),
    Error(String),
}

#[derive(Debug)]
pub enum DialogCmdOutput {
    AdjustScrolling,
    DelayedRpcAnswer(ResultMsg),
}

#[derive(Debug)]
pub enum DialogOutput {
    NewData(AppState<Data, Desc>),
}

#[relm4::component(pub)]
impl Component for Dialog {
    type Init = ();

    type Input = DialogInput;
    type Output = DialogOutput;
    type CommandOutput = DialogCmdOutput;

    view! {
        #[root]
        adw::Window {
            set_modal: true,
            set_default_size: (700, 400),
            set_resizable: true,
            #[watch]
            set_visible: !model.hidden,
            #[watch]
            set_title: Some(&model.run_type.get_title()),
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
                        set_sensitive: !model.is_running && !model.commands.is_empty(),
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
                        set_visible: model.is_running,
                    },
                    gtk::Box {
                        set_orientation: gtk::Orientation::Horizontal,
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        #[watch]
                        set_visible: !model.is_running && !model.end_with_error && !model.commands.is_empty(),
                        gtk::Image::from_icon_name("emblem-ok-symbolic") {
                            set_size_request: (50, 50),
                            set_icon_size: gtk::IconSize::Large,
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
                        set_visible: !model.is_running && !model.end_with_error && model.commands.is_empty(),
                        gtk::Image::from_icon_name("dialog-warning-symbolic") {
                            set_size_request: (50, 50),
                            set_icon_size: gtk::IconSize::Large,
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
                        set_visible: !model.is_running && model.end_with_error,
                        gtk::Image::from_icon_name("dialog-error-symbolic") {
                            set_size_request: (50, 50),
                            set_icon_size: gtk::IconSize::Large,
                        },
                        gtk::Label {
                            set_label: "Erreur pendant l'exécution",
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        set_halign: gtk::Align::Start,
                        set_label: "Opérations effectuées :",
                    },
                    gtk::Paned {
                        set_hexpand: true,
                        set_vexpand: true,
                        set_margin_all: 5,
                        set_orientation: gtk::Orientation::Vertical,
                        #[wrap(Some)]
                        set_start_child = &gtk::Box {
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Vertical,
                            #[name(scrolled_window)]
                            gtk::ScrolledWindow {
                                set_hexpand: true,
                                set_vexpand: true,
                                set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
                                set_margin_all: 5,
                                #[local_ref]
                                cmds_listbox -> gtk::ListBox {
                                    set_hexpand: true,
                                    add_css_class: "boxed-list",
                                    set_selection_mode: gtk::SelectionMode::None,
                                }
                            },
                        },
                        #[wrap(Some)]
                        set_end_child = &gtk::Box {
                            set_margin_all: 5,
                            set_hexpand: true,
                            set_orientation: gtk::Orientation::Vertical,
                            gtk::Label {
                                set_halign: gtk::Align::Start,
                                set_label: "Informations de débogage :",
                            },
                            append = model.rpc_logger.widget(),
                        },
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        add_css_class: "dimmed",
                        #[watch]
                        set_label: &model.run_type.get_under_text(),
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
        let error_dialog = error_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .detach();

        let warning_running = warning_running::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                warning_running::DialogOutput::Accept => DialogInput::Cancel,
            });

        let ok_dialog = ok_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.command_sender(), |msg| match msg {
                ok_dialog::DialogOutput::Ok => DialogCmdOutput::DelayedRpcAnswer(
                    ResultMsg::AckGui(collomatique_rpc::GuiAnswer::OkDialogClosed),
                ),
            });

        let confirm_dialog = confirm_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.command_sender(), |msg| match msg {
                confirm_dialog::DialogOutput::Confirmed => DialogCmdOutput::DelayedRpcAnswer(
                    ResultMsg::AckGui(collomatique_rpc::GuiAnswer::ConfirmDialog(true)),
                ),
                confirm_dialog::DialogOutput::Cancelled => DialogCmdOutput::DelayedRpcAnswer(
                    ResultMsg::AckGui(collomatique_rpc::GuiAnswer::ConfirmDialog(false)),
                ),
            });

        let input_dialog = input_dialog::Dialog::builder()
            .transient_for(&root)
            .launch(())
            .forward(sender.command_sender(), |msg| match msg {
                input_dialog::DialogOutput::Accepted(text) => DialogCmdOutput::DelayedRpcAnswer(
                    ResultMsg::AckGui(collomatique_rpc::GuiAnswer::InputDialog(Some(text))),
                ),
                input_dialog::DialogOutput::Cancelled => DialogCmdOutput::DelayedRpcAnswer(
                    ResultMsg::AckGui(collomatique_rpc::GuiAnswer::InputDialog(None)),
                ),
            });

        use rpc_server::{RpcLogger, RpcLoggerOutput};
        let rpc_logger = RpcLogger::builder()
            .launch(())
            .forward(sender.input_sender(), |msg| match msg {
                RpcLoggerOutput::ProcessFinished => DialogInput::ProcessFinished,
                RpcLoggerOutput::Cmd(cmd) => DialogInput::Cmd(cmd),
                RpcLoggerOutput::Error(e) => DialogInput::Error(e),
            });

        let commands = FactoryVecDeque::builder()
            .launch(gtk::ListBox::default())
            .detach();

        let model = Dialog {
            hidden: true,
            run_type: RunType::Script(PathBuf::new(), String::new()),
            error_dialog,
            warning_running,
            ok_dialog,
            confirm_dialog,
            input_dialog,
            rpc_logger,
            is_running: false,
            end_with_error: false,
            commands,
            adjust_scrolling: false,
            app_session: None,
        };

        let cmds_listbox = model.commands.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.adjust_scrolling = false;
        match msg {
            DialogInput::Run(run_type, app_state) => {
                self.hidden = false;
                self.run_type = run_type;
                self.app_session = Some(AppSession::new(app_state));
                self.commands.guard().clear();
                self.is_running = true;
                self.end_with_error = false;
                self.rpc_logger
                    .sender()
                    .send(rpc_server::RpcLoggerInput::RunRcpEngine(
                        self.run_type.get_init_msg(),
                    ))
                    .unwrap();
            }
            DialogInput::CancelRequest => {
                if self.is_running {
                    self.warning_running
                        .sender()
                        .send(warning_running::DialogInput::Show)
                        .unwrap();
                } else {
                    sender.input(DialogInput::Cancel);
                }
            }
            DialogInput::Cancel => {
                self.hidden = true;
                self.rpc_logger
                    .sender()
                    .send(rpc_server::RpcLoggerInput::KillProcess)
                    .unwrap();
            }
            DialogInput::Cmd(cmd) => match cmd {
                Ok(cmd_msg) => {
                    let app_session = self
                        .app_session
                        .as_mut()
                        .expect("there should be some current state to accept");
                    let data = app_session.get_data();

                    let update_op = match cmd_msg {
                        CmdMsg::GetData => {
                            self.rpc_logger
                                .sender()
                                .send(rpc_server::RpcLoggerInput::SendMsg(
                                    ResultMsg::generate_data_msg(data),
                                ))
                                .unwrap();
                            return;
                        }
                        CmdMsg::GuiRequest(gui_cmd) => {
                            self.handle_gui_request(sender, gui_cmd);
                            return;
                        }
                        CmdMsg::Update(update_msg) => update_msg,
                    };

                    match update_op.apply(app_session) {
                        Ok(new_id) => {
                            self.add_command(
                                sender,
                                msg_display::EntryData::Success(update_op.get_desc().1),
                            );
                            self.rpc_logger
                                .sender()
                                .send(rpc_server::RpcLoggerInput::SendMsg(ResultMsg::Ack(
                                    new_id.map(|x| x.into()),
                                )))
                                .unwrap();
                        }
                        Err(e) => {
                            self.add_command(sender, msg_display::EntryData::Failed(e.to_string()));
                            self.rpc_logger
                                .sender()
                                .send(rpc_server::RpcLoggerInput::SendMsg(ResultMsg::Error(
                                    e.into(),
                                )))
                                .unwrap();
                        }
                    }
                }
                Err(e) => {
                    if !e.is_empty() {
                        self.add_command(sender, msg_display::EntryData::Invalid(e));
                    }
                    self.rpc_logger
                        .sender()
                        .send(rpc_server::RpcLoggerInput::SendMsg(ResultMsg::InvalidMsg))
                        .unwrap();
                }
            },
            DialogInput::Accept => {
                self.hidden = true;
                let app_session = self
                    .app_session
                    .take()
                    .expect("there should be some current state to accept");
                let last_op_cat = match app_session.get_undo_name() {
                    Some((cat, _desc)) => cat.clone(),
                    None => collomatique_ops::OpCategory::None,
                };
                sender
                    .output(DialogOutput::NewData(
                        app_session.commit((last_op_cat, self.run_type.get_op_name())),
                    ))
                    .unwrap();
            }
            DialogInput::ProcessFinished => {
                self.is_running = false;
            }
            DialogInput::Error(error) => {
                self.end_with_error = true;
                self.error_dialog
                    .sender()
                    .send(error_dialog::DialogInput::Show(error))
                    .unwrap();
            }
        }
    }

    fn post_view(&self, widgets: &mut Self::Widgets, _sender: ComponentSender<Self>) {
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
        match message {
            DialogCmdOutput::AdjustScrolling => {
                self.adjust_scrolling = true;
            }
            DialogCmdOutput::DelayedRpcAnswer(result_msg) => {
                self.rpc_logger
                    .sender()
                    .send(rpc_server::RpcLoggerInput::SendMsg(result_msg))
                    .unwrap();
            }
        }
    }
}

impl Dialog {
    fn add_command(&mut self, sender: ComponentSender<Self>, data: msg_display::EntryData) {
        self.commands.guard().push_back(data);
        sender.oneshot_command(async move {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            DialogCmdOutput::AdjustScrolling
        });
    }

    fn handle_gui_request(
        &mut self,
        sender: ComponentSender<Self>,
        gui_cmd: collomatique_rpc::cmd_msg::GuiMsg,
    ) {
        match gui_cmd {
            collomatique_rpc::cmd_msg::GuiMsg::OpenFileDialog(params) => {
                let path = self.run_type.get_path();
                sender.oneshot_command(async move {
                    let ext_vec: Vec<_> = params
                        .list
                        .iter()
                        .map(|ext| (ext.desc.as_str(), ext.extension.as_str()))
                        .collect();

                    let file_name = crate::tools::open_save::generic_open_dialog(
                        &params.title,
                        &ext_vec[..],
                        path.as_ref().map(|x| x.as_path()),
                    )
                    .await;

                    DialogCmdOutput::DelayedRpcAnswer(ResultMsg::AckGui(
                        collomatique_rpc::GuiAnswer::OpenFileDialog(OpenFileDialogAnswer {
                            file_path: file_name,
                        }),
                    ))
                });
            }
            collomatique_rpc::cmd_msg::GuiMsg::OkDialog(text) => {
                self.ok_dialog
                    .sender()
                    .send(ok_dialog::DialogInput::Show(text))
                    .unwrap();
            }
            collomatique_rpc::cmd_msg::GuiMsg::ConfirmDialog(text) => {
                self.confirm_dialog
                    .sender()
                    .send(confirm_dialog::DialogInput::Show(text))
                    .unwrap();
            }
            collomatique_rpc::cmd_msg::GuiMsg::InputDialog(info_text, placeholder_text) => {
                self.input_dialog
                    .sender()
                    .send(input_dialog::DialogInput::Show(info_text, placeholder_text))
                    .unwrap();
            }
        }
    }
}
