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
    DefaultStrategy, NoObjectiveStarterProgress, SerializableProgress, SolveProgress, SolveStatus,
    Strategy, StrategyContext, StrategyError, StrategyKind, StrategyOutcome, StrategyProgress,
    StrategyProgressData,
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
        if let Some(bound) = self.best_bound {
            write!(f, "bound={bound:.4} ")?;
        }
        write!(
            f,
            "incumbent={}",
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
        if let Some(bound) = self.best_bound {
            write!(f, "bound={bound:.4} ")?;
        }
        write!(
            f,
            "incumbent={}",
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

/// Lock the shared status, apply `mutate`, and return a snapshot only if the best bound
/// or the best objective actually improved. Centralizes the "emit a Conductor update only
/// when something changed" rule shared by progress folding and result folding.
fn emit_if_changed<V: UsableData + Send>(
    status: &Mutex<ConductorStatus<V>>,
    mutate: impl FnOnce(&mut ConductorStatus<V>),
) -> Option<ConductorStatus<V>> {
    let mut st = status.lock().expect("conductor status mutex");
    let before_bound = st.best_bound;
    let before_obj = st.best_solution.as_ref().map(|s| s.objective);
    mutate(&mut st);
    let changed = st.best_bound != before_bound
        || st.best_solution.as_ref().map(|s| s.objective) != before_obj;
    changed.then(|| st.clone())
}

/// Route a worker's progress update and, when meaningful for that strategy kind, fold it
/// into the conductor's global knowledge and emit a `Conductor` update.
///
/// The `Worker` update is *always* routed. The global status is *always* folded (even when
/// the worker route asked to stop), but `on_progress` is never called a second time once it
/// has returned `false`.
fn report_worker_progress<V, P>(
    worker_num: u32,
    progress: P,
    status: &Mutex<ConductorStatus<V>>,
    sense: ObjectiveSense,
    on_progress: &(dyn Fn(ConductorProgress<V>) -> bool + Send + Sync),
) -> bool
where
    V: UsableData + Send,
    P: Into<StrategyProgress<V>>,
{
    let sp: StrategyProgress<V> = progress.into();

    // Always route the raw worker update upstairs.
    let cont = on_progress(ConductorProgress::Worker {
        worker_num,
        progress: Box::new(sp.clone()),
    });

    // Always fold into the global knowledge, even if the worker route asked to stop.
    let snapshot = match &sp {
        // Default solve progress contributes its bound and (when present) its incumbent.
        StrategyProgress::Default(p)
        | StrategyProgress::NoObjectiveStarter(NoObjectiveStarterProgress::Default(p)) => {
            emit_if_changed(status, |st| {
                update_best_bound(st, p.best_bound, sense);
                if let Some(incumbent) = p.incumbent.clone() {
                    update_best_solution(st, incumbent, p.best_obj, sense);
                }
            })
        }
        // A hint carries a feasible solution (with its objective) but no meaningful bound.
        StrategyProgress::NoObjectiveStarter(NoObjectiveStarterProgress::HintFound {
            config,
            objective,
        }) => emit_if_changed(status, |st| {
            update_best_solution(st, config.clone(), *objective, sense);
        }),
        // A sub-conductor reports its own aggregated knowledge; trust both its solution and
        // its bound.
        StrategyProgress::Conductor(ConductorProgress::Conductor(sub)) => {
            emit_if_changed(status, |st| {
                if let Some(sol) = &sub.best_solution {
                    update_best_solution(st, sol.config.clone(), sol.objective, sense);
                }
                if let Some(bound) = sub.best_bound {
                    update_best_bound(st, bound, sense);
                }
            })
        }
        // Non-contributing progress: still routed above, but nothing to fold.
        StrategyProgress::NoObjective(_)
        | StrategyProgress::NoObjectiveStarter(NoObjectiveStarterProgress::Starter(_))
        | StrategyProgress::Conductor(
            ConductorProgress::Worker { .. } | ConductorProgress::WorkerAssigned { .. },
        ) => None,
    };

    // Don't call on_progress again if the worker route already asked to stop.
    if !cont {
        return false;
    }
    match snapshot {
        Some(s) => on_progress(ConductorProgress::Conductor(s)),
        None => true,
    }
}

/// How the conductor should treat a finished worker's outcome.
enum WorkerResolution<V: UsableData + Send> {
    /// A final answer: close the debate and return this outcome.
    Definitive(StrategyOutcome<V>),
    /// An update to the global knowledge (already folded and emitted): keep going.
    Update,
}

/// Interpret a finished worker's outcome according to its strategy kind.
///
/// Definitiveness is decided per strategy. An `Infeasible` result is only globally
/// definitive for strategies that solve the *complete* problem; the per-strategy structure
/// leaves room for future strategies that solve a stricter sub-problem to treat their own
/// infeasibility as non-definitive.
fn resolve_worker_outcome<V: UsableData + Send>(
    kind: &StrategyKind,
    outcome: StrategyOutcome<V>,
    status: &Mutex<ConductorStatus<V>>,
    sense: ObjectiveSense,
    on_progress: &(dyn Fn(ConductorProgress<V>) -> bool + Send + Sync),
) -> WorkerResolution<V> {
    match kind {
        // These strategies solve the real problem; their outcome is the answer we want.
        StrategyKind::Default(_)
        | StrategyKind::NoObjectiveStarter(_)
        | StrategyKind::Conductor(_) => WorkerResolution::Definitive(outcome),
        // NoObjective solves the complete problem, so infeasibility is globally definitive;
        // but a feasible result is not objective-optimal, so it is only an update.
        StrategyKind::NoObjective(_) => {
            if outcome.status == SolveStatus::Infeasible {
                return WorkerResolution::Definitive(outcome);
            }
            if let (Some(sol), Some(obj)) = (outcome.solution, outcome.objective) {
                let snapshot = emit_if_changed(status, |st| {
                    update_best_solution(st, sol, obj, sense);
                });
                if let Some(s) = snapshot {
                    on_progress(ConductorProgress::Conductor(s));
                }
            }
            WorkerResolution::Update
        }
    }
}

/// Terminate any in-flight workers once a definitive result has been reached.
///
/// No-op for now: returning from `run_with_callback` drops the `FuturesUnordered`, which
/// drops the worker futures. A later design will use this to actively kill worker
/// subprocesses instead of waiting for drop.
fn kill_workers() {}

struct WorkerResult<V: UsableData + Send> {
    worker_num: u32,
    kind: StrategyKind,
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
                        report_worker_progress(DEFAULT_WORKER_NUM, p, &status, sense, on_progress)
                    },
                    &|line| Some(format!("[worker {DEFAULT_WORKER_NUM}] {}", line)),
                )
                .await;
            WorkerResult {
                worker_num: DEFAULT_WORKER_NUM,
                kind: StrategyKind::Default(default_strategy.clone()),
                outcome,
            }
        }));

        // React to workers as they finish.
        while let Some(worker_result) = workers.next().await {
            let outcome = worker_result.outcome?;

            // The worker is done; report it as idle.
            on_progress(ConductorProgress::WorkerAssigned {
                worker_num: worker_result.worker_num,
                strategy: None,
            });

            match resolve_worker_outcome(&worker_result.kind, outcome, &status, sense, on_progress)
            {
                WorkerResolution::Definitive(outcome) => {
                    kill_workers();
                    return Ok(outcome);
                }
                WorkerResolution::Update => {}
            }
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
    use crate::{NoObjectiveStarterStrategy, NoObjectiveStrategy};

    #[test]
    fn conductor_progress_data_round_trips_via_json() {
        let progress = ConductorProgressData::Conductor(ConductorStatusData {
            best_solution: Some(SolutionData {
                config: vec![1.0, 0.0, 1.0],
                objective: 3.5,
            }),
            best_bound: Some(2.0),
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

    /// Tag the kind of each emitted progress so tests can assert routing without caring
    /// about payloads.
    fn tag(p: &ConductorProgress<usize>) -> &'static str {
        match p {
            ConductorProgress::Conductor(_) => "conductor",
            ConductorProgress::Worker { .. } => "worker",
            ConductorProgress::WorkerAssigned { .. } => "assigned",
        }
    }

    fn empty_status() -> Mutex<ConductorStatus<usize>> {
        Mutex::new(ConductorStatus {
            best_solution: None,
            best_bound: None,
        })
    }

    fn config(values: &[(usize, f64)]) -> ConfigData<usize> {
        ConfigData::from(values.to_vec())
    }

    #[test]
    fn default_progress_with_incumbent_folds_and_emits_conductor() {
        let status = empty_status();
        let events: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
        let on_progress = |p: ConductorProgress<usize>| {
            events.lock().unwrap().push(tag(&p));
            true
        };

        let progress = SolveProgress {
            best_obj: 3.0,
            best_bound: 1.0,
            node_count: 5,
            solutions_found: 1,
            incumbent: Some(config(&[(0, 1.0)])),
        };
        let cont =
            report_worker_progress(0, progress, &status, ObjectiveSense::Minimize, &on_progress);

        assert!(cont);
        // The Worker route always fires; a Conductor update fires because bound + incumbent improved.
        assert_eq!(*events.lock().unwrap(), vec!["worker", "conductor"]);
        let st = status.lock().unwrap();
        assert_eq!(st.best_bound, Some(1.0));
        assert_eq!(st.best_solution.as_ref().unwrap().objective, 3.0);
    }

    #[test]
    fn no_objective_progress_routes_but_does_not_contribute() {
        let status = empty_status();
        let events: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
        let on_progress = |p: ConductorProgress<usize>| {
            events.lock().unwrap().push(tag(&p));
            true
        };

        let cont = report_worker_progress(
            0,
            crate::NoObjectiveProgressData::SolutionFound,
            &status,
            ObjectiveSense::Minimize,
            &on_progress,
        );

        assert!(cont);
        // Routed as a Worker update, but nothing folded -> no Conductor update.
        assert_eq!(*events.lock().unwrap(), vec!["worker"]);
        let st = status.lock().unwrap();
        assert!(st.best_solution.is_none());
        assert!(st.best_bound.is_none());
    }

    #[test]
    fn stop_request_still_folds_but_does_not_re_emit() {
        let status = empty_status();
        let calls = Mutex::new(0u32);
        let on_progress = |_p: ConductorProgress<usize>| {
            *calls.lock().unwrap() += 1;
            false // ask to stop on the first (Worker) call
        };

        let progress = SolveProgress {
            best_obj: 3.0,
            best_bound: 1.0,
            node_count: 5,
            solutions_found: 1,
            incumbent: Some(config(&[(0, 1.0)])),
        };
        let cont =
            report_worker_progress(0, progress, &status, ObjectiveSense::Minimize, &on_progress);

        assert!(!cont);
        // Only the Worker route was called; no second (Conductor) call after `false`.
        assert_eq!(*calls.lock().unwrap(), 1);
        // ...but the global status was still updated.
        let st = status.lock().unwrap();
        assert_eq!(st.best_bound, Some(1.0));
        assert!(st.best_solution.is_some());
    }

    #[test]
    fn hint_found_updates_solution_with_carried_objective_and_leaves_bound() {
        let status = empty_status();
        let on_progress = |_p: ConductorProgress<usize>| true;

        let progress = NoObjectiveStarterProgress::HintFound {
            config: config(&[(0, 1.0)]),
            objective: 2.5,
        };
        report_worker_progress(0, progress, &status, ObjectiveSense::Minimize, &on_progress);

        let st = status.lock().unwrap();
        assert_eq!(st.best_solution.as_ref().unwrap().objective, 2.5);
        assert!(st.best_bound.is_none());
    }

    fn no_objective_strategy() -> NoObjectiveStrategy {
        NoObjectiveStrategy {
            checker_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: true,
        }
    }

    fn outcome(status: SolveStatus) -> StrategyOutcome<usize> {
        StrategyOutcome {
            status,
            objective: Some(1.0),
            best_bound: Some(0.0),
            solution: Some(config(&[(0, 1.0)])),
        }
    }

    #[test]
    fn complete_problem_strategies_are_always_definitive() {
        let kinds = [
            StrategyKind::Default(DefaultStrategy::default()),
            StrategyKind::NoObjectiveStarter(NoObjectiveStarterStrategy {
                no_objective: no_objective_strategy(),
                default: DefaultStrategy::default(),
            }),
            StrategyKind::Conductor(ConductorStrategy::default()),
        ];
        for kind in &kinds {
            for status in [
                SolveStatus::Optimal,
                SolveStatus::Stopped,
                SolveStatus::Infeasible,
            ] {
                let st = empty_status();
                let res = resolve_worker_outcome(
                    kind,
                    outcome(status.clone()),
                    &st,
                    ObjectiveSense::Minimize,
                    &|_p: ConductorProgress<usize>| true,
                );
                assert!(
                    matches!(res, WorkerResolution::Definitive(_)),
                    "{kind:?} / {status:?} should be definitive",
                );
            }
        }
    }

    #[test]
    fn no_objective_outcome_is_definitive_only_when_infeasible() {
        // Infeasible -> definitive.
        let st = empty_status();
        let res = resolve_worker_outcome(
            &StrategyKind::NoObjective(no_objective_strategy()),
            outcome(SolveStatus::Infeasible),
            &st,
            ObjectiveSense::Minimize,
            &|_p: ConductorProgress<usize>| true,
        );
        assert!(matches!(res, WorkerResolution::Definitive(_)));

        // Optimal (feasible but not objective-optimal) -> update that folds + emits.
        let st = empty_status();
        let events: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
        let res = resolve_worker_outcome(
            &StrategyKind::NoObjective(no_objective_strategy()),
            outcome(SolveStatus::Optimal),
            &st,
            ObjectiveSense::Minimize,
            &|p: ConductorProgress<usize>| {
                events.lock().unwrap().push(tag(&p));
                true
            },
        );
        assert!(matches!(res, WorkerResolution::Update));
        assert_eq!(*events.lock().unwrap(), vec!["conductor"]);
        let st = st.lock().unwrap();
        assert_eq!(st.best_solution.as_ref().unwrap().objective, 1.0);
        // The NoObjective bound is never used.
        assert!(st.best_bound.is_none());
    }
}
