use std::sync::Mutex;

use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};

use crate::process::{OutputData, Process, ProcessEvent, SendError, StdinWriter};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    LogLine(String),
    RpcCommand(Result<CmdMsg, String>),
    GracefulExit,
    ProcessExited(Option<u32>),
    Error(String),
}

/// Owned RAII handle to an RPC worker subprocess (`<self> --rpc-engine`).
///
/// A `Worker` owns its backing [`Process`], so dropping it tears the subprocess down
/// (killing it if still running). It exposes the RPC framing on top of the raw process:
/// stdout is parsed into [`WorkerEvent`]s by the spawn callback, and [`Worker::send_rpc_message`]
/// frames outgoing [`ResultMsg`]s.
pub struct Worker {
    process: Process,
}

impl Worker {
    pub fn spawn<F>(init_msg: InitMsg, callback: F) -> Result<Worker, String>
    where
        F: Fn(WorkerEvent) + Send + 'static,
    {
        let exe = std::env::current_exe()
            .map_err(|e| format!("Impossible de déterminer l'exécutable courant : {}", e))?;
        let exe_str = exe.to_str().ok_or_else(|| {
            "Le chemin de l'exécutable contient des caractères non-UTF-8".to_string()
        })?;

        let current_cmd: Mutex<String> = Mutex::new(String::new());

        let rpc_callback = move |event: ProcessEvent| match event {
            ProcessEvent::Stdout(data) => {
                let line = match data {
                    OutputData::Utf8(s) => s,
                    OutputData::Raw(bytes) => {
                        callback(WorkerEvent::Error(format!(
                            "Données non-UTF-8 reçues ({} octets)",
                            bytes.len()
                        )));
                        return;
                    }
                };

                if !EncodedMsg::check_if_msg(&line) {
                    callback(WorkerEvent::LogLine(line));
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
                        callback(WorkerEvent::RpcCommand(Ok(cmd)));
                    }
                    Ok(CompleteCmdMsg::GracefulExit) => {
                        callback(WorkerEvent::GracefulExit);
                    }
                    Err(e) => {
                        callback(WorkerEvent::RpcCommand(Err(e)));
                    }
                }
            }
            ProcessEvent::Stderr(_) => {
                // PTY merges streams, this shouldn't happen
            }
            ProcessEvent::ProcessExited(code) => {
                callback(WorkerEvent::ProcessExited(code));
            }
            ProcessEvent::Error(e) => {
                callback(WorkerEvent::Error(e));
            }
        };

        let process = Process::spawn_pty(exe_str, &["--rpc-engine"], rpc_callback)?;

        let encoded = EncodedMsg::from(init_msg);
        process
            .send_stdin(encoded.encode().as_bytes())
            .map_err(|e| match e {
                SendError::Finished => {
                    "Le sous-processus s'est terminé avant l'envoi du message initial".to_string()
                }
                SendError::Io(msg) => format!("Erreur à l'envoi du message initial : {}", msg),
            })?;

        Ok(Worker { process })
    }

    /// A clone of the worker's stdin slot, for callbacks that need to write RPC responses
    /// back to the worker from the reader thread. Writes become no-ops once the worker exits.
    pub fn get_stdin_writer(&self) -> StdinWriter {
        self.process.get_stdin_writer()
    }

    /// Frame and send an RPC [`ResultMsg`] to the worker.
    ///
    /// Returns [`SendError::Finished`] if the worker has already exited or been killed,
    /// or [`SendError::Io`] on a genuine write failure.
    pub fn send_rpc_message(&self, msg: ResultMsg) -> Result<(), SendError> {
        let encoded = EncodedMsg::from(msg);
        self.process.send_stdin(encoded.encode().as_bytes())
    }

    /// Kill the worker if it is still running. Idempotent (see [`Process::kill`]).
    pub fn kill(&self) -> Result<(), String> {
        self.process.kill()
    }
}
