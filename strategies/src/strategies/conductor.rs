use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};

use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};

use collomatique_ilp::{ConfigData, ObjectiveSense, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::{
    DefaultStrategy, SolveProgress, SolveStatus, Strategy, StrategyContext, StrategyError,
    StrategyOutcome, StrategyProgress,
};

#[derive(Debug, Clone)]
pub struct Solution<V: UsableData + Send> {
    pub config: ConfigData<V>,
    pub objective: f64,
}

#[derive(Debug, Clone)]
pub struct ConductorStatus<V: UsableData + Send> {
    pub best_solution: Option<Solution<V>>,
    pub best_bound: Option<f64>,
    pub solution_found_count: u64,
    pub finished_workers: u64,
    pub total_workers: u64,
}

#[derive(Debug, Clone)]
pub enum ConductorProgress<V: UsableData + Send> {
    Conductor(ConductorStatus<V>),
    DefaultWorker(SolveProgress<V>),
}

impl<V: UsableData + Send> fmt::Display for Solution<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A `Solution` always carries a config, so incumbent presence is implicit.
        write!(f, "objective={:.4}", self.objective)
    }
}

impl<V: UsableData + Send> fmt::Display for ConductorStatus<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "workers={}/{} solutions={}",
            self.finished_workers, self.total_workers, self.solution_found_count
        )?;
        if let Some(bound) = self.best_bound {
            write!(f, " bound={bound:.4}")?;
        }
        write!(
            f,
            " incumbent={}",
            if self.best_solution.is_some() {
                "yes"
            } else {
                "no"
            }
        )
    }
}

impl<V: UsableData + Send> fmt::Display for ConductorProgress<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConductorProgress::Conductor(s) => write!(f, "[conductor] {s}"),
            ConductorProgress::DefaultWorker(p) => write!(f, "[default worker] {p}"),
        }
    }
}

enum WorkerTag {
    Default,
}

pub struct ConductorStrategy {
    pub default_worker: bool,
}

impl Default for ConductorStrategy {
    fn default() -> Self {
        Self {
            default_worker: true,
        }
    }
}

impl ConductorStrategy {
    pub fn has_workers(&self) -> bool {
        self.default_worker
    }

    fn default_status<V: UsableData + Send>(&self) -> ConductorStatus<V> {
        let total_workers = u64::from(self.default_worker);
        ConductorStatus {
            best_solution: None,
            best_bound: None,
            solution_found_count: 0,
            finished_workers: 0,
            total_workers,
        }
    }
}

pub fn update_best_solution<V: UsableData + Send>(
    status: &mut ConductorStatus<V>,
    new_solution: ConfigData<V>,
    new_objective: f64,
    sense: ObjectiveSense,
) {
    let dominated = status
        .best_solution
        .as_ref()
        .is_some_and(|current| match sense {
            ObjectiveSense::Minimize => new_objective >= current.objective,
            ObjectiveSense::Maximize => new_objective <= current.objective,
        });
    if !dominated {
        status.best_solution = Some(Solution {
            config: new_solution,
            objective: new_objective,
        });
    }
}

pub fn update_best_bound<V: UsableData + Send>(
    status: &mut ConductorStatus<V>,
    new_bound: f64,
    sense: ObjectiveSense,
) {
    let dominated = status.best_bound.is_some_and(|current| match sense {
        ObjectiveSense::Minimize => new_bound <= current,
        ObjectiveSense::Maximize => new_bound >= current,
    });
    if !dominated {
        status.best_bound = Some(new_bound);
    }
}

struct WorkerResult<V: UsableData + Send> {
    tag: WorkerTag,
    outcome: Result<StrategyOutcome<V>, StrategyError>,
    solutions_found: u64,
}

#[async_trait]
impl Strategy for ConductorStrategy {
    type Progress<V: UsableData + Send> = ConductorProgress<V>;

    fn name(&self) -> &'static str {
        "conductor"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        if !self.has_workers() {
            return Err(StrategyError::Other("No workers selected".to_string()));
        }

        let mut status = self.default_status::<InternalVar<B, E>>();
        let sense = model.problem().get_objective().get_sense();

        // Per-worker state (defined before FuturesUnordered so borrows outlive the futures)
        let default_strategy = DefaultStrategy::default();
        let default_solutions_found = AtomicU64::new(0);

        // Launch all workers
        let mut workers: FuturesUnordered<
            Pin<Box<dyn Future<Output = WorkerResult<InternalVar<B, E>>> + Send + '_>>,
        > = FuturesUnordered::new();

        if self.default_worker {
            workers.push(Box::pin(async {
                let outcome = ctx
                    .spawn_strategy_with_echo(
                        &default_strategy,
                        model,
                        warm_start,
                        &|result: Result<SolveProgress<InternalVar<B, E>>, StrategyProgress>| {
                            match result {
                                Ok(p) => {
                                    default_solutions_found
                                        .store(p.solutions_found, Ordering::Relaxed);
                                    on_progress(ConductorProgress::DefaultWorker(p))
                                }
                                Err(_) => false,
                            }
                        },
                        &|line| format!("[default worker] {}", line),
                    )
                    .await;
                WorkerResult {
                    tag: WorkerTag::Default,
                    outcome,
                    solutions_found: default_solutions_found.load(Ordering::Relaxed),
                }
            }));
        }

        // React to workers as they finish
        while let Some(worker_result) = workers.next().await {
            let outcome = match worker_result.outcome {
                Err(e) => return Err(e),
                Ok(outcome) => outcome,
            };

            match worker_result.tag {
                WorkerTag::Default => {
                    if outcome.status == SolveStatus::Infeasible {
                        return Ok(StrategyOutcome {
                            status: SolveStatus::Infeasible,
                            objective: None,
                            best_bound: None,
                            solution: None,
                        });
                    }
                    if let (Some(sol), Some(obj)) = (outcome.solution, outcome.objective) {
                        update_best_solution(&mut status, sol, obj, sense);
                    }
                    if let Some(bound) = outcome.best_bound {
                        update_best_bound(&mut status, bound, sense);
                    }
                    status.solution_found_count += worker_result.solutions_found;
                }
            }

            status.finished_workers += 1;
        }

        Ok(StrategyOutcome {
            status: if status.best_solution.is_some() {
                SolveStatus::Optimal
            } else {
                SolveStatus::Stopped
            },
            objective: status.best_solution.as_ref().map(|s| s.objective),
            best_bound: status.best_bound,
            solution: status.best_solution.map(|s| s.config),
        })
    }
}
