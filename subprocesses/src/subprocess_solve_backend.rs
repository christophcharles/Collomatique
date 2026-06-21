use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use collomatique_ilp_modeler::model_desc::ModelDesc;
use collomatique_strategies::{
    RawSolveOutcome, SolveBackend, SolveConfig, SolveProgress, SolveStatus, StrategyError,
    StrategyKind, StrategyProgress,
};
use futures::{FutureExt, StreamExt};

use crate::ilp_solver::{IlpSolverConfig, IlpStatus, SolverSubprocess};
use crate::strategy_solver::{StrategyResult, StrategySubprocess};
use crate::worker_manager::WorkerManager;

pub struct SubprocessSolveBackend {
    worker_manager: Arc<Mutex<WorkerManager>>,
}

impl SubprocessSolveBackend {
    pub fn new(worker_manager: Arc<Mutex<WorkerManager>>) -> Self {
        Self { worker_manager }
    }
}

fn convert_result(result: crate::ilp_solver::IlpResult) -> RawSolveOutcome {
    let status = match result.status {
        IlpStatus::Optimal => SolveStatus::Optimal,
        IlpStatus::Infeasible => SolveStatus::Infeasible,
        IlpStatus::Stopped => SolveStatus::Stopped,
        IlpStatus::Error => SolveStatus::Error,
    };

    RawSolveOutcome {
        status,
        objective: result.obj_value,
        best_bound: result.best_bound,
        solution: result.solution,
    }
}

#[async_trait]
impl SolveBackend for SubprocessSolveBackend {
    async fn solve_with_progress(
        &self,
        desc: &collomatique_ilp::ProblemDesc,
        opts: SolveConfig,
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
        on_echo: &(dyn Fn(String) + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError> {
        let config = IlpSolverConfig {
            problem_desc: desc.clone(),
            warm_start: opts.warm_start,
            time_limit_seconds: opts.time_limit_seconds,
            disable_logging: opts.disable_logging,
        };

        let (result_tx, result_rx) = futures::channel::oneshot::channel();
        let result_tx = Mutex::new(Some(result_tx));
        let result_callback = move |result: crate::ilp_solver::IlpResult| {
            if let Some(sender) = result_tx.lock().unwrap().take() {
                let _ = sender.send(result);
            }
        };

        let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded();
        let progress_callback = move |progress: &crate::ilp_solver::IlpProgress| {
            let _ = progress_tx.unbounded_send(SolveProgress {
                best_obj: progress.best_obj,
                best_bound: progress.best_bound,
                node_count: progress.node_count,
                solutions_found: progress.solutions_found,
            });
        };

        let (echo_tx, echo_rx) = futures::channel::mpsc::unbounded();
        let log_callback = move |line: &str| {
            let _ = echo_tx.unbounded_send(line.to_owned());
        };

        let handle = {
            let mut wm = self
                .worker_manager
                .lock()
                .map_err(|e| StrategyError::SolveError(format!("lock poisoned: {e}")))?;
            SolverSubprocess::spawn(
                &mut wm,
                config,
                result_callback,
                progress_callback,
                log_callback,
            )
            .map_err(|e| StrategyError::SolveError(format!("failed to spawn solver: {e}")))?
        };

        let mut result_rx = result_rx.fuse();
        let mut progress_rx = progress_rx.fuse();
        let mut echo_rx = echo_rx.fuse();

        loop {
            futures::select! {
                result = result_rx => {
                    while let Some(Some(p)) = progress_rx.next().now_or_never() {
                        on_progress(p);
                    }
                    while let Some(Some(line)) = echo_rx.next().now_or_never() {
                        on_echo(line);
                    }
                    let result = result.map_err(|_| {
                        StrategyError::SolveError("solver channel closed".into())
                    })?;
                    return Ok(convert_result(result));
                }
                progress = progress_rx.next() => {
                    if let Some(p) = progress {
                        if !on_progress(p) {
                            handle.stop();
                        }
                    }
                }
                echo = echo_rx.next() => {
                    if let Some(line) = echo {
                        on_echo(line);
                    }
                }
            }
        }
    }

    async fn run_strategy_subprocess(
        &self,
        model_desc: &ModelDesc,
        strategy: &StrategyKind,
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
        on_echo: &(dyn Fn(String) + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError> {
        let (result_tx, result_rx) = futures::channel::oneshot::channel();
        let result_tx = Mutex::new(Some(result_tx));
        let result_callback = move |result: StrategyResult| {
            if let Some(sender) = result_tx.lock().unwrap().take() {
                let _ = sender.send(result);
            }
        };

        let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded();
        let progress_callback = move |progress: Result<StrategyProgress, String>| {
            let _ = progress_tx.unbounded_send(progress);
        };

        let (echo_tx, echo_rx) = futures::channel::mpsc::unbounded();
        let log_callback = move |line: &str| {
            let _ = echo_tx.unbounded_send(line.to_owned());
        };

        let handle = {
            let mut wm = self
                .worker_manager
                .lock()
                .map_err(|e| StrategyError::Other(format!("lock poisoned: {e}")))?;
            StrategySubprocess::spawn_raw(
                &mut wm,
                model_desc.clone(),
                strategy.clone(),
                result_callback,
                progress_callback,
                log_callback,
            )
            .map_err(|e| {
                StrategyError::Other(format!("failed to spawn strategy subprocess: {e}"))
            })?
        };

        let mut result_rx = result_rx.fuse();
        let mut progress_rx = progress_rx.fuse();
        let mut echo_rx = echo_rx.fuse();

        loop {
            futures::select! {
                result = result_rx => {
                    while let Some(Some(p)) = progress_rx.next().now_or_never() {
                        if let Ok(progress) = p {
                            on_progress(progress);
                        }
                    }
                    while let Some(Some(line)) = echo_rx.next().now_or_never() {
                        on_echo(line);
                    }
                    let result = result.map_err(|_| {
                        StrategyError::Other("strategy subprocess channel closed".into())
                    })?;
                    return Ok(result.into_raw_outcome());
                }
                progress = progress_rx.next() => {
                    if let Some(progress_result) = progress {
                        match progress_result {
                            Ok(p) => {
                                if !on_progress(p) {
                                    handle.stop();
                                }
                            }
                            Err(e) => {
                                on_echo(format!("progress deserialization error: {e}"));
                            }
                        }
                    }
                }
                echo = echo_rx.next() => {
                    if let Some(line) = echo {
                        on_echo(line);
                    }
                }
            }
        }
    }
}
