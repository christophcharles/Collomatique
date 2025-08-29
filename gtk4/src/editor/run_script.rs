use collomatique_rpc::{CmdMsg, OutMsg};
use gtk::prelude::{AdjustmentExt, ButtonExt, GtkWindowExt, OrientableExt, WidgetExt};
use relm4::factory::FactoryVecDeque;
use relm4::{adw, gtk, Component, ComponentController};
use relm4::{ComponentParts, ComponentSender, Controller, RelmWidgetExt};

use collomatique_state::AppState;
use collomatique_state_colloscopes::Data;

use crate::widgets::rpc_server;
use std::path::PathBuf;

mod error_dialog;
mod msg_display;
mod warning_running;

pub struct Dialog {
    hidden: bool,
    path: PathBuf,
    is_running: bool,
    error_dialog: Controller<error_dialog::Dialog>,
    warning_running: Controller<warning_running::Dialog>,
    rpc_logger: Controller<rpc_server::RpcLogger>,
    commands: FactoryVecDeque<msg_display::Entry>,
    adjust_scrolling: bool,
    app_state: AppState<Data>,
}

#[derive(Debug)]
pub enum DialogInput {
    Run(PathBuf, String, AppState<Data>),
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
}

#[derive(Debug)]
pub enum DialogOutput {
    NewData(AppState<Data>),
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
            set_size_request: (700, 400),
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
                        set_sensitive: !model.is_running,
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
                    gtk::Image::from_icon_name("emblem-ok-symbolic") {
                        set_halign: gtk::Align::Center,
                        set_valign: gtk::Align::Center,
                        set_size_request: (50, 50),
                        set_icon_size: gtk::IconSize::Large,
                        #[watch]
                        set_visible: !model.is_running,
                    },
                    gtk::Label {
                        set_margin_all: 5,
                        set_halign: gtk::Align::Start,
                        set_label: "Opérations effectuées :",
                    },
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
                    gtk::Expander {
                        set_margin_all: 5,
                        set_hexpand: true,
                        #[wrap(Some)]
                        set_label_widget = &gtk::Label {
                            set_label: "Informations de débogage",
                        },
                        set_child: Some(model.rpc_logger.widget()),
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
            path: PathBuf::new(),
            error_dialog,
            warning_running,
            rpc_logger,
            is_running: false,
            commands,
            adjust_scrolling: false,
            app_state: AppState::new(Data::new()),
        };

        let cmds_listbox = model.commands.widget();

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, msg: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        self.adjust_scrolling = false;
        match msg {
            DialogInput::Run(path, script, app_state) => {
                self.hidden = false;
                self.path = path;
                self.app_state = app_state;
                self.commands.guard().clear();
                self.is_running = true;
                self.rpc_logger
                    .sender()
                    .send(rpc_server::RpcLoggerInput::RunRcpEngine(
                        collomatique_rpc::InitMsg::RunPythonScript(script),
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
                    self.add_command(
                        sender,
                        match cmd_msg {
                            CmdMsg::Success => {
                                msg_display::EntryData::Success("Successful command".into())
                            }
                            CmdMsg::Warning => {
                                msg_display::EntryData::Failed("Failed command".into())
                            }
                        },
                    );
                    self.rpc_logger
                        .sender()
                        .send(rpc_server::RpcLoggerInput::SendMsg(OutMsg::Ack))
                        .unwrap();
                }
                Err(e) => {
                    self.add_command(sender, msg_display::EntryData::Invalid(e));
                    self.rpc_logger
                        .sender()
                        .send(rpc_server::RpcLoggerInput::SendMsg(OutMsg::Invalid))
                        .unwrap();
                }
            },
            DialogInput::Accept => {
                self.hidden = true;
                sender
                    .output(DialogOutput::NewData(self.app_state.clone()))
                    .unwrap();
            }
            DialogInput::ProcessFinished => {
                self.is_running = false;
            }
            DialogInput::Error(error) => {
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
}
