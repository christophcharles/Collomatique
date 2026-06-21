use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::model_desc::ModelDesc;
use collomatique_ilp_modeler::{InternalVar, Model};
use collomatique_rpc::{EncodedMsg, InitMsg, ResultMsg, SerializedStrategyRequest, StrategyMsg};
use collomatique_strategies::{
    RawSolveOutcome, SolveStatus, SpawnableStrategy, StrategyKind, StrategyOutcome,
    StrategyProgress, StrategyRequest,
};

use crate::process::StdinWriter;
use crate::worker::{WorkerEvent, WorkerId};
use crate::worker_manager::WorkerManager;

#[derive(Debug, Clone)]
pub struct StrategyResult {
    pub status: StrategyStatus,
    pub objective: Option<f64>,
    pub best_bound: Option<f64>,
    pub solution: Option<Vec<f64>>,
}

impl StrategyResult {
    pub fn into_raw_outcome(self) -> RawSolveOutcome {
        let status = match self.status {
            StrategyStatus::Optimal => SolveStatus::Optimal,
            StrategyStatus::Infeasible => SolveStatus::Infeasible,
            StrategyStatus::Stopped => SolveStatus::Stopped,
            StrategyStatus::Error => SolveStatus::Error,
        };
        RawSolveOutcome {
            status,
            objective: self.objective,
            best_bound: self.best_bound,
            solution: self.solution,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyStatus {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}

pub struct StrategySubprocess {
    worker_id: WorkerId,
    stop_flag: Arc<AtomicBool>,
    last_progress: Arc<Mutex<Option<StrategyProgress>>>,
}

impl StrategySubprocess {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    pub fn kill(&self, worker_manager: &WorkerManager) -> Result<(), String> {
        worker_manager.kill_worker(self.worker_id)
    }

    pub fn last_progress(&self) -> Option<StrategyProgress> {
        self.last_progress.lock().unwrap().clone()
    }

    pub fn spawn<B, E, C, S: SpawnableStrategy>(
        worker_manager: &mut WorkerManager,
        model: &Model<B, E, C>,
        strategy: &S,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        result_callback: impl Fn(StrategyOutcome<InternalVar<B, E>>) + Send + 'static,
        progress_callback: impl Fn(Result<S::Progress, String>) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<StrategySubprocess, String>
    where
        S::Progress: Send + 'static,
        B: UsableData + Send + 'static,
        E: UsableData + Send + 'static,
        C: UsableData + Send + 'static,
    {
        let (model_desc, var_order) = model.to_desc();
        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));
        let raw_result_callback = move |result: StrategyResult| {
            let outcome = result.into_raw_outcome().into_typed(&var_order);
            result_callback(outcome);
        };
        let strategy_kind = strategy.to_strategy_kind();
        let wrapped_progress = move |result: Result<StrategyProgress, String>| match result {
            Ok(sp) => match S::convert_progress(sp) {
                Ok(typed) => progress_callback(Ok(typed)),
                Err(unexpected) => {
                    progress_callback(Err(format!("unexpected progress variant: {unexpected}")))
                }
            },
            Err(e) => progress_callback(Err(e)),
        };
        Self::spawn_raw(
            worker_manager,
            model_desc,
            strategy_kind,
            raw_warm_start,
            raw_result_callback,
            wrapped_progress,
            log_callback,
        )
    }

    pub fn spawn_raw(
        worker_manager: &mut WorkerManager,
        model_desc: ModelDesc,
        strategy: StrategyKind,
        warm_start: Option<Vec<f64>>,
        result_callback: impl Fn(StrategyResult) + Send + 'static,
        progress_callback: impl Fn(Result<StrategyProgress, String>) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<StrategySubprocess, String> {
        let request = StrategyRequest {
            model_desc,
            strategy,
            warm_start,
        };
        let serialized_str = request.serialize();
        let serialized = SerializedStrategyRequest::from(serialized_str);
        let init_msg = InitMsg::RunStrategy(serialized);

        let stop_flag = Arc::new(AtomicBool::new(false));
        let last_progress: Arc<Mutex<Option<StrategyProgress>>> = Arc::new(Mutex::new(None));
        let stdin_slot: Arc<Mutex<Option<StdinWriter>>> = Arc::new(Mutex::new(None));

        let stop_flag_cb = stop_flag.clone();
        let last_progress_cb = last_progress.clone();
        let stdin_slot_cb = stdin_slot.clone();

        let callback = move |event: WorkerEvent| match event {
            WorkerEvent::RpcCommand(Ok(cmd)) => match cmd {
                collomatique_rpc::CmdMsg::Strategy(StrategyMsg::Progress(data)) => {
                    let serialized_str: String = data.progress.into();
                    let progress_result =
                        StrategyProgress::deserialize(&serialized_str).map_err(|e| e.to_string());

                    if let Ok(ref progress) = progress_result {
                        *last_progress_cb.lock().unwrap() = Some(progress.clone());
                    }

                    progress_callback(progress_result);

                    let stopped = stop_flag_cb.load(Ordering::Relaxed);
                    let response = ResultMsg::StrategyControl(!stopped);

                    let guard = stdin_slot_cb.lock().unwrap();
                    if let Some(stdin) = guard.as_ref() {
                        send_via_stdin(stdin, response);
                    }
                }
                collomatique_rpc::CmdMsg::Strategy(StrategyMsg::Result(data)) => {
                    let result = StrategyResult {
                        status: match data.status {
                            collomatique_rpc::StrategyStatus::Optimal => StrategyStatus::Optimal,
                            collomatique_rpc::StrategyStatus::Infeasible => {
                                StrategyStatus::Infeasible
                            }
                            collomatique_rpc::StrategyStatus::Stopped => StrategyStatus::Stopped,
                            collomatique_rpc::StrategyStatus::Error => StrategyStatus::Error,
                        },
                        objective: data.objective.map(|v| v.into_inner()),
                        best_bound: data.best_bound.map(|v| v.into_inner()),
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

        Ok(StrategySubprocess {
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
