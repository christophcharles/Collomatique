use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use collomatique_rpc::{
    EncodedMsg, IlpSolveRequest, InitMsg, ResultMsg, SerializedIlpProblem, SolverMsg,
};

use crate::process::StdinWriter;
use crate::worker::{WorkerEvent, WorkerId};
use crate::worker_manager::WorkerManager;

pub struct IlpSolverConfig {
    pub problem_desc: collomatique_ilp::ProblemDesc,
    pub warm_start: Option<Vec<f64>>,
    pub time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

#[derive(Debug, Clone)]
pub struct IlpProgress {
    pub best_obj: f64,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent_info: Option<IlpIncumbentInfo>,
    pub incumbent_solution: Option<Vec<f64>>,
}

#[derive(Debug, Clone)]
pub struct IlpIncumbentInfo {
    pub objective: f64,
    pub feasible: bool,
}

#[derive(Debug, Clone)]
pub struct IlpResult {
    pub status: IlpStatus,
    pub obj_value: Option<f64>,
    pub best_bound: Option<f64>,
    pub node_count: u64,
    pub solution: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IlpStatus {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}

pub struct SolverSubprocess {
    worker_id: WorkerId,
    stop_flag: Arc<AtomicBool>,
    last_progress: Arc<Mutex<Option<IlpProgress>>>,
}

impl SolverSubprocess {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn kill(&self, worker_manager: &WorkerManager) -> Result<(), String> {
        worker_manager.kill_worker(self.worker_id)
    }

    pub fn last_progress(&self) -> Option<IlpProgress> {
        self.last_progress.lock().unwrap().clone()
    }

    pub fn spawn(
        worker_manager: &mut WorkerManager,
        config: IlpSolverConfig,
        result_callback: impl Fn(IlpResult) + Send + 'static,
        progress_callback: impl Fn(&IlpProgress) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<SolverSubprocess, String> {
        let request = IlpSolveRequest {
            problem_desc: config.problem_desc,
            warm_start: config.warm_start,
            time_limit_seconds: config.time_limit_seconds,
            disable_logging: config.disable_logging,
        };
        let serialized = SerializedIlpProblem::from(request);
        let init_msg = InitMsg::SolveIlp(serialized);

        let stop_flag = Arc::new(AtomicBool::new(false));
        let last_progress: Arc<Mutex<Option<IlpProgress>>> = Arc::new(Mutex::new(None));
        let stdin_slot: Arc<Mutex<Option<StdinWriter>>> = Arc::new(Mutex::new(None));

        let stop_flag_cb = stop_flag.clone();
        let last_progress_cb = last_progress.clone();
        let stdin_slot_cb = stdin_slot.clone();

        let callback = move |event: WorkerEvent| match event {
            WorkerEvent::RpcCommand(Ok(cmd)) => match cmd {
                collomatique_rpc::CmdMsg::Solver(SolverMsg::Progress(data)) => {
                    let progress = IlpProgress {
                        best_obj: data.best_obj.into_inner(),
                        best_bound: data.best_bound.into_inner(),
                        node_count: data.node_count,
                        solutions_found: data.solutions_found,
                        incumbent_info: data.incumbent_info.map(|info| IlpIncumbentInfo {
                            objective: info.objective.into_inner(),
                            feasible: info.feasible,
                        }),
                        incumbent_solution: data
                            .incumbent_solution
                            .map(|s| s.into_iter().map(|v| v.into_inner()).collect()),
                    };
                    progress_callback(&progress);
                    *last_progress_cb.lock().unwrap() = Some(progress);

                    let stopped = stop_flag_cb.load(Ordering::Relaxed);
                    let response = ResultMsg::SolverControl(!stopped);

                    let guard = stdin_slot_cb.lock().unwrap();
                    if let Some(stdin) = guard.as_ref() {
                        send_via_stdin(stdin, response);
                    }
                }
                collomatique_rpc::CmdMsg::Solver(SolverMsg::Result(data)) => {
                    let result = IlpResult {
                        status: match data.status {
                            collomatique_rpc::SolverStatus::Optimal => IlpStatus::Optimal,
                            collomatique_rpc::SolverStatus::Infeasible => IlpStatus::Infeasible,
                            collomatique_rpc::SolverStatus::Stopped => IlpStatus::Stopped,
                            collomatique_rpc::SolverStatus::Error => IlpStatus::Error,
                        },
                        obj_value: data.obj_value.map(|v| v.into_inner()),
                        best_bound: data.best_bound.map(|v| v.into_inner()),
                        node_count: data.node_count,
                        solution: data
                            .solution
                            .map(|s| s.into_iter().map(|v| v.into_inner()).collect()),
                    };

                    let guard = stdin_slot_cb.lock().unwrap();
                    if let Some(stdin) = guard.as_ref() {
                        send_via_stdin(stdin, ResultMsg::Ack(None));
                    }
                    drop(guard);

                    result_callback(result);
                }
                _ => {}
            },
            WorkerEvent::LogLine(line) => {
                log_callback(&line);
            }
            _ => {}
        };

        let worker_id = worker_manager.spawn_worker(init_msg, callback)?;

        let stdin_writer = worker_manager.get_worker_stdin(worker_id);
        *stdin_slot.lock().unwrap() = stdin_writer;

        Ok(SolverSubprocess {
            worker_id,
            stop_flag,
            last_progress,
        })
    }
}

fn send_via_stdin(stdin: &StdinWriter, msg: ResultMsg) {
    let encoded = EncodedMsg::from(msg).encode();
    let mut guard = stdin.lock().unwrap();
    if let Some(writer) = guard.as_mut() {
        let _ = writer.write_all(encoded.as_bytes());
        let _ = writer.flush();
    }
}
