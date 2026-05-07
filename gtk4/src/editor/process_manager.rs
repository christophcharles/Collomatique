mod generic_process;
mod rpc_process;

pub use generic_process::{GenericProcessEvent, GenericProcessId, GenericProcessState};
pub use generic_process::{OutputData, OutputEntry, ProcessStatus};
pub use rpc_process::{RpcProcessEvent, RpcProcessId, RpcProcessState};

use generic_process::GenericProcess;
use rpc_process::RpcProcess;

use collomatique_rpc::{InitMsg, ResultMsg};
use std::collections::HashMap;

pub struct ProcessManager {
    rpc_processes: HashMap<RpcProcessId, RpcProcess>,
    generic_processes: HashMap<GenericProcessId, GenericProcess>,
    next_rpc_id: u64,
    next_generic_id: u64,
}

impl ProcessManager {
    pub fn new() -> Self {
        ProcessManager {
            rpc_processes: HashMap::new(),
            generic_processes: HashMap::new(),
            next_rpc_id: 0,
            next_generic_id: 0,
        }
    }

    pub fn spawn_rpc<F>(&mut self, init_msg: InitMsg, callback: F) -> Result<RpcProcessId, String>
    where
        F: Fn(RpcProcessEvent) + Send + 'static,
    {
        let id = RpcProcessId(self.next_rpc_id);
        self.next_rpc_id += 1;

        let process = RpcProcess::spawn(init_msg, callback)?;
        self.rpc_processes.insert(id, process);
        Ok(id)
    }

    pub fn spawn_generic_pty<F>(
        &mut self,
        command: &str,
        args: &[&str],
        callback: F,
    ) -> Result<GenericProcessId, String>
    where
        F: Fn(GenericProcessEvent) + Send + 'static,
    {
        let id = GenericProcessId(self.next_generic_id);
        self.next_generic_id += 1;

        let process = GenericProcess::spawn_pty(command, args, callback)?;
        self.generic_processes.insert(id, process);
        Ok(id)
    }

    pub fn spawn_generic_pipes<F>(
        &mut self,
        command: &str,
        args: &[&str],
        callback: F,
    ) -> Result<GenericProcessId, String>
    where
        F: Fn(GenericProcessEvent) + Send + Clone + 'static,
    {
        let id = GenericProcessId(self.next_generic_id);
        self.next_generic_id += 1;

        let process = GenericProcess::spawn_pipes(command, args, callback)?;
        self.generic_processes.insert(id, process);
        Ok(id)
    }

    pub fn kill_rpc(&self, id: RpcProcessId) -> Result<(), String> {
        let process = self
            .rpc_processes
            .get(&id)
            .ok_or_else(|| "Processus RPC introuvable".to_string())?;
        process.kill()
    }

    pub fn kill_generic(&self, id: GenericProcessId) -> Result<(), String> {
        let process = self
            .generic_processes
            .get(&id)
            .ok_or_else(|| "Processus introuvable".to_string())?;
        process.kill()
    }

    pub fn send_rpc_message(&self, id: RpcProcessId, msg: ResultMsg) -> Result<(), String> {
        let process = self
            .rpc_processes
            .get(&id)
            .ok_or_else(|| "Processus RPC introuvable".to_string())?;
        process.send_rpc_message(msg)
    }

    pub fn send_generic_stdin(&self, id: GenericProcessId, data: &[u8]) -> Result<(), String> {
        let process = self
            .generic_processes
            .get(&id)
            .ok_or_else(|| "Processus introuvable".to_string())?;
        process.send_stdin(data)
    }

    pub fn get_rpc_state(&self, id: RpcProcessId) -> Option<&RpcProcessState> {
        self.rpc_processes.get(&id).map(|p| p.state())
    }

    pub fn get_generic_state(&self, id: GenericProcessId) -> Option<&GenericProcessState> {
        self.generic_processes.get(&id).map(|p| p.state())
    }

    pub fn handle_rpc_event(&mut self, id: RpcProcessId, event: &RpcProcessEvent) {
        if let Some(process) = self.rpc_processes.get_mut(&id) {
            process.handle_event(event);
        }
    }

    pub fn handle_generic_event(&mut self, id: GenericProcessId, event: &GenericProcessEvent) {
        if let Some(process) = self.generic_processes.get_mut(&id) {
            process.handle_event(event);
        }
    }
}
