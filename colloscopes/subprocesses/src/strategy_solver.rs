use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::model_desc::ModelDesc;
use collomatique_ilp_modeler::{InternalVar, Model};
use collomatique_rpc::{InitMsg, NoApp, ResultMsg, SerializedStrategyRequest, StrategyMsg};
use collomatique_strategies::{
    RawSolveOutcome, SolveStatus, SpawnableStrategy, StrategyKind, StrategyOutcome,
    StrategyPayloadData, StrategyProgressData, StrategyRequest,
};

use crate::worker::{EngineExe, RpcWriter, Worker, WorkerEvent, WorkerSpawnError, send_via_rpc};

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
            StrategyStatus::Stopped(reason) => SolveStatus::Stopped(reason),
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
    Stopped(collomatique_ilp::solvers::StopReason),
    Error,
}

pub struct StrategySubprocess {
    /// Held only for its `Drop`: dropping the handle kills the worker if still running.
    ///
    /// `NoApp`: this channel runs one strategy and speaks nothing else.
    _worker: Worker<NoApp>,
    stop_flag: Arc<AtomicBool>,
    last_progress: Arc<Mutex<Option<StrategyProgressData>>>,
}

impl StrategySubprocess {
    pub fn stop(&self) {
        self.stop_flag.store(true, Ordering::Relaxed);
    }

    /// Forcefully terminate the worker. Equivalent to dropping the handle: the actual kill
    /// happens in the owned [`Worker`]'s `Drop`.
    pub fn kill(self) {
        // `self` is dropped here, killing the worker.
    }

    pub fn last_progress(&self) -> Option<StrategyProgressData> {
        self.last_progress.lock().unwrap().clone()
    }

    pub fn spawn<B, E, C, S>(
        engine: &EngineExe,
        model: &Model<B, E, C>,
        strategy: &S,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: S::Payload,
        result_callback: impl Fn(StrategyOutcome<InternalVar<B, E>>) + Send + 'static,
        progress_callback: impl Fn(Result<S::Progress, String>) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<StrategySubprocess, WorkerSpawnError>
    where
        S: SpawnableStrategy<B, E>,
        S::Progress: Send + 'static,
        B: UsableData + Send + 'static,
        E: UsableData + Send + 'static,
        C: UsableData + Send + 'static,
    {
        let (model_desc, var_order) = model.to_desc();
        let progress_var_order = var_order.clone();
        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));
        let payload_data = S::payload_into_data(payload, &var_order);
        let raw_result_callback = move |result: StrategyResult| {
            let outcome = result.into_raw_outcome().into_typed(&var_order);
            result_callback(outcome);
        };
        let strategy_kind = strategy.to_strategy_kind();
        let wrapped_progress = move |result: Result<StrategyProgressData, String>| match result {
            Ok(sp) => match S::convert_progress(sp, &progress_var_order) {
                Ok(typed) => progress_callback(Ok(typed)),
                Err(unexpected) => {
                    progress_callback(Err(format!("unexpected progress variant: {unexpected}")))
                }
            },
            Err(e) => progress_callback(Err(e)),
        };
        Self::spawn_raw(
            engine,
            model_desc,
            strategy_kind,
            raw_warm_start,
            payload_data,
            raw_result_callback,
            wrapped_progress,
            log_callback,
        )
    }

    pub fn spawn_raw(
        engine: &EngineExe,
        model_desc: ModelDesc,
        strategy: StrategyKind,
        warm_start: Option<Vec<f64>>,
        payload: StrategyPayloadData,
        result_callback: impl Fn(StrategyResult) + Send + 'static,
        progress_callback: impl Fn(Result<StrategyProgressData, String>) + Send + 'static,
        log_callback: impl Fn(&str) + Send + 'static,
    ) -> Result<StrategySubprocess, WorkerSpawnError> {
        let request = StrategyRequest {
            model_desc,
            strategy,
            warm_start,
            payload,
        };
        let serialized_str = request.serialize();
        let serialized = SerializedStrategyRequest::from(serialized_str);
        let init_msg = InitMsg::<NoApp>::RunStrategy(serialized);

        let stop_flag = Arc::new(AtomicBool::new(false));
        let last_progress: Arc<Mutex<Option<StrategyProgressData>>> = Arc::new(Mutex::new(None));
        let rpc_slot: Arc<Mutex<Option<RpcWriter>>> = Arc::new(Mutex::new(None));

        let stop_flag_cb = stop_flag.clone();
        let last_progress_cb = last_progress.clone();
        let rpc_slot_cb = rpc_slot.clone();

        let callback = move |event: WorkerEvent<NoApp>| match event {
            WorkerEvent::RpcCommand(Ok(cmd)) => match cmd {
                collomatique_rpc::CmdMsg::Strategy(StrategyMsg::Progress(data)) => {
                    let serialized_str: String = data.progress.into();
                    let progress_result = StrategyProgressData::deserialize(&serialized_str)
                        .map_err(|e| e.to_string());

                    if let Ok(ref progress) = progress_result {
                        *last_progress_cb.lock().unwrap() = Some(progress.clone());
                    }

                    progress_callback(progress_result);

                    let stopped = stop_flag_cb.load(Ordering::Relaxed);
                    let response = ResultMsg::<NoApp>::StrategyControl(!stopped);

                    let guard = rpc_slot_cb.lock().unwrap();
                    if let Some(rpc) = guard.as_ref() {
                        let _ = send_via_rpc(rpc, response);
                    }
                }
                collomatique_rpc::CmdMsg::Strategy(StrategyMsg::Result(data)) => {
                    let result = StrategyResult {
                        status: match data.status {
                            collomatique_rpc::StrategyStatus::Optimal => StrategyStatus::Optimal,
                            collomatique_rpc::StrategyStatus::Infeasible => {
                                StrategyStatus::Infeasible
                            }
                            collomatique_rpc::StrategyStatus::Stopped(reason) => {
                                StrategyStatus::Stopped(reason)
                            }
                            collomatique_rpc::StrategyStatus::Error => StrategyStatus::Error,
                        },
                        objective: data.objective.map(|v| v.into_inner()),
                        best_bound: data.best_bound.map(|v| v.into_inner()),
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

        Ok(StrategySubprocess {
            _worker: worker,
            stop_flag,
            last_progress,
        })
    }
}
