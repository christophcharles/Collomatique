use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
use relm4::{gtk, Component};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt};

use tokio::io::BufReader;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

use collomatique_rpc::{CmdMsg, InitMsg, OutMsg};

#[derive(Debug)]
pub enum RpcLoggerInput {
    RunRcpEngine(InitMsg),
    SendMsg(OutMsg),
    KillProcess,
}

#[derive(Debug)]
pub enum RpcLoggerOutput {
    ProcessFinished,
    Cmd(Result<CmdMsg, String>),
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BufferOp {
    Clear,
    Insert(String),
}

pub struct RpcLogger {
    buffer_op: Option<BufferOp>,
    child_process: Option<tokio::process::Child>,
    launching: bool,
    child_stdin: Option<tokio::process::ChildStdin>,
}

impl RpcLogger {
    pub fn is_running(&self) -> bool {
        self.child_process.is_some() || self.launching
    }
}

#[derive(Debug)]
pub enum LoggerCommandOutput {
    CheckRunning,
    ProcessKilled(std::io::Result<()>),
    ProcessLaunched(std::io::Result<tokio::process::Child>, InitMsg),
    NewStdoutData(String, BufReader<tokio::process::ChildStdout>),
    NewStderrData(String, BufReader<tokio::process::ChildStderr>),
    MsgSent(std::io::Result<()>, tokio::process::ChildStdin),
}

#[relm4::component(pub)]
impl Component for RpcLogger {
    type Input = RpcLoggerInput;
    type Output = RpcLoggerOutput;
    type Init = ();
    type CommandOutput = LoggerCommandOutput;

    view! {
        #[root]
        gtk::ScrolledWindow {
            set_hexpand: true,
            set_policy: (gtk::PolicyType::Never, gtk::PolicyType::Automatic),
            set_margin_all: 5,
            #[name(text_view)]
            gtk::TextView {
                add_css_class: "frame",
                add_css_class: "osd",
                set_wrap_mode: gtk::WrapMode::Char,
                set_editable: false,
                set_monospace: true,
                #[name(text_buffer)]
                #[wrap(Some)]
                set_buffer = &gtk::TextBuffer {
                    #[track(model.buffer_op == Some(BufferOp::Clear))]
                    set_text: "",
                },
            }
        }
    }

