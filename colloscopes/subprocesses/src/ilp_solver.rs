use std::fmt;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use collomatique_rpc::{
    IlpSolveRequest, InitMsg, NoApp, ResultMsg, SerializedIlpProblem, SolverMsg,
};

use crate::worker::{EngineExe, RpcWriter, Worker, WorkerEvent, WorkerSpawnError, send_via_rpc};

pub struct IlpSolverConfig {
    pub problem_desc: collomatique_ilp::ProblemDesc,
    pub warm_start: Option<Vec<f64>>,
    pub time_limit: collomatique_time::TimeLimit,
    /// Time limit counted from the first feasible incumbent, independent of
    /// [IlpSolverConfig::time_limit]: the solve stops at whichever comes first.
    pub incumbent_time_limit: collomatique_time::TimeLimit,
    pub disable_logging: bool,
}

#[derive(Debug, Clone)]
pub struct IlpProgress {
    /// Objective of the current incumbent, or `None` if none has been found yet.
    pub best_obj: Option<f64>,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent_info: Option<IlpIncumbentInfo>,
    pub incumbent_solution: Option<Vec<f64>>,
}

impl fmt::Display for IlpProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "obj=")?;
        match self.best_obj {
            Some(obj) => write!(f, "{obj:.4}")?,
            None => write!(f, "—")?,
        }
        write!(
            f,
            " bound={:.4} nodes={} solutions={} incumbent={}",
            self.best_bound,
            self.node_count,
            self.solutions_found,
            if self.incumbent_solution.is_some() {
                "yes"
            } else {
                "no"
            },
        )
    }
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
    Stopped(collomatique_ilp::solvers::StopReason),
    Error,
}

pub struct SolverSubprocess {
    /// Held only for its `Drop`: dropping the handle kills the worker if still running.
    ///
    /// `NoApp`: this channel runs one ILP and speaks nothing else.
    _worker: Worker<NoApp>,
    stop_flag: Arc<AtomicBool>,
    last_progress: Arc<Mutex<Option<IlpProgress>>>,
}

impl SolverSubprocess {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Forcefully terminate the worker. Equivalent to dropping the handle: the kill happens
    /// in the owned [`Worker`]'s `Drop`.
    pub fn kill(self) {
        // `self` is dropped here, killing the worker.
    }

    pub fn last_progress(&self) -> Option<IlpProgress> {
        self.last_progress.lock().unwrap().clone()
    }

    pub fn spawn(
        engine: &EngineExe,
        config: IlpSolverConfig,
        result_callback: impl Fn(IlpResult) + Send + 'static,
        progress_callback: impl Fn(&IlpProgress) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<SolverSubprocess, WorkerSpawnError> {
        let request = IlpSolveRequest {
            problem_desc: config.problem_desc,
            warm_start: config.warm_start,
            time_limit: config.time_limit,
            incumbent_time_limit: config.incumbent_time_limit,
            disable_logging: config.disable_logging,
        };
        let serialized = SerializedIlpProblem::from(request);
        let init_msg = InitMsg::<NoApp>::SolveIlp(serialized);

        let stop_flag = Arc::new(AtomicBool::new(false));
        let last_progress: Arc<Mutex<Option<IlpProgress>>> = Arc::new(Mutex::new(None));
        let rpc_slot: Arc<Mutex<Option<RpcWriter>>> = Arc::new(Mutex::new(None));

        let stop_flag_cb = stop_flag.clone();
        let last_progress_cb = last_progress.clone();
        let rpc_slot_cb = rpc_slot.clone();

        let callback = move |event: WorkerEvent<NoApp>| match event {
            WorkerEvent::RpcCommand(Ok(cmd)) => match cmd {
                collomatique_rpc::CmdMsg::Solver(SolverMsg::Progress(data)) => {
                    let progress = IlpProgress {
                        best_obj: data.best_obj.map(|v| v.into_inner()),
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
                    let response = ResultMsg::<NoApp>::SolverControl(!stopped);

                    let guard = rpc_slot_cb.lock().unwrap();
                    if let Some(rpc) = guard.as_ref() {
                        let _ = send_via_rpc(rpc, response);
                    }
                }
                collomatique_rpc::CmdMsg::Solver(SolverMsg::Result(data)) => {
                    let result = IlpResult {
                        status: match data.status {
                            collomatique_rpc::SolverStatus::Optimal => IlpStatus::Optimal,
                            collomatique_rpc::SolverStatus::Infeasible => IlpStatus::Infeasible,
                            collomatique_rpc::SolverStatus::Stopped(reason) => {
                                IlpStatus::Stopped(reason)
                            }
                            collomatique_rpc::SolverStatus::Error => IlpStatus::Error,
                        },
                        obj_value: data.obj_value.map(|v| v.into_inner()),
                        best_bound: data.best_bound.map(|v| v.into_inner()),
                        node_count: data.node_count,
                        solution: data
                            .solution
                            .map(|s| s.into_iter().map(|v| v.into_inner()).collect()),
                    };

                    let guard = rpc_slot_cb.lock().unwrap();
                    if let Some(rpc) = guard.as_ref() {
                        let _ = send_via_rpc(rpc, ResultMsg::<NoApp>::Ack);
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

        let worker = Worker::spawn(engine, init_msg, callback)?;

        *rpc_slot.lock().unwrap() = Some(worker.get_rpc_writer());

        Ok(SolverSubprocess {
            _worker: worker,
            stop_flag,
            last_progress,
        })
    }
}
