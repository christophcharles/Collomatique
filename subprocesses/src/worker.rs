use collomatique_rpc::{CmdMsg, InitMsg};

use crate::process::{ProcessId, ProcessStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WorkerId(pub(crate) u64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerEvent {
    LogLine(String),
    RpcCommand(Result<CmdMsg, String>),
    GracefulExit,
    ProcessExited(Option<u32>),
    Error(String),
}

pub struct WorkerState {
    pub status: ProcessStatus,
    pub log_lines: Vec<String>,
    pub errors: Vec<String>,
}

pub(crate) struct Worker {
    pub(crate) process_id: ProcessId,
    pub(crate) state: WorkerState,
    pub(crate) init_msg: InitMsg,
}

impl Worker {
    pub(crate) fn handle_event(&mut self, event: &WorkerEvent) {
        match event {
            WorkerEvent::LogLine(line) => {
                self.state.log_lines.push(line.clone());
            }
            WorkerEvent::RpcCommand(Err(e)) => {
                self.state.errors.push(e.clone());
            }
            WorkerEvent::RpcCommand(Ok(_)) => {}
            WorkerEvent::GracefulExit => {
                self.state.status = ProcessStatus::Exited(Some(0));
            }
            WorkerEvent::ProcessExited(code) => {
                self.state.status = ProcessStatus::Exited(*code);
            }
            WorkerEvent::Error(e) => {
                self.state.errors.push(e.clone());
            }
        }
    }
}