    fn init(
        _params: Self::Init,
        root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = RpcLogger {
            buffer_op: None,
            child_process: None,
            launching: false,
            child_stdin: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            RpcLoggerInput::RunRcpEngine(init_msg) => {
                if self.is_running() {
                    panic!("A Python engine is already running!");
                }
                self.launching = true;
                self.buffer_op = Some(BufferOp::Clear);

                sender.oneshot_command(async move {
                    LoggerCommandOutput::ProcessLaunched(
                        tokio::process::Command::new(std::env::current_exe().unwrap())
                            .arg("--rpc-engine")
                            .stdin(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::piped())
                            .kill_on_drop(true)
                            .spawn(),
                        init_msg,
                    )
                });
            }
            RpcLoggerInput::KillProcess => {
                if let Some(mut child) = self.child_process.take() {
                    self.child_stdin = None;
                    sender.oneshot_command(async move {
                        LoggerCommandOutput::ProcessKilled(child.kill().await)
                    });
                }
            }
            RpcLoggerInput::SendMsg(out_msg) => {
                if self.child_stdin.is_none() {
                    panic!("No running child process to send a message to");
                }
                self.send_text_cmd(sender.clone(), out_msg.into_text_msg());
            }
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            LoggerCommandOutput::CheckRunning => {
                let Some(child) = &mut self.child_process else {
                    return;
                };

                match child.try_wait() {
                    Ok(status) => match status {
                        Some(s) => {
                            self.child_process = None;
                            if !s.success() {
                                sender
                                    .output(RpcLoggerOutput::Error(match s.code() {
                                        Some(v) => format!("Processus arrêté avec le statut {}", v),
                                        None => "Processus arrêté mais aucun statut reçu".into(),
                                    }))
                                    .unwrap();
                            }
                            sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                        }
                        None => {
                            self.schedule_check(sender);
                        }
                    },
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(e.to_string()))
                            .unwrap();
                        self.schedule_check(sender);
                    }
                }
            }
            LoggerCommandOutput::ProcessKilled(result) => {
                if let Err(e) = result {
                    sender
                        .output(RpcLoggerOutput::Error(format!(
                            "Erreur à l'arrêt du processus : {}",
                            e.to_string()
                        )))
                        .unwrap();
                } else {
                    sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                }
            }
            LoggerCommandOutput::ProcessLaunched(child_result, init_msg) => {
                if self.child_process.is_some() {
                    panic!("A Python engine is already running!");
                }

                match child_result {
                    Ok(mut child) => {
                        let stdout_opt = child.stdout.take();
                        let stderr_opt = child.stderr.take();

                        self.child_process = Some(child);
                        self.launching = false;
                        if let Some(stdout) = stdout_opt {
                            let stdout_buf = BufReader::new(stdout);
                            self.wait_stdout_data(sender.clone(), stdout_buf);
                        }
                        if let Some(stderr) = stderr_opt {
                            let stderr_buf = BufReader::new(stderr);
                            self.wait_stderr_data(sender.clone(), stderr_buf);
                        }

                        self.send_text_cmd(sender.clone(), init_msg.into_text_msg());
                        self.schedule_check(sender);
                    }
                    Err(e) => {
                        self.launching = false;
                        sender
                            .output(RpcLoggerOutput::Error(e.to_string()))
                            .unwrap();
                        sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                    }
                }
            }
            LoggerCommandOutput::NewStdoutData(data, stdout_buf) => {
                self.buffer_op = Some(BufferOp::Insert(data));
                if self.child_process.is_some() {
                    self.wait_stdout_data(sender, stdout_buf);
                }
            }
            LoggerCommandOutput::NewStderrData(data, stderr_buf) => {
                let cmd = CmdMsg::from_text_msg(&data);
                sender.output(RpcLoggerOutput::Cmd(cmd)).unwrap();
                // Process content and turn into command
                if self.child_process.is_some() {
                    self.wait_stderr_data(sender, stderr_buf);
                }
            }
            LoggerCommandOutput::MsgSent(result, child_stdin) => {
                self.child_stdin = Some(child_stdin);
                if let Err(e) = result {
                    sender
                        .output(RpcLoggerOutput::Error(format!(
                            "Envoie dans une RPC : {}",
                            e.to_string()
                        )))
                        .unwrap();
                }
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
        self.update_buffer_if_needed(widgets);
    }

    fn update_cmd_with_view(
        &mut self,
        widgets: &mut Self::Widgets,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        root: &Self::Root,
    ) {
        self.update_cmd(message, sender.clone(), root);
        self.update_view(widgets, sender);
        self.update_buffer_if_needed(widgets);
    }
}

impl RpcLogger {
    fn update_buffer_if_needed(&mut self, widgets: &mut <Self as Component>::Widgets) {
        if let Some(BufferOp::Insert(content)) = self.buffer_op.take() {
            let mut end_iter = widgets.text_buffer.end_iter();
            widgets.text_buffer.insert(&mut end_iter, &content);
            let mut end_iter = widgets.text_buffer.end_iter();
            widgets
                .text_view
                .scroll_to_iter(&mut end_iter, 0., false, 0., 0.);
        }
    }
}

impl RpcLogger {
    fn schedule_check(&mut self, sender: ComponentSender<Self>) {
        sender.oneshot_command(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            LoggerCommandOutput::CheckRunning
        });
    }

    fn wait_stdout_data(
        &mut self,
        sender: ComponentSender<Self>,
        mut stdout_buf: BufReader<tokio::process::ChildStdout>,
    ) {
        sender.oneshot_command(async move {
            let mut line = String::new();
            stdout_buf.read_line(&mut line).await.unwrap();
            LoggerCommandOutput::NewStdoutData(line, stdout_buf)
        });
    }

    fn wait_stderr_data(
        &mut self,
        sender: ComponentSender<Self>,
        mut stderr_buf: BufReader<tokio::process::ChildStderr>,
    ) {
        sender.oneshot_command(async move {
            let mut line = String::new();
            stderr_buf.read_line(&mut line).await.unwrap();
            LoggerCommandOutput::NewStderrData(line, stderr_buf)
        });
    }

    fn send_text_cmd(&mut self, sender: ComponentSender<Self>, cmd: String) {
        if let Some(child) = &mut self.child_process {
            let mut child_stdin = child.stdin.take().expect("No other command being sent");
            sender.oneshot_command(async move {
                LoggerCommandOutput::MsgSent(
                    child_stdin.write_all(cmd.as_bytes()).await,
                    child_stdin,
                )
            });
        }
    }
}
