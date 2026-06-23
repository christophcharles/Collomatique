use std::convert::Infallible;
use std::fmt;
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, ObjectiveSense, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::{
    DefaultStrategy, SerializableProgress, SolveProgress, SolveStatus, Strategy, StrategyContext,
    StrategyError, StrategyKind, StrategyOutcome, StrategyProgress, StrategyProgressData,
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
}

#[derive(Debug, Clone)]
pub enum ConductorProgress<V: UsableData + Send> {
    /// Aggregated conductor-level status, emitted whenever the best bound or best
    /// solution improves.
    Conductor(ConductorStatus<V>),
    /// A worker was (re)assigned: `Some(strategy)` when a substrategy is launched on it,
    /// `None` when the worker goes idle.
    WorkerAssigned {
        worker_num: u32,
        strategy: Option<Box<StrategyKind>>,
    },
    /// An inner progress update forwarded from a worker's substrategy.
    Worker {
        worker_num: u32,
        progress: Box<StrategyProgress<V>>,
    },
}

impl<V: UsableData + Send> fmt::Display for Solution<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A `Solution` always carries a config, so incumbent presence is implicit.
        write!(f, "objective={:.4}", self.objective)
    }
}

impl<V: UsableData + Send> fmt::Display for ConductorStatus<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "solutions={}", self.solution_found_count)?;
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
            ConductorProgress::WorkerAssigned {
                worker_num,
                strategy,
            } => match strategy {
                Some(s) => write!(f, "[worker {worker_num}] assigned: {} strategy", s.name()),
                None => write!(f, "[worker {worker_num}] idle"),
            },
            ConductorProgress::Worker {
                worker_num,
                progress,
            } => write!(f, "[worker {worker_num}] {progress}"),
        }
    }
}

/// Serializable counterpart of [`Solution<V>`]: the config is erased to a
/// column-indexed `Vec<f64>` against the model's `var_order`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolutionData {
    pub config: Vec<f64>,
    pub objective: f64,
}

/// Serializable counterpart of [`ConductorStatus<V>`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConductorStatusData {
    pub best_solution: Option<SolutionData>,
    pub best_bound: Option<f64>,
    pub solution_found_count: u64,
}

/// Serializable counterpart of [`ConductorProgress<V>`], used to carry conductor
/// progress across the subprocess boundary. The conductor's best solution lives in
/// the top-level model coordinate system, so it is preserved (erased to `Vec<f64>`);
/// reconstruct the typed form with [`ConductorProgressData::into_typed`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConductorProgressData {
    Conductor(ConductorStatusData),
    WorkerAssigned {
        worker_num: u32,
        strategy: Option<Box<StrategyKind>>,
    },
    Worker {
        worker_num: u32,
        progress: Box<StrategyProgressData>,
    },
}

impl fmt::Display for ConductorStatusData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "solutions={}", self.solution_found_count)?;
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

impl fmt::Display for ConductorProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConductorProgressData::Conductor(s) => write!(f, "[conductor] {s}"),
            ConductorProgressData::WorkerAssigned {
                worker_num,
                strategy,
            } => match strategy {
                Some(s) => write!(f, "[worker {worker_num}] assigned: {} strategy", s.name()),
                None => write!(f, "[worker {worker_num}] idle"),
            },
            ConductorProgressData::Worker {
                worker_num,
                progress,
            } => write!(f, "[worker {worker_num}] {progress}"),
        }
    }
}

impl<V: UsableData + Send> ConductorStatus<V> {
    /// Erase the typed status into its serializable form, encoding the incumbent
    /// against `var_order`.
    pub fn into_data(self, var_order: &[V]) -> ConductorStatusData {
        ConductorStatusData {
            best_solution: self.best_solution.map(|s| SolutionData {
                config: collomatique_ilp::config_data_to_hint(&s.config, var_order),
                objective: s.objective,
            }),
            best_bound: self.best_bound,
            solution_found_count: self.solution_found_count,
        }
    }
}

impl ConductorStatusData {
    /// Reconstruct the typed status, turning the raw incumbent vector back into a
    /// [`ConfigData<V>`] keyed by `var_order`.
    pub fn into_typed<V: UsableData + Send>(self, var_order: &[V]) -> ConductorStatus<V> {
        ConductorStatus {
            best_solution: self.best_solution.map(|s| Solution {
                config: collomatique_ilp::solution_to_config_data(&s.config, var_order),
                objective: s.objective,
            }),
            best_bound: self.best_bound,
            solution_found_count: self.solution_found_count,
        }
    }
}

