use std::sync::Mutex;

use collomatique_rpc::{CmdMsg, CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg, RpcDecodeError};

use crate::process::{
    KillError, OutputData, Process, ProcessEvent, SendError, SpawnError, StdinWriter,
};

/// A runtime error surfaced by a [`Worker`]'s reader thread (as opposed to a spawn-time failure).
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum WorkerError {
    #[error("Données non-UTF-8 reçues ({0} octets)")]
    NonUtf8Output(usize),
}

/// Failure to spawn a [`Worker`] subprocess.
#[derive(Debug, thiserror::Error)]
pub enum WorkerSpawnError {
    #[error("Impossible de déterminer l'exécutable courant : {0}")]
    CurrentExe(#[source] std::io::Error),
    #[error("Le chemin de l'exécutable contient des caractères non-UTF-8")]
    NonUtf8ExePath,
    #[error(transparent)]
    Spawn(#[from] SpawnError),
    #[error("Le sous-processus s'est terminé avant l'envoi du message initial")]
    InitFinished,
    #[error("Erreur à l'envoi du message initial : {0}")]
    InitSend(#[source] std::io::Error),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    LogLine(String),
    RpcCommand(Result<CmdMsg, RpcDecodeError>),
    GracefulExit,
    ProcessExited(Option<u32>),
    Error(WorkerError),
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
    pub fn spawn<F>(init_msg: InitMsg, callback: F) -> Result<Worker, WorkerSpawnError>
    where
        F: Fn(WorkerEvent) + Send + 'static,
    {
        let exe = std::env::current_exe().map_err(WorkerSpawnError::CurrentExe)?;
        let exe_str = exe.to_str().ok_or(WorkerSpawnError::NonUtf8ExePath)?;

        let current_cmd: Mutex<String> = Mutex::new(String::new());

        let rpc_callback = move |event: ProcessEvent| match event {
            ProcessEvent::Stdout(data) => {
                let line = match data {
                    OutputData::Utf8(s) => s,
                    OutputData::Raw(bytes) => {
                        callback(WorkerEvent::Error(WorkerError::NonUtf8Output(bytes.len())));
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
        };

        let process = Process::spawn_pty(exe_str, &["--rpc-engine"], rpc_callback)?;

        let encoded = EncodedMsg::from(init_msg);
        process
            .send_stdin(encoded.encode().as_bytes())
            .map_err(|e| match e {
                SendError::Finished => WorkerSpawnError::InitFinished,
                SendError::Io(io) => WorkerSpawnError::InitSend(io),
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
    pub fn kill(&self) -> Result<(), KillError> {
        self.process.kill()
    }
}
