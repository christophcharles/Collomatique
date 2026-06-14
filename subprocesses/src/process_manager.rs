use std::collections::HashMap;

use crate::process::{Process, ProcessEvent, ProcessId, ProcessState, StdinWriter};

pub struct ProcessManager {
    processes: HashMap<ProcessId, Process>,
    next_id: u64,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            processes: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn spawn_pty<F>(
        &mut self,
        command: &str,
        args: &[&str],
        callback: F,
    ) -> Result<ProcessId, String>
    where
        F: Fn(ProcessEvent) + Send + 'static,
    {
        let id = ProcessId(self.next_id);
        self.next_id += 1;

        let process = Process::spawn_pty(command, args, callback)?;
        self.processes.insert(id, process);
        Ok(id)
    }

    pub fn spawn_pipes<F>(
        &mut self,
        command: &str,
        args: &[&str],
        callback: F,
    ) -> Result<ProcessId, String>
    where
        F: Fn(ProcessEvent) + Send + Clone + 'static,
    {
        let id = ProcessId(self.next_id);
        self.next_id += 1;

        let process = Process::spawn_pipes(command, args, callback)?;
        self.processes.insert(id, process);
        Ok(id)
    }

    pub fn kill(&self, id: ProcessId) -> Result<(), String> {
        let process = self
            .processes
            .get(&id)
            .ok_or_else(|| "Processus introuvable".to_string())?;
        process.kill()
    }

    pub fn send_stdin(&self, id: ProcessId, data: &[u8]) -> Result<(), String> {
        let process = self
            .processes
            .get(&id)
            .ok_or_else(|| "Processus introuvable".to_string())?;
        process.send_stdin(data)
    }

    pub fn get_stdin_writer(&self, id: ProcessId) -> Option<StdinWriter> {
        self.processes.get(&id).map(|p| p.get_stdin_writer())
    }

    pub fn get_state(&self, id: ProcessId) -> Option<&ProcessState> {
        self.processes.get(&id).map(|p| p.state())
    }

    pub fn handle_event(&mut self, id: ProcessId, event: &ProcessEvent) {
        if let Some(process) = self.processes.get_mut(&id) {
            process.handle_event(event);
        }
    }
}
