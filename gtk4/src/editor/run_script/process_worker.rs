use relm4::{Component, ComponentParts, ComponentSender};

#[derive(Debug)]
pub enum ProcessWorkerInput {
    RunScript(String),
    SendStdin(String),
    KillScript,
}

#[derive(Debug)]
pub enum ProcessWorkerOutput {
    StdOut(String),
    StdErr(String),
    ScriptFinished,
    Error(String),
}

#[derive(Debug)]
pub enum ProcessWorkerCmdOutput {
    CheckRunning,
    ProcessKilled(std::io::Result<()>),
    ProcessLaunched(std::io::Result<tokio::process::Child>),
}

pub struct ProcessWorker {
    child_process: Option<tokio::process::Child>,
    launching: bool,
}

impl Component for ProcessWorker {
    type Init = ();
    type Input = ProcessWorkerInput;
    type Output = ProcessWorkerOutput;
    type CommandOutput = ProcessWorkerCmdOutput;
    type Root = ();
    type Widgets = ();

    fn init_root() -> Self::Root {}

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = ProcessWorker {
            child_process: None,
            launching: false,
        };

        ComponentParts { model, widgets: () }
    }

    fn update(
        &mut self,
        msg: ProcessWorkerInput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match msg {
            ProcessWorkerInput::RunScript(_script) => {
                if self.child_process.is_some() || self.launching {
                    panic!("A Python engine is already running!");
                }

                self.launching = true;

                sender.oneshot_command(async move {
                    ProcessWorkerCmdOutput::ProcessLaunched(
                        tokio::process::Command::new(std::env::current_exe().unwrap())
                            .arg("--python-engine")
                            .stdin(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .stdout(std::process::Stdio::piped())
                            .kill_on_drop(true)
                            .spawn(),
                    )
                });
            }
            ProcessWorkerInput::KillScript => {
                if let Some(mut child) = self.child_process.take() {
                    sender.oneshot_command(async move {
                        ProcessWorkerCmdOutput::ProcessKilled(child.kill().await)
                    });
                }
            }
            ProcessWorkerInput::SendStdin(_content) => {}
        }
    }

    fn update_cmd(
        &mut self,
        message: Self::CommandOutput,
        sender: ComponentSender<Self>,
        _root: &Self::Root,
    ) {
        match message {
            ProcessWorkerCmdOutput::CheckRunning => {
                let Some(child) = &mut self.child_process else {
                    return;
                };

                match child.try_wait() {
                    Ok(status) => match status {
                        Some(s) => {
                            self.child_process = None;
                            if !s.success() {
                                sender
                                    .output(ProcessWorkerOutput::Error(match s.code() {
                                        Some(v) => format!("Processus arrêté avec le statut {}", v),
                                        None => "Processus arrêté mais aucun statut reçu".into(),
                                    }))
                                    .unwrap();
                            }
                            sender.output(ProcessWorkerOutput::ScriptFinished).unwrap();
                        }
                        None => {
                            self.schedule_check(sender);
                        }
                    },
                    Err(e) => {
                        sender
                            .output(ProcessWorkerOutput::Error(e.to_string()))
                            .unwrap();
                        self.schedule_check(sender);
                    }
                }
            }
            ProcessWorkerCmdOutput::ProcessKilled(result) => {
                if let Err(e) = result {
                    sender
                        .output(ProcessWorkerOutput::Error(format!(
                            "Erreur à l'arrêt du processus : {}",
                            e.to_string()
                        )))
                        .unwrap();
                }
            }
            ProcessWorkerCmdOutput::ProcessLaunched(child_result) => {
                if self.child_process.is_some() {
                    panic!("A Python engine is already running!");
                }
                self.launching = false;

                match child_result {
                    Ok(child) => {
                        self.child_process = Some(child);
                        self.schedule_check(sender);
                    }
                    Err(e) => {
                        sender
                            .output(ProcessWorkerOutput::Error(e.to_string()))
                            .unwrap();
                    }
                }
            }
        }
    }
}

impl ProcessWorker {
    fn schedule_check(&mut self, sender: ComponentSender<Self>) {
        sender.oneshot_command(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            ProcessWorkerCmdOutput::CheckRunning
        });
    }
}
