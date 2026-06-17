use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use collomatique_strategies::{
    RawSolveOutcome, SolveBackend, SolveConfig, SolveStatus, StrategyError,
};

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

#[async_trait]
impl SolveBackend for SubprocessSolveBackend {
    async fn solve(
        &self,
        desc: &collomatique_ilp::ProblemDesc,
        opts: SolveConfig,
    ) -> Result<RawSolveOutcome, StrategyError> {
        let config = IlpSolverConfig {
            problem_desc: desc.clone(),
            warm_start: opts.warm_start,
            time_limit_seconds: opts.time_limit_seconds,
            disable_logging: opts.disable_logging,
        };

        let (tx, rx) = futures::channel::oneshot::channel();
        let tx = Mutex::new(Some(tx));

        let result_callback = move |result: crate::ilp_solver::IlpResult| {
            if let Some(sender) = tx.lock().unwrap().take() {
                let _ = sender.send(result);
            }
        };

        let echo_progress = self.echo_solver_progress;
        let echo_logs = self.echo_solver_logs;

        {
            let mut wm = self
                .worker_manager
                .lock()
                .map_err(|e| StrategyError::SolveError(format!("lock poisoned: {e}")))?;
            SolverSubprocess::spawn(
                &mut wm,
                config,
                result_callback,
                move |progress| {
                    if echo_progress {
                        eprintln!(
                            "[solver subprocess] obj={:.4} bound={:.4} nodes={} solutions={}",
                            progress.best_obj,
                            progress.best_bound,
                            progress.node_count,
                            progress.solutions_found
                        );
                    }
                },
                move |line| {
                    if echo_logs {
                        eprintln!("[solver subprocess] {}", line.trim_end());
                    }
                },
            )
            .map_err(|e| StrategyError::SolveError(format!("failed to spawn solver: {e}")))?;
        }

        let result = rx
            .await
            .map_err(|_| StrategyError::SolveError("solver channel closed".into()))?;

        let status = match result.status {
            IlpStatus::Optimal => SolveStatus::Optimal,
            IlpStatus::Infeasible => SolveStatus::Infeasible,
            IlpStatus::Stopped => SolveStatus::Stopped,
            IlpStatus::Error => SolveStatus::Error,
        };

        Ok(RawSolveOutcome {
            status,
            objective: result.obj_value,
            best_bound: result.best_bound,
            solution: result.solution,
        })
    }
}
