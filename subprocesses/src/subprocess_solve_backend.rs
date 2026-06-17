use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use collomatique_strategies::{
    RawSolveOutcome, SolveBackend, SolveConfig, SolveProgress, SolveStatus, StrategyError,
};
use futures::{FutureExt, StreamExt};

use crate::ilp_solver::{IlpSolverConfig, IlpStatus, SolverSubprocess};
use crate::worker_manager::WorkerManager;

pub struct SubprocessSolveBackend {
    worker_manager: Arc<Mutex<WorkerManager>>,
    echo_solver_logs: bool,
    echo_solver_progress: bool,
}

impl SubprocessSolveBackend {
    pub fn new(
        worker_manager: Arc<Mutex<WorkerManager>>,
        echo_solver_logs: bool,
        echo_solver_progress: bool,
    ) -> Self {
        Self {
            worker_manager,
            echo_solver_logs,
            echo_solver_progress,
        }
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
        let echo_progress = self.echo_solver_progress;
        let progress_callback = move |progress: &crate::ilp_solver::IlpProgress| {
            if echo_progress {
                eprintln!(
                    "[solver subprocess] obj={:.4} bound={:.4} nodes={} solutions={}",
                    progress.best_obj,
                    progress.best_bound,
                    progress.node_count,
                    progress.solutions_found
                );
            }
            let _ = progress_tx.unbounded_send(SolveProgress {
                best_obj: progress.best_obj,
                best_bound: progress.best_bound,
                node_count: progress.node_count,
                solutions_found: progress.solutions_found,
            });
        };

        let echo_logs = self.echo_solver_logs;
        let log_callback = move |line: &str| {
            if echo_logs {
                eprintln!("[solver subprocess] {}", line.trim_end());
            }
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

        loop {
            futures::select! {
                result = result_rx => {
                    while let Some(Some(p)) = progress_rx.next().now_or_never() {
                        on_progress(p);
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
            }
        }
    }
}
