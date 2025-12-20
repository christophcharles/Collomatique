use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
use relm4::{gtk, Component};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt};

use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use std::io::Write;

use collomatique_rpc::{CmdMsg, CompleteCmdMsg, InitMsg, ResultMsg};

#[derive(Debug)]
pub enum RpcLoggerInput {
    RunRcpEngine(InitMsg),
    SendMsg(ResultMsg),
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
    child_process: Option<std::process::Child>,
    child_stdin: Option<std::process::ChildStdin>,
}

impl RpcLogger {
    pub fn is_running(&self) -> bool {
        self.child_process.is_some()
    }
}

#[derive(Debug)]
pub enum LoggerCommandOutput {
    CheckRunning,
    NewStdoutData(String, BufReader<tokio::process::ChildStdout>),
    NewStderrData(String, BufReader<tokio::process::ChildStderr>),
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
            set_vexpand: true,
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
            child_stdin: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>, _root: &Self::Root) {
        match message {
            RpcLoggerInput::RunRcpEngine(init_msg) => {
                if self.is_running() {
                    panic!("An RPC engine is already running!");
                }
                self.buffer_op = Some(BufferOp::Clear);

                let child_result = std::process::Command::new(std::env::current_exe().unwrap())
                    .arg("--rpc-engine")
                    .stdin(std::process::Stdio::piped())
                    .stderr(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::piped())
                    .spawn();

                match child_result {
                    Ok(mut child) => {
                        let stdout_opt = child.stdout.take();
                        let stderr_opt = child.stderr.take();
                        let stdin_opt = child.stdin.take();

                        self.child_process = Some(child);
                        if let Some(stdout) = stdout_opt {
                            match tokio::process::ChildStdout::from_std(stdout) {
                                Ok(stdout) => {
                                    let stdout_buf = BufReader::new(stdout);
                                    self.wait_stdout_data(sender.clone(), stdout_buf);
                                }
                                Err(e) => {
                                    sender
                                        .output(RpcLoggerOutput::Error(format!(
                                            "Erreur à l'acquisition de la sortie standard : {}",
                                            e.to_string()
                                        )))
                                        .unwrap();
                                }
                            };
                        }
                        if let Some(stderr) = stderr_opt {
                            match tokio::process::ChildStderr::from_std(stderr) {
                                Ok(stderr) => {
                                    let stderr_buf = BufReader::new(stderr);
                                    self.wait_stderr_data(sender.clone(), stderr_buf);
                                }
                                Err(e) => {
                                    sender
                                        .output(RpcLoggerOutput::Error(format!(
                                            "Erreur à l'acquisition de la sortie d'erreur : {}",
                                            e.to_string()
                                        )))
                                        .unwrap();
                                }
                            };
                        }
                        match stdin_opt {
                            Some(stdin) => {
                                self.child_stdin = Some(stdin);
                            }
                            None => {
                                sender
                                    .output(RpcLoggerOutput::Error(format!(
                                        "Erreur à l'acquisition de l'entrée standard"
                                    )))
                                    .unwrap();
                            }
                        }

                        self.send_text_cmd(sender.clone(), init_msg.into_text_msg());
                        self.schedule_check(sender);
                    }
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'exécution du sous-processus : {}",
                                e.to_string()
                            )))
                            .unwrap();
                        sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                    }
                }
            }
            RpcLoggerInput::KillProcess => {
                if let Some(mut child) = self.child_process.take() {
                    self.child_stdin = None;

                    if let Err(e) = child.kill() {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'arrêt du processus : {}",
                                e.to_string()
                            )))
                            .unwrap();
                    }
                    if let Err(e) = child.wait() {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'arrêt du processus : {}",
                                e.to_string()
                            )))
                            .unwrap();
                    }

                    sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                }
            }
            RpcLoggerInput::SendMsg(out_msg) => {
                self.send_text_cmd(sender, out_msg.into_text_msg());
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
            LoggerCommandOutput::NewStderrData(data, stderr_buf) => {
                self.buffer_op = Some(BufferOp::Insert(data));
                if self.child_process.is_some() {
                    self.wait_stderr_data(sender, stderr_buf);
                }
            }
            LoggerCommandOutput::NewStdoutData(data, stdout_buf) => {
                if !CompleteCmdMsg::check_if_msg(&data) {
                    self.buffer_op = Some(BufferOp::Insert(data));
                    if self.child_process.is_some() {
                        self.wait_stdout_data(sender, stdout_buf);
                    }
                    return;
                }

                let complete_cmd = CompleteCmdMsg::from_text_msg(&data);
                let cmd = match complete_cmd {
                    Ok(c) => match c {
                        CompleteCmdMsg::CmdMsg(cmd) => Ok(cmd),
                        CompleteCmdMsg::GracefulExit => {
                            self.child_stdin = None;
                            if let Some(mut child_process) = self.child_process.take() {
                                if let Err(e) = child_process.wait() {
                                    sender
                                        .output(RpcLoggerOutput::Error(format!(
                                            "Erreur à l'arrêt du processus : {}",
                                            e.to_string()
                                        )))
                                        .unwrap();
                                }
                            }
                            sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                            return;
                        }
                    },
                    Err(e) => Err(e),
                };

                sender.output(RpcLoggerOutput::Cmd(cmd)).unwrap();
                // Process content and turn into command
                if self.child_process.is_some() {
                    self.wait_stdout_data(sender, stdout_buf);
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
        if let Some(stdin) = &mut self.child_stdin {
            let result = stdin.write_all(cmd.as_bytes());
            if let Err(e) = result {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    return;
                }
                sender
                    .output(RpcLoggerOutput::Error(format!(
                        "Erreur dans une RPC : {}",
                        e.to_string()
                    )))
                    .unwrap();
                return;
            }
            let result = stdin.write_all("\r\n".as_bytes());
            if let Err(e) = result {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    return;
                }
                sender
                    .output(RpcLoggerOutput::Error(format!(
                        "Erreur dans une RPC : {}",
                        e.to_string()
                    )))
                    .unwrap();
                return;
            }
            let result = stdin.flush();
            if let Err(e) = result {
                if e.kind() == std::io::ErrorKind::BrokenPipe {
                    return;
                }
                sender
                    .output(RpcLoggerOutput::Error(format!(
                        "Erreur dans une RPC : {}",
                        e.to_string()
                    )))
                    .unwrap();
                return;
            }
        }
    }
}
