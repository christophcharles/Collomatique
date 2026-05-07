use std::sync::Mutex;

use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};

use super::generic_process::{GenericProcess, GenericProcessEvent, OutputData, ProcessStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RpcProcessId(pub(super) u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RpcProcessEvent {
    LogLine(String),
    RpcCommand(Result<CmdMsg, String>),
    GracefulExit,
    ProcessExited(Option<u32>),
    Error(String),
}

pub struct RpcProcessState {
    pub status: ProcessStatus,
    pub log_lines: Vec<String>,
    pub errors: Vec<String>,
}

pub struct RpcProcess {
    state: RpcProcessState,
    init_msg: InitMsg,
    inner: GenericProcess,
}

impl RpcProcess {
    pub(super) fn spawn<F>(init_msg: InitMsg, callback: F) -> Result<Self, String>
    where
        F: Fn(RpcProcessEvent) + Send + 'static,
    {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Impossible de déterminer l'exécutable courant : {}", e))?;
        let exe_str = exe.to_str().ok_or_else(|| {
            "Le chemin de l'exécutable contient des caractères non-UTF-8".to_string()
        })?;

        let current_cmd: Mutex<String> = Mutex::new(String::new());

        let rpc_callback = move |event: GenericProcessEvent| match event {
            GenericProcessEvent::Stdout(data) => {
                let line = match data {
                    OutputData::Utf8(s) => s,
                    OutputData::Raw(bytes) => {
                        callback(RpcProcessEvent::Error(format!(
                            "Données non-UTF-8 reçues ({} octets)",
                            bytes.len()
                        )));
                        return;
                    }
                };

                if !EncodedMsg::check_if_msg(&line) {
                    callback(RpcProcessEvent::LogLine(line));
                    return;
                }

                let mut cmd_buf = current_cmd.lock().unwrap();
                *cmd_buf += &line;

                if !EncodedMsg::check_if_end(&line) {
                    return;
                }

                let raw = std::mem::take(&mut *cmd_buf);
                drop(cmd_buf);

                let encoded_msg = EncodedMsg::from_raw_string(raw);
                let complete_cmd = encoded_msg.and_then(CompleteCmdMsg::try_from);
                match complete_cmd {
                    Ok(CompleteCmdMsg::CmdMsg(cmd)) => {
                        callback(RpcProcessEvent::RpcCommand(Ok(cmd)));
                    }
                    Ok(CompleteCmdMsg::GracefulExit) => {
                        callback(RpcProcessEvent::GracefulExit);
                    }
                    Err(e) => {
                        callback(RpcProcessEvent::RpcCommand(Err(e)));
                    }
                }
            }
            GenericProcessEvent::Stderr(_) => {
                // PTY merges streams, this shouldn't happen
            }
            GenericProcessEvent::ProcessExited(code) => {
                callback(RpcProcessEvent::ProcessExited(code));
            }
            GenericProcessEvent::Error(e) => {
                callback(RpcProcessEvent::Error(e));
            }
        };

        let inner = GenericProcess::spawn_pty(exe_str, &["--rpc-engine"], rpc_callback)?;

        // Send the init message
        let encoded = EncodedMsg::from(init_msg.clone());
        inner
            .send_stdin(encoded.encode().as_bytes())
            .map_err(|e| format!("Erreur à l'envoi du message initial : {}", e))?;

        Ok(RpcProcess {
            state: RpcProcessState {
                status: ProcessStatus::Running,
                log_lines: Vec::new(),
                errors: Vec::new(),
            },
            init_msg,
            inner,
        })
    }

    pub fn send_rpc_message(&self, msg: ResultMsg) -> Result<(), String> {
        let encoded = EncodedMsg::from(msg);
        self.inner.send_stdin(encoded.encode().as_bytes())
    }

    pub fn kill(&self) -> Result<(), String> {
        self.inner.kill()
    }

    pub fn state(&self) -> &RpcProcessState {
        &self.state
    }

    pub fn init_msg(&self) -> &InitMsg {
        &self.init_msg
    }

    pub fn handle_event(&mut self, event: &RpcProcessEvent) {
        match event {
            RpcProcessEvent::LogLine(line) => {
                self.state.log_lines.push(line.clone());
            }
            RpcProcessEvent::RpcCommand(Err(e)) => {
                self.state.errors.push(e.clone());
            }
            RpcProcessEvent::RpcCommand(Ok(_)) => {}
            RpcProcessEvent::GracefulExit => {
                self.state.status = ProcessStatus::Exited(None);
            }
            RpcProcessEvent::ProcessExited(code) => {
                self.state.status = ProcessStatus::Exited(*code);
            }
            RpcProcessEvent::Error(e) => {
                self.state.errors.push(e.clone());
            }
        }
    }
}