impl<V: UsableData + Send> ConductorProgress<V> {
    /// Erase the typed progress into its serializable form.
    pub fn into_data(self, var_order: &[V]) -> ConductorProgressData {
        match self {
            ConductorProgress::Conductor(s) => {
                ConductorProgressData::Conductor(s.into_data(var_order))
            }
            ConductorProgress::WorkerAssigned {
                worker_num,
                strategy,
            } => ConductorProgressData::WorkerAssigned {
                worker_num,
                strategy,
            },
            ConductorProgress::Worker {
                worker_num,
                progress,
            } => {
                let data = SerializableProgress::into_data(progress.as_ref(), var_order)
                    .unwrap_or_else(|e: Infallible| match e {});
                ConductorProgressData::Worker {
                    worker_num,
                    progress: Box::new(data),
                }
            }
        }
    }
}

impl ConductorProgressData {
    /// Reconstruct the typed progress from the serializable form.
    pub fn into_typed<V: UsableData + Send>(self, var_order: &[V]) -> ConductorProgress<V> {
        match self {
            ConductorProgressData::Conductor(s) => {
                ConductorProgress::Conductor(s.into_typed(var_order))
            }
            ConductorProgressData::WorkerAssigned {
                worker_num,
                strategy,
            } => ConductorProgress::WorkerAssigned {
                worker_num,
                strategy,
            },
            ConductorProgressData::Worker {
                worker_num,
                progress,
            } => {
                let typed = <StrategyProgress<V> as SerializableProgress<V>>::from_data(
                    progress.as_ref(),
                    var_order,
                )
                .unwrap_or_else(|e: Infallible| match e {});
                ConductorProgress::Worker {
                    worker_num,
                    progress: Box::new(typed),
                }
            }
        }
    }
}

