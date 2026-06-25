use collomatique_rpc::{CompleteCmdMsg, EncodedMsg, InitMsg, ResultMsg};
use std::collections::HashMap;
use std::sync::Mutex;

use crate::ProcessManager;
use crate::process::{OutputData, Process, ProcessEvent, ProcessStatus, StdinWriter};
use crate::worker::{Worker, WorkerEvent, WorkerId, WorkerState};

pub struct WorkerManager {
    process_manager: ProcessManager,
    workers: HashMap<WorkerId, Worker>,
    next_worker_id: u64,
}

impl WorkerManager {
    pub fn new() -> Self {
        WorkerManager {
            process_manager: ProcessManager::new(),
            workers: HashMap::new(),
            next_worker_id: 0,
        }
    }

    pub fn process_manager(&self) -> &ProcessManager {
        &self.process_manager
    }

    pub fn process_manager_mut(&mut self) -> &mut ProcessManager {
        &mut self.process_manager
    }

    pub fn spawn_worker<F>(&mut self, init_msg: InitMsg, callback: F) -> Result<WorkerId, String>
    where
        F: Fn(WorkerEvent) + Send + 'static,
    {
        let worker_id = WorkerId(self.next_worker_id);
        self.next_worker_id += 1;

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

        let process_id =
            self.process_manager
                .spawn_pty(exe_str, &["--rpc-engine"], rpc_callback)?;

        let encoded = EncodedMsg::from(init_msg);
        self.process_manager
            .send_stdin(process_id, encoded.encode().as_bytes())
            .map_err(|e| format!("Erreur à l'envoi du message initial : {}", e))?;

        self.workers.insert(
            worker_id,
            Worker {
                process_id,
                state: WorkerState {
                    status: ProcessStatus::Running,
                    log_lines: Vec::new(),
                    errors: Vec::new(),
                },
            },
        );

        Ok(worker_id)
    }

    pub fn kill_worker(&self, id: WorkerId) -> Result<(), String> {
        let worker = self
            .workers
            .get(&id)
            .ok_or_else(|| "Worker introuvable".to_string())?;
        self.process_manager.kill(worker.process_id)
    }

    /// Remove a worker and its backing process from the manager, returning the owned
    /// [`Process`] if present.
    ///
    /// This cascades into [`ProcessManager::remove`] so neither the `workers` map nor the
    /// `processes` map leaks the entry. Returning the owned `Process` lets the caller decide
    /// whether to kill it (mid-flight) or just drop it (already finished).
    pub(crate) fn remove_worker(&mut self, id: WorkerId) -> Option<Process> {
        let worker = self.workers.remove(&id)?;
        self.process_manager.remove(worker.process_id)
    }

    pub fn send_rpc_message(&self, id: WorkerId, msg: ResultMsg) -> Result<(), String> {
        let worker = self
            .workers
            .get(&id)
            .ok_or_else(|| "Worker introuvable".to_string())?;
        let encoded = EncodedMsg::from(msg);
        self.process_manager
            .send_stdin(worker.process_id, encoded.encode().as_bytes())
    }

    pub fn get_worker_stdin(&self, id: WorkerId) -> Option<StdinWriter> {
        let worker = self.workers.get(&id)?;
        self.process_manager.get_stdin_writer(worker.process_id)
    }

    pub fn get_worker_state(&self, id: WorkerId) -> Option<&WorkerState> {
        self.workers.get(&id).map(|w| &w.state)
    }

    pub fn handle_worker_event(&mut self, id: WorkerId, event: &WorkerEvent) {
        if let Some(worker) = self.workers.get_mut(&id) {
            worker.handle_event(event);
        }
    }
}
