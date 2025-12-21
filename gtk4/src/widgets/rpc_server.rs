use gtk::prelude::{TextBufferExt, TextViewExt, WidgetExt};
use relm4::{gtk, Component};
use relm4::{ComponentParts, ComponentSender, RelmWidgetExt};

use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::io::BufRead;
use std::io::BufReader;

use std::io::Write;

use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};

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
    child_process: Option<Box<dyn portable_pty::Child + Send + Sync>>,
    child_stdin: Option<Box<dyn std::io::Write + Send>>,
    pty_master: Option<Box<dyn portable_pty::MasterPty + Send>>,
    current_cmd: String,
}

impl RpcLogger {
    pub fn is_running(&self) -> bool {
        self.child_process.is_some()
    }
}

pub enum LoggerCommandOutput {
    CheckRunning,
    NewStdoutData(String, BufReader<Box<dyn std::io::Read + Send>>),
}

impl std::fmt::Debug for LoggerCommandOutput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoggerCommandOutput::CheckRunning => f.debug_tuple("CheckRunning").finish(),
            LoggerCommandOutput::NewStdoutData(line, _reader) => {
                f.debug_tuple("NewStdoutData").field(line).finish()
            }
        }
    }
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
            pty_master: None,
            current_cmd: String::new(),
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
                self.current_cmd.clear();

                let pty_system = native_pty_system();

                // Create PTY with reasonable terminal size
                let pair = match pty_system.openpty(PtySize {
                    rows: 36,
                    cols: 120,
                    pixel_width: 0,
                    pixel_height: 0,
                }) {
                    Ok(p) => p,
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à  la création du PTY : {}",
                                e.to_string()
                            )))
                            .unwrap();
                        sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                        return;
                    }
                };

                // Disable echo on the PTY
                #[cfg(unix)]
                {
                    let fd = pair
                        .master
                        .as_raw_fd()
                        .expect("Should have a raw fd on UNIX platform");
                    unsafe {
                        let mut termios: libc::termios = std::mem::zeroed();
                        libc::tcgetattr(fd, &mut termios);
                        termios.c_lflag &= !libc::ECHO; // Disable echo
                        termios.c_lflag &= !libc::ECHONL; // Disable newline echo too
                        libc::tcsetattr(fd, libc::TCSANOW, &termios);
                    }
                }

                // Build command
                let mut cmd = CommandBuilder::new(std::env::current_exe().unwrap());
                cmd.arg("--rpc-engine");

                // Spawn the child in the PTY
                let child = match pair.slave.spawn_command(cmd) {
                    Ok(c) => c,
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'exécution du sous-processus : {}",
                                e.to_string()
                            )))
                            .unwrap();
                        sender.output(RpcLoggerOutput::ProcessFinished).unwrap();
                        return;
                    }
                };

                // Get reader from master (this is stdout+stderr combined)
                let reader_result = pair.master.try_clone_reader();

                match reader_result {
                    Ok(reader) => {
                        let buf_reader = std::io::BufReader::new(reader);
                        self.wait_stdout_data(sender.clone(), buf_reader);
                    }
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'acquisition du reader PTY : {}",
                                e.to_string()
                            )))
                            .unwrap();
                    }
                }

                // Get writer for stdin
                let writer = pair.master.take_writer();
                match writer {
                    Ok(writer) => {
                        self.child_stdin = Some(writer);
                    }
                    Err(e) => {
                        sender
                            .output(RpcLoggerOutput::Error(format!(
                                "Erreur à l'acquisition de l'entrée standard : {}",
                                e.to_string()
                            )))
                            .unwrap();
                    }
                }

                self.child_process = Some(child);
                self.pty_master = Some(pair.master);
                // Don't need slave anymore
                drop(pair.slave);

                let encoded_msg = EncodedMsg::from(init_msg);
                self.send_text_cmd(sender.clone(), encoded_msg.encode());
                self.schedule_check(sender);
            }
            RpcLoggerInput::KillProcess => {
                if let Some(mut child) = self.child_process.take() {
                    self.child_stdin = None;
                    self.pty_master = None;

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
                let encoded_msg = EncodedMsg::from(out_msg);
                self.send_text_cmd(sender, encoded_msg.encode());
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
                                    .output(RpcLoggerOutput::Error(format!(
                                        "Processus arrêté avec le statut {}",
                                        s.exit_code()
                                    )))
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
            LoggerCommandOutput::NewStdoutData(data, stdout_buf) => {
                if !EncodedMsg::check_if_msg(&data) {
                    self.buffer_op = Some(BufferOp::Insert(data));
                    if self.child_process.is_some() {
                        self.wait_stdout_data(sender, stdout_buf);
                    }
                    return;
                }

                self.current_cmd += &data;
                if !EncodedMsg::check_if_end(&data) {
                    if self.child_process.is_some() {
                        self.wait_stdout_data(sender, stdout_buf);
                    }
                    return;
                }

                let encoded_msg = EncodedMsg::from_raw_string(self.current_cmd.clone());
                self.current_cmd.clear();

                let complete_cmd = encoded_msg.map(|x| CompleteCmdMsg::try_from(x)).flatten();
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
        mut stdout_buf: BufReader<Box<dyn std::io::Read + Send>>,
    ) {
        sender.spawn_oneshot_command(move || {
            let mut line = String::new();
            stdout_buf.read_line(&mut line).unwrap();
            LoggerCommandOutput::NewStdoutData(line, stdout_buf)
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