impl<V: UsableData + Send> SerializableProgress<V> for ConductorProgress<V> {
    type Data = ConductorProgressData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<ConductorProgressData, Infallible> {
        Ok(ConductorProgress::into_data(self.clone(), var_order))
    }
    fn from_data(data: &ConductorProgressData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(ConductorProgressData::into_typed(data.clone(), var_order))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConductorStrategy {
    pub worker_count: NonZeroU32,
}

impl Default for ConductorStrategy {
    fn default() -> Self {
        Self {
            worker_count: NonZeroU32::new(1).expect("1 is non-zero"),
        }
    }
}

impl ConductorStrategy {
    fn default_status<V: UsableData + Send>() -> ConductorStatus<V> {
        ConductorStatus {
            best_solution: None,
            best_bound: None,
            solution_found_count: 0,
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
    worker_num: u32,
    outcome: Result<StrategyOutcome<V>, StrategyError>,
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
        // Shared conductor status: both the streaming worker callback and the
        // completion handler fold improvements into it. A `Mutex` makes this safe to
        // share by reference across concurrent worker futures (generalizes to N workers).
        let status: Mutex<ConductorStatus<InternalVar<B, E>>> =
            Mutex::new(Self::default_status::<InternalVar<B, E>>());
        let sense = model.problem().get_objective().get_sense();

        // Per-worker state (defined before FuturesUnordered so borrows outlive the futures)
        let default_strategy = DefaultStrategy::default();

        // For now we run a single substrategy: the default strategy on worker 0. The
        // logic that decides how many workers to spawn (and which substrategies) lives
        // elsewhere and is not yet implemented.
        const DEFAULT_WORKER_NUM: u32 = 0;

        let mut workers: FuturesUnordered<
            Pin<Box<dyn Future<Output = WorkerResult<InternalVar<B, E>>> + Send + '_>>,
        > = FuturesUnordered::new();

        on_progress(ConductorProgress::WorkerAssigned {
            worker_num: DEFAULT_WORKER_NUM,
            strategy: Some(Box::new(StrategyKind::Default(default_strategy.clone()))),
        });

        workers.push(Box::pin(async {
            let outcome = ctx
                .spawn_strategy_with_echo(
                    &default_strategy,
                    model,
                    warm_start,
                    &|p: SolveProgress<InternalVar<B, E>>| {
                        // Forward the worker's inner update upstairs.
                        let cont = on_progress(ConductorProgress::Worker {
                            worker_num: DEFAULT_WORKER_NUM,
                            progress: Box::new(StrategyProgress::Default(p.clone())),
                        });

                        // Fold the worker's bound/incumbent into the conductor's global
                        // status, emitting a Conductor update only when something improves.
                        let improved = {
                            let mut status = status.lock().expect("conductor status mutex");
                            status.solution_found_count = p.solutions_found;
                            let before_bound = status.best_bound;
                            let before_obj = status.best_solution.as_ref().map(|s| s.objective);
                            update_best_bound(&mut status, p.best_bound, sense);
                            if let Some(incumbent) = p.incumbent.clone() {
                                update_best_solution(&mut status, incumbent, p.best_obj, sense);
                            }
                            let changed = status.best_bound != before_bound
                                || status.best_solution.as_ref().map(|s| s.objective) != before_obj;
                            changed.then(|| status.clone())
                        };
                        if let Some(snapshot) = improved {
                            on_progress(ConductorProgress::Conductor(snapshot));
                        }
                        cont
                    },
                    &|line| Some(format!("[worker {DEFAULT_WORKER_NUM}] {}", line)),
                )
                .await;
            WorkerResult {
                worker_num: DEFAULT_WORKER_NUM,
                outcome,
            }
        }));

        // React to workers as they finish
        while let Some(worker_result) = workers.next().await {
            let outcome = match worker_result.outcome {
                Err(e) => return Err(e),
                Ok(outcome) => outcome,
            };

            if outcome.status == SolveStatus::Infeasible {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Infeasible,
                    objective: None,
                    best_bound: None,
                    solution: None,
                });
            }

            {
                let mut status = status.lock().expect("conductor status mutex");
                if let (Some(sol), Some(obj)) = (outcome.solution, outcome.objective) {
                    update_best_solution(&mut status, sol, obj, sense);
                }
                if let Some(bound) = outcome.best_bound {
                    update_best_bound(&mut status, bound, sense);
                }
            }

            // The worker is done; report it as idle.
            on_progress(ConductorProgress::WorkerAssigned {
                worker_num: worker_result.worker_num,
                strategy: None,
            });
        }

        let status = status.lock().expect("conductor status mutex").clone();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conductor_progress_data_round_trips_via_json() {
        let progress = ConductorProgressData::Conductor(ConductorStatusData {
            best_solution: Some(SolutionData {
                config: vec![1.0, 0.0, 1.0],
                objective: 3.5,
            }),
            best_bound: Some(2.0),
            solution_found_count: 4,
        });

        let json = serde_json::to_string(&progress).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, progress);
    }

    #[test]
    fn conductor_worker_progress_data_round_trips_via_json() {
        use crate::{SolveProgressData, StrategyKind};

        let assigned = ConductorProgressData::WorkerAssigned {
            worker_num: 0,
            strategy: Some(Box::new(StrategyKind::Default(DefaultStrategy::default()))),
        };
        let json = serde_json::to_string(&assigned).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, assigned);

        let idle = ConductorProgressData::WorkerAssigned {
            worker_num: 0,
            strategy: None,
        };
        let json = serde_json::to_string(&idle).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, idle);

        let inner = ConductorProgressData::Worker {
            worker_num: 0,
            progress: Box::new(StrategyProgressData::Default(SolveProgressData {
                best_obj: 1.5,
                best_bound: 0.5,
                node_count: 7,
                solutions_found: 2,
                incumbent: None,
            })),
        };
        let json = serde_json::to_string(&inner).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, inner);
    }

    #[test]
    fn conductor_status_incumbent_survives_erase_and_reconstruct() {
        // The conductor's best solution lives in the top-level model space, so erasing it
        // to a Vec<f64> and reconstructing against the same var_order must recover the
        // exact config. Use a plain `usize` variable type and a fixed ordering so the test
        // is insensitive to HashMap iteration order.
        let var_order: Vec<usize> = vec![0, 1, 2];
        let raw = vec![1.0, 0.0, 1.0];
        let config = collomatique_ilp::solution_to_config_data(&raw, &var_order);

        let status: ConductorStatus<usize> = ConductorStatus {
            best_solution: Some(Solution {
                config,
                objective: 3.5,
            }),
            best_bound: Some(2.0),
            solution_found_count: 4,
        };

        let data = status.into_data(&var_order);
        assert_eq!(
            data.best_solution.as_ref().map(|s| s.config.clone()),
            Some(raw.clone())
        );

        let restored: ConductorStatus<usize> = data.into_typed(&var_order);
        let restored_raw = restored
            .best_solution
            .as_ref()
            .map(|s| collomatique_ilp::config_data_to_hint(&s.config, &var_order));
        assert_eq!(restored_raw, Some(raw));
    }
}
