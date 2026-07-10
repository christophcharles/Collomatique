use std::collections::{BTreeSet, VecDeque};
use std::convert::Infallible;
use std::fmt;
use std::future::Future;
use std::num::NonZeroU32;
use std::pin::Pin;
use std::sync::Mutex;

use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use futures::select;
use futures::stream::{FuturesUnordered, StreamExt};
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, ObjectiveSense, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

#[cfg(test)]
use crate::SolveProgress;
use crate::{
    DefaultStrategy, FindClosestStrategy, FuzzyPayload, FuzzyStrategy, IncrementalPayload,
    IncrementalPayloadData, IncrementalStrategy, NoObjectiveStarterProgress, NoObjectiveStrategy,
    SolveStatus, Strategy, StrategyContext, StrategyError, StrategyKind, StrategyOutcome,
    StrategyPayload, StrategyProgress, StrategyProgressData, VarOrderSerializable,
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
    WorkerProgress {
        worker_num: u32,
        progress: Box<StrategyProgress<V>>,
    },
    /// A line of console output from a worker's substrategy subprocess.
    WorkerEcho { worker_num: u32, echo: String },
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
            ConductorProgress::WorkerProgress {
                worker_num,
                progress,
            } => write!(f, "[worker {worker_num}] {progress}"),
            ConductorProgress::WorkerEcho { worker_num, echo } => {
                write!(f, "[worker {worker_num}] {}", trim_newline(echo))
            }
        }
    }
}

fn trim_newline(s: &str) -> &str {
    s.strip_suffix('\n')
        .map_or(s, |s| s.strip_suffix('\r').unwrap_or(s))
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
    WorkerProgress {
        worker_num: u32,
        progress: Box<StrategyProgressData>,
    },
    WorkerEcho {
        worker_num: u32,
        echo: String,
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
            ConductorProgressData::WorkerProgress {
                worker_num,
                progress,
            } => write!(f, "[worker {worker_num}] {progress}"),
            ConductorProgressData::WorkerEcho { worker_num, echo } => {
                write!(f, "[worker {worker_num}] {}", trim_newline(echo))
            }
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
            ConductorProgress::WorkerProgress {
                worker_num,
                progress,
            } => {
                let data = VarOrderSerializable::into_data(progress.as_ref(), var_order)
                    .unwrap_or_else(|e: Infallible| match e {});
                ConductorProgressData::WorkerProgress {
                    worker_num,
                    progress: Box::new(data),
                }
            }
            ConductorProgress::WorkerEcho { worker_num, echo } => {
                ConductorProgressData::WorkerEcho { worker_num, echo }
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
            ConductorProgressData::WorkerProgress {
                worker_num,
                progress,
            } => {
                let typed = <StrategyProgress<V> as VarOrderSerializable<V>>::from_data(
                    progress.as_ref(),
                    var_order,
                )
                .unwrap_or_else(|e: Infallible| match e {});
                ConductorProgress::WorkerProgress {
                    worker_num,
                    progress: Box::new(typed),
                }
            }
            ConductorProgressData::WorkerEcho { worker_num, echo } => {
                ConductorProgress::WorkerEcho { worker_num, echo }
            }
        }
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for ConductorProgress<V> {
    type Data = ConductorProgressData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<ConductorProgressData, Infallible> {
        Ok(ConductorProgress::into_data(self.clone(), var_order))
    }
    fn from_data(data: &ConductorProgressData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(ConductorProgressData::into_typed(data.clone(), var_order))
    }
}

/// Tuning knobs for the conductor's fuzzy exploration. Only meaningful when fuzzy is enabled,
/// hence carried as `ConductorStrategy::fuzzy_config: Option<FuzzyConfig>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FuzzyConfig {
    /// Gaussian per-variable perturbation strength used by the fuzzy exploration workers.
    pub fuzzy_sigma: f64,
    /// Absolute L1-distance tolerance handed to every `FindClosestStrategy` the conductor
    /// builds: the closeness repair stops at the first feasible point within this distance
    /// of the closest possible one.
    pub find_closest_tolerance: f64,
}

impl Default for FuzzyConfig {
    fn default() -> Self {
        Self {
            fuzzy_sigma: 0.2, // gives ~1.2% of variables flipped if they're all binary
            find_closest_tolerance: 10.0,
        }
    }
}

/// Tuning knobs for the conductor's incremental priming solve. Only meaningful when incremental is
/// enabled, hence carried as `ConductorStrategy::incremental_config: Option<IncrementalConfig>`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IncrementalConfig {
    /// L1 anchor weight handed to the queued `IncrementalStrategy` (see
    /// [`IncrementalStrategy::l1_weight`](crate::IncrementalStrategy)).
    pub l1_weight: f64,
    /// Absolute epoch-gap tolerance handed to the queued `IncrementalStrategy` (see
    /// [`IncrementalStrategy::distance_tolerance`](crate::IncrementalStrategy)).
    pub distance_tolerance: f64,
    /// Per-epoch solve time limit handed to the queued `IncrementalStrategy` (see
    /// [`IncrementalStrategy::epoch_time_limit`](crate::IncrementalStrategy)).
    /// [`TimeLimit::none()`](collomatique_time::TimeLimit::none) leaves each epoch unbounded.
    /// Does not affect the final reconstruction solve.
    pub epoch_time_limit: collomatique_time::TimeLimit,
}

impl Default for IncrementalConfig {
    fn default() -> Self {
        // Match IncrementalStrategy's own defaults.
        Self {
            l1_weight: 1000.0,
            distance_tolerance: 5.0,
            epoch_time_limit: collomatique_time::TimeLimit::none(),
        }
    }
}

/// A misconfiguration the conductor can detect before running. Surfaced via
/// [`ConductorStrategy::warnings`] so a UI can flag setups that waste work or never finish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ConductorWarning {
    /// No substrategy is enabled at all: nothing would run.
    NoStrategyEnabled,
    /// A feasible search runs (warm-start or incremental) but neither the default branch-and-bound
    /// nor fuzzy is enabled: solutions are found but never optimised.
    NoOptimizing,
    /// Fuzzy is enabled but nothing produces an initial incumbent (no default, warm-start, or
    /// incremental) to seed it: fuzzy never fires and the conductor exits immediately.
    NoSeed,
    /// Fuzzy is enabled alongside the default worker but there is only one slot, which the default
    /// worker occupies: fuzzy never gets an idle slot to fill.
    StarvedFuzzy,
    /// Default is disabled while fuzzy runs off a warm-start / incremental incumbent: with no
    /// default there is never a bound to prove optimality, so fuzzy refills the workers indefinitely.
    WontFinish,
    /// Fuzzy is enabled but no initial-solution provider (warm-start or incremental) is: fuzzy can
    /// only fire once the default worker has already gone far, so the fuzzers are usually wasted.
    ColdFuzzy,
    /// Both warm-start and incremental are enabled — they play the same seeding role. Incremental
    /// usually gives a better starting point, so warm-start is redundant here (worth keeping only
    /// for a quick, lower-quality initial solution).
    RedundantWarmStart,
    /// More worker slots than available CPU cores.
    OverwhelmedCpu,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConductorStrategy {
    pub worker_count: NonZeroU32,
    /// Queue a `DefaultStrategy` (the full branch-and-bound solve). Queued last.
    pub enable_default: bool,
    /// Queue a `NoObjectiveStrategy` first, to produce a feasible warm-start incumbent.
    pub enable_warm_start: bool,
    /// Queue an `IncrementalStrategy` after the warm-start (before default) as a fast initial-solution
    /// provider, tuned by the carried config and using the per-run payload's epoch assignment (an
    /// empty assignment still yields a single-epoch priming solve). `None` disables incremental.
    pub incremental_config: Option<IncrementalConfig>,
    /// Fill otherwise-idle workers with `FuzzyStrategy` exploration around the incumbent, tuned by
    /// the carried config. `None` disables fuzzy exploration entirely.
    pub fuzzy_config: Option<FuzzyConfig>,
}

impl Default for ConductorStrategy {
    fn default() -> Self {
        Self {
            worker_count: NonZeroU32::new(1).expect("1 is non-zero"),
            enable_default: false,
            enable_warm_start: true,
            incremental_config: None,
            fuzzy_config: None,
        }
    }
}

/// Per-run payload for [`ConductorStrategy`]: the epoch assignment forwarded to the queued
/// `IncrementalStrategy` when [`ConductorStrategy::incremental_config`] is set. Always present (an
/// empty assignment is a single-epoch priming solve); ignored when incremental is disabled.
#[derive(Debug, Clone)]
pub struct ConductorPayload<V: UsableData> {
    pub incremental: IncrementalPayload<V>,
}

impl<V: UsableData> Default for ConductorPayload<V> {
    fn default() -> Self {
        ConductorPayload {
            incremental: IncrementalPayload::default(),
        }
    }
}

/// Serializable counterpart of [`ConductorPayload<V>`] (crosses the IPC barrier).
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ConductorPayloadData {
    pub incremental: IncrementalPayloadData,
}

impl<V: UsableData + Send> VarOrderSerializable<V> for ConductorPayload<V> {
    type Data = ConductorPayloadData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<ConductorPayloadData, Infallible> {
        Ok(ConductorPayloadData {
            incremental: self.incremental.into_data(var_order)?,
        })
    }
    fn from_data(data: &ConductorPayloadData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(ConductorPayload {
            incremental: IncrementalPayload::from_data(&data.incremental, var_order)?,
        })
    }
}

impl ConductorStrategy {
    /// Build a conductor with a sane number of worker slots: roughly half the available CPU
    /// cores (as reported by [`std::thread::available_parallelism`]), capped at 4. The
    /// conductor gains little from more than a Default worker plus a few fuzzers, and most
    /// users don't want a solve to bog down the whole machine. Falls back to a single worker
    /// when the available parallelism cannot be determined. Fuzzy exploration is enabled so the
    /// extra worker slots have something to run, and incremental is enabled as a fast initial-solution
    /// provider to seed the default/fuzzy workers early. Warm-start is left off: incremental fills the
    /// same seeding role and usually gives a better starting point, so running both would be redundant.
    pub fn with_parallelism_defaults() -> Self {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        let workers = cores.div_ceil(2).clamp(1, 4);
        let worker_count = u32::try_from(workers)
            .ok()
            .and_then(NonZeroU32::new)
            .unwrap_or(NonZeroU32::MIN);
        Self {
            worker_count,
            // The full branch-and-bound is what makes this a "complete" optimisation: it lets the
            // solve prove optimality (close the gap) rather than only hill-climb via fuzzy. Set
            // explicitly so it does not track `Default`, where it is off (a warm-start-only search).
            enable_default: true,
            // Enabled so its incumbent seeds the default/fuzzy workers early (a warm-start-only
            // search leaves them cold until the default worker has gone far).
            incremental_config: Some(IncrementalConfig::default()),
            // Off on purpose: incremental already provides the initial incumbent, and better, so a
            // warm-start on top would be redundant work.
            enable_warm_start: false,
            fuzzy_config: Some(FuzzyConfig::default()),
            ..Self::default()
        }
    }

    /// Misconfigurations detectable before running (see [`ConductorWarning`]). Returned as a set
    /// (ordered by the variants' declaration order) so a UI can flag any combination that would
    /// waste work or never terminate.
    pub fn warnings(&self) -> BTreeSet<ConductorWarning> {
        let d = self.enable_default;
        let w = self.enable_warm_start;
        let f = self.fuzzy_config.is_some();
        let i = self.incremental_config.is_some();
        // Both warm-start and incremental produce an initial incumbent for the optimisers to lean on.
        let seed = w || i;
        let wc = self.worker_count.get();

        let mut warnings = BTreeSet::new();
        if !d && !f && !seed {
            warnings.insert(ConductorWarning::NoStrategyEnabled);
        }
        if !d && !f && seed {
            warnings.insert(ConductorWarning::NoOptimizing);
        }
        if f && !d && !seed {
            warnings.insert(ConductorWarning::NoSeed);
        }
        if f && d && wc == 1 {
            warnings.insert(ConductorWarning::StarvedFuzzy);
        }
        if f && seed && !d {
            warnings.insert(ConductorWarning::WontFinish);
        }
        if f && d && !seed {
            warnings.insert(ConductorWarning::ColdFuzzy);
        }
        if w && i {
            warnings.insert(ConductorWarning::RedundantWarmStart);
        }
        let oversubscribed = std::thread::available_parallelism()
            .map(|n| wc as usize > n.get())
            .unwrap_or(false);
        if oversubscribed {
            warnings.insert(ConductorWarning::OverwhelmedCpu);
        }
        warnings
    }

    fn default_status<V: UsableData + Send>() -> ConductorStatus<V> {
        ConductorStatus {
            best_solution: None,
            best_bound: None,
        }
    }

    /// Build the incremental priming substrategy tuned by the given `IncrementalConfig`.
    fn incremental_substrategy(&self, cfg: &IncrementalConfig) -> IncrementalStrategy {
        IncrementalStrategy {
            l1_weight: cfg.l1_weight,
            distance_tolerance: cfg.distance_tolerance,
            epoch_time_limit: cfg.epoch_time_limit,
            ..IncrementalStrategy::default()
        }
    }

    /// Build a fuzzy exploration substrategy tuned by the given `FuzzyConfig`.
    fn fuzzy_substrategy(&self, cfg: &FuzzyConfig) -> FuzzyStrategy {
        FuzzyStrategy {
            sigma: cfg.fuzzy_sigma,
            // Entropy-seeded: each spawned fuzzy worker perturbs the incumbent differently.
            seed: None,
            find_closest: FindClosestStrategy {
                closeness_time_limit: collomatique_time::TimeLimit::none(),
                reconstruction_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
                distance_tolerance: cfg.find_closest_tolerance,
            },
        }
    }

    /// Build the initial worker queue from the toggles: warm-start first (so it produces an
    /// incumbent everything else can lean on), default last (so it does not monopolize the
    /// only slot when cores are scarce).
    fn seed_queue(&self, incremental: Option<&IncrementalConfig>) -> VecDeque<StrategyKind> {
        let mut queue = VecDeque::new();
        if self.enable_warm_start {
            queue.push_back(StrategyKind::NoObjective(NoObjectiveStrategy {
                checker_time_limit: collomatique_time::TimeLimit::none(),
                reconstruction_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
            }));
        }
        if let Some(cfg) = incremental {
            queue.push_back(StrategyKind::Incremental(self.incremental_substrategy(cfg)));
        }
        if self.enable_default {
            queue.push_back(StrategyKind::Default(DefaultStrategy::default()));
        }
        queue
    }
}

/// Absolute gap tolerance for declaring the incumbent optimal. Objectives here are
/// integer-valued in practice, so anything below 1 closes the gap; keep a tight epsilon.
pub const OPTIMALITY_GAP_EPS: f64 = 1e-6;

/// True once a feasible incumbent exists and the best bound has met it (gap closed), i.e. the
/// incumbent is proven optimal. Used to stop launching new fuzzy exploration work.
fn optimum_reached<V: UsableData + Send>(
    status: &ConductorStatus<V>,
    sense: ObjectiveSense,
) -> bool {
    let (Some(sol), Some(bound)) = (&status.best_solution, status.best_bound) else {
        return false;
    };
    match sense {
        ObjectiveSense::Minimize => bound + OPTIMALITY_GAP_EPS >= sol.objective,
        ObjectiveSense::Maximize => bound - OPTIMALITY_GAP_EPS <= sol.objective,
    }
}

/// Fold a freshly reported Default incumbent objective into the tracked best, per sense.
fn merge_default_obj(current: Option<f64>, candidate: f64, sense: ObjectiveSense) -> f64 {
    match current {
        None => candidate,
        Some(cur) => match sense {
            ObjectiveSense::Minimize => cur.min(candidate),
            ObjectiveSense::Maximize => cur.max(candidate),
        },
    }
}

/// Decide whether a fresh incumbent (`new_obj`) warrants killing and respawning the Default
/// worker. With no tracked Default objective yet (cold boot), any incumbent triggers the
/// one-time warm-start reboot. Otherwise it triggers only once `new_obj` has closed at least
/// half the gap between Default's objective `D` and the best bound `B` (midpoint `(D+B)/2`).
/// With no bound there is no midpoint, so we do not restart.
fn should_restart_default(
    new_obj: f64,
    default_obj: Option<f64>,
    best_bound: Option<f64>,
    sense: ObjectiveSense,
) -> bool {
    let Some(d) = default_obj else {
        return true;
    };
    let Some(b) = best_bound else {
        return false;
    };
    let midpoint = (d + b) / 2.0;
    match sense {
        ObjectiveSense::Minimize => new_obj <= midpoint,
        ObjectiveSense::Maximize => new_obj >= midpoint,
    }
}

/// Assemble the conductor's final outcome from its accumulated status: `Optimal` when a
/// feasible incumbent exists, `Stopped` otherwise. Carries the best bound and solution.
fn conductor_outcome<V: UsableData + Send>(status: &ConductorStatus<V>) -> StrategyOutcome<V> {
    StrategyOutcome {
        status: if status.best_solution.is_some() {
            SolveStatus::Optimal
        } else {
            SolveStatus::Stopped
        },
        objective: status.best_solution.as_ref().map(|s| s.objective),
        best_bound: status.best_bound,
        solution: status.best_solution.as_ref().map(|s| s.config.clone()),
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

/// Pick the warm start for a newly launched worker.
///
/// Prefer the conductor's current best solution once one has been found; until then fall
/// back to the original `warm_start` hint passed into the conductor. The incoming hint is
/// *only* a hint (possibly not a feasible solution), so it is never folded into
/// `ConductorStatus` — it serves purely as this fallback.
fn warm_start_for<V: UsableData + Send>(
    status: &Mutex<ConductorStatus<V>>,
    fallback: &Option<ConfigData<V>>,
) -> Option<ConfigData<V>> {
    status
        .lock()
        .expect("conductor status mutex")
        .best_solution
        .as_ref()
        .map(|s| s.config.clone())
        .or_else(|| fallback.clone())
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
    let cont = on_progress(ConductorProgress::WorkerProgress {
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
                if let (Some(incumbent), Some(objective)) = (p.incumbent.clone(), p.best_obj) {
                    update_best_solution(st, incumbent, objective, sense);
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
        // A FindClosest sub-solve's progress lives in the surrogate/sub-problem
        // coordinate system, so like NoObjective it carries no foldable incumbent.
        StrategyProgress::NoObjective(_)
        | StrategyProgress::FindClosest(_)
        | StrategyProgress::Fuzzy(_)
        | StrategyProgress::Incremental(_)
        | StrategyProgress::NoObjectiveStarter(NoObjectiveStarterProgress::Starter(_))
        | StrategyProgress::Conductor(
            ConductorProgress::WorkerProgress { .. }
            | ConductorProgress::WorkerAssigned { .. }
            | ConductorProgress::WorkerEcho { .. },
        ) => None,
    };

    // Don't call on_progress again if the worker route already asked to stop.
    if !cont {
        return false;
    }
    if let Some(s) = snapshot {
        if !on_progress(ConductorProgress::Conductor(s)) {
            return false;
        }
    }
    // If folding this update closed the optimality gap, ask this worker to stop: the conductor
    // will finish with the proven-optimal incumbent once control returns to the main loop
    // (which re-checks `optimum_reached` and returns `Optimal`).
    let proven = optimum_reached(&status.lock().expect("conductor status mutex"), sense);
    !proven
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
        // NoObjective and FindClosest solve the complete feasibility problem, so
        // infeasibility is globally definitive; but their feasible result optimizes a
        // surrogate (nothing / closeness to a warm start), not the real objective, so it
        // is only an update. Incremental likewise yields a complete feasible solution — with
        // a real objective value, but staggered and unproven — so it too folds as an update
        // while its infeasibility (a sub-problem of the whole) stays definitive.
        StrategyKind::NoObjective(_)
        | StrategyKind::FindClosest(_)
        | StrategyKind::Fuzzy(_)
        | StrategyKind::Incremental(_) => {
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

struct WorkerResult<V: UsableData + Send> {
    worker_num: u32,
    kind: StrategyKind,
    outcome: Result<StrategyOutcome<V>, StrategyError>,
}

/// Outcome of awaiting a worker: it either finished on its own, or was cancelled (killed)
/// by the conductor — currently only the Default worker, when it is superseded.
enum WorkerEnd<V: UsableData + Send> {
    Finished(WorkerResult<V>),
    Cancelled { worker_num: u32 },
}

/// The conductor's worker-slot pool; the slot index *is* the `worker_num`. On drop — which
/// happens on every exit from `run_with_callback`, including the early returns that force-kill
/// live workers by dropping `workers` — it emits an idle `WorkerAssigned { None }` for each
/// slot still marked busy, so the UI never shows a worker as running after it was killed.
struct WorkerSlots<'a, V: UsableData + Send> {
    busy: Vec<bool>,
    on_progress: &'a (dyn Fn(ConductorProgress<V>) -> bool + Send + Sync),
}

impl<'a, V: UsableData + Send> WorkerSlots<'a, V> {
    fn new(
        count: usize,
        on_progress: &'a (dyn Fn(ConductorProgress<V>) -> bool + Send + Sync),
    ) -> Self {
        Self {
            busy: vec![false; count],
            on_progress,
        }
    }

    /// Index of the first idle slot, if any.
    fn first_free(&self) -> Option<usize> {
        self.busy.iter().position(|busy| !*busy)
    }

    /// How many slots are currently idle.
    fn free_count(&self) -> usize {
        self.busy.iter().filter(|busy| !**busy).count()
    }

    /// Grab the first idle slot, marking it busy, and return its index; `None` if all busy.
    fn assign(&mut self) -> Option<usize> {
        let slot = self.first_free()?;
        self.busy[slot] = true;
        Some(slot)
    }

    /// Mark a busy slot idle again. Panics if it was not busy — freeing an idle slot is a bug.
    fn free(&mut self, slot: usize) {
        assert!(self.busy[slot], "freeing an already-idle slot {slot}");
        self.busy[slot] = false;
    }
}

impl<V: UsableData + Send> Drop for WorkerSlots<'_, V> {
    fn drop(&mut self) {
        for (slot, busy) in self.busy.iter().enumerate() {
            if *busy {
                (self.on_progress)(ConductorProgress::WorkerAssigned {
                    worker_num: slot as u32,
                    strategy: None,
                });
            }
        }
    }
}

/// Run a single substrategy on a worker slot and tag the outcome with its slot index.
///
/// The strategy is spawned uniformly as a [`StrategyKind`]: it is itself a `SpawnableStrategy`
/// whose progress is `StrategyProgress<V>`, which `report_worker_progress` already folds for
/// every variant — so adding new strategy kinds needs no change here.
///
/// When `cancel` is provided (the Default worker), the solve is raced against it: firing the
/// channel drops the strategy future, which RAII-kills its subprocess, and yields
/// [`WorkerEnd::Cancelled`]. `default_obj` tracks the Default worker's own best incumbent
/// objective so the conductor can decide when a far-better incumbent warrants a restart.
#[allow(clippy::too_many_arguments)]
async fn run_one_worker<'a, B, E, C>(
    ctx: &'a StrategyContext,
    model: &'a Model<B, E, C>,
    status: &'a Mutex<ConductorStatus<InternalVar<B, E>>>,
    default_obj: &'a Mutex<Option<f64>>,
    sense: ObjectiveSense,
    on_progress: &'a (dyn Fn(ConductorProgress<InternalVar<B, E>>) -> bool + Send + Sync),
    worker_num: u32,
    kind: StrategyKind,
    warm_start: Option<ConfigData<InternalVar<B, E>>>,
    payload: StrategyPayload<InternalVar<B, E>>,
    cancel: Option<oneshot::Receiver<()>>,
    wake_tx: &'a mpsc::UnboundedSender<()>,
) -> WorkerEnd<InternalVar<B, E>>
where
    B: UsableData + Send,
    E: UsableData + Send,
    C: UsableData + Send,
{
    let progress = |p: StrategyProgress<InternalVar<B, E>>| {
        // Only the Default worker emits real-coordinate Default progress in the conductor's
        // worker set, so this unambiguously tracks Default's own incumbent objective.
        if let StrategyProgress::Default(sp) = &p {
            if let Some(objective) = sp.best_obj {
                let mut d = default_obj.lock().expect("default obj mutex");
                *d = Some(merge_default_obj(*d, objective, sense));
            }
        }
        // Snapshot the incumbent objective around the fold: if this update installs a new
        // (better) incumbent, wake the scheduler so idle slots get topped up with fuzzy
        // exploration immediately, without waiting for a worker to finish. `update_best_solution`
        // only accepts non-dominated solutions, so a changed objective (including None -> Some)
        // reliably means a fresh incumbent. `report_worker_progress` never holds the status lock
        // across its return, so these two short-lived locks cannot deadlock.
        let before = status
            .lock()
            .expect("conductor status mutex")
            .best_solution
            .as_ref()
            .map(|s| s.objective);
        let cont = report_worker_progress(worker_num, p, status, sense, on_progress);
        let after = status
            .lock()
            .expect("conductor status mutex")
            .best_solution
            .as_ref()
            .map(|s| s.objective);
        if after != before {
            let _ = wake_tx.unbounded_send(());
        }
        cont
    };
    let echo = |line| {
        // Route the worker's console output as first-class progress so it can be attributed
        // to this worker across the subprocess boundary, instead of folding it into the
        // conductor's ambient echo sink.
        on_progress(ConductorProgress::WorkerEcho {
            worker_num,
            echo: line,
        });
        None
    };

    let fut = ctx.spawn_strategy_with_echo(&kind, model, warm_start, payload, &progress, &echo);
    match cancel {
        None => {
            let outcome = fut.await;
            WorkerEnd::Finished(WorkerResult {
                worker_num,
                kind: kind.clone(),
                outcome,
            })
        }
        Some(rx) => {
            futures::pin_mut!(fut);
            match futures::future::select(fut, rx).await {
                futures::future::Either::Left((outcome, _)) => WorkerEnd::Finished(WorkerResult {
                    worker_num,
                    kind: kind.clone(),
                    outcome,
                }),
                // Cancelled: `fut` is dropped at scope end → RAII-kills the subprocess.
                futures::future::Either::Right((_, _fut)) => WorkerEnd::Cancelled { worker_num },
            }
        }
    }
}

#[async_trait]
impl Strategy for ConductorStrategy {
    type Progress<V: UsableData + Send> = ConductorProgress<V>;
    type Payload<V: UsableData + Send> = ConductorPayload<V>;

    fn name(&self) -> &'static str {
        "conductor"
    }

    fn ui_name(&self) -> &'static str {
        "Coordinateur"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        conductor_payload: ConductorPayload<InternalVar<B, E>>,
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

        // A fixed-size pool of worker slots (one busy flag per slot) and a queue of
        // substrategies waiting for a free slot. The slot index *is* the `worker_num`.
        // The queue is seeded from the toggles (warm-start first, default last); idle slots
        // are later topped up with fuzzy exploration once an incumbent exists.
        let worker_count = self.worker_count.get() as usize;
        let mut slots = WorkerSlots::new(worker_count, on_progress);
        let mut queue: VecDeque<StrategyKind> = self.seed_queue(self.incremental_config.as_ref());

        // Trace of the single Default worker so it can be superseded: its slot, a `oneshot`
        // sender that cancels (kills) its future, and its own best incumbent objective — shared
        // because the Default worker's streaming callback refines it while the main loop reads it.
        let default_obj: Mutex<Option<f64>> = Mutex::new(None);
        let mut default_slot: Option<usize> = None;
        let mut default_cancel: Option<oneshot::Sender<()>> = None;

        // Wake channel: a worker that installs a new incumbent mid-run signals here so the loop
        // re-runs its top (fuzzy top-up + fill) without waiting for a worker to finish. Without
        // this, an idle slot would only ever be topped up when some *other* worker ends. Declared
        // before `workers` so that the worker futures (which borrow `&wake_tx`) drop first.
        let (wake_tx, mut wake_rx) = mpsc::unbounded::<()>();

        let mut workers: FuturesUnordered<
            Pin<Box<dyn Future<Output = WorkerEnd<InternalVar<B, E>>> + Send + '_>>,
        > = FuturesUnordered::new();

        loop {
            // Keep spare workers exploring around the incumbent: once the seeded queue is
            // drained, a warm start exists, and we are not yet at a proven optimum, fill every
            // idle slot with a fuzzy perturbation attempt. Fuzzy needs an incumbent, so this
            // can only start after warm_start/default has produced one.
            if let Some(fuzzy_cfg) = &self.fuzzy_config {
                if queue.is_empty() {
                    let free = slots.free_count();
                    let (has_incumbent, solved) = {
                        let st = status.lock().expect("conductor status mutex");
                        (st.best_solution.is_some(), optimum_reached(&st, sense))
                    };
                    if free > 0 && has_incumbent && !solved {
                        for _ in 0..free {
                            queue.push_back(StrategyKind::Fuzzy(self.fuzzy_substrategy(fuzzy_cfg)));
                        }
                    }
                }
            }

            // Fill every free slot from the queue, spawning a worker for each. A worker is
            // assigned -> emit `WorkerAssigned { Some }`.
            while let Some(slot) = slots.assign() {
                let Some(kind) = queue.pop_front() else {
                    // Assigned a slot but the queue is empty — nothing to run on it; release it.
                    slots.free(slot);
                    break;
                };
                on_progress(ConductorProgress::WorkerAssigned {
                    worker_num: slot as u32,
                    strategy: Some(Box::new(kind.clone())),
                });
                let worker_warm_start = warm_start_for(&status, &warm_start);
                // Fuzzy takes the incumbent as its target *payload* (and no warm-start hint);
                // every other kind takes an empty payload and the incumbent as a genuine
                // warm-start hint.
                let (spawn_warm_start, payload) = match &kind {
                    StrategyKind::Fuzzy(_) => {
                        let target = worker_warm_start
                            .clone()
                            .expect("fuzzy is queued only once an incumbent exists");
                        (None, StrategyPayload::Fuzzy(FuzzyPayload { target }))
                    }
                    // Incremental ignores warm_start; its start comes from the payload's epochs.
                    StrategyKind::Incremental(_) => (
                        None,
                        StrategyPayload::Incremental(conductor_payload.incremental.clone()),
                    ),
                    other => (
                        worker_warm_start.clone(),
                        other
                            .empty_payload()
                            .expect("conductor runs only empty-payload kinds besides Fuzzy"),
                    ),
                };
                // Only the Default worker is cancellable. Trace its slot + cancel handle and seed
                // its tracked objective to the warm start's objective (the anti-thrash fuse).
                let cancel = if matches!(kind, StrategyKind::Default(_)) {
                    let (tx, rx) = oneshot::channel();
                    default_slot = Some(slot);
                    default_cancel = Some(tx);
                    *default_obj.lock().expect("default obj mutex") = status
                        .lock()
                        .expect("conductor status mutex")
                        .best_solution
                        .as_ref()
                        .map(|s| s.objective);
                    Some(rx)
                } else {
                    None
                };
                workers.push(Box::pin(run_one_worker(
                    ctx,
                    model,
                    &status,
                    &default_obj,
                    sense,
                    on_progress,
                    slot as u32,
                    kind,
                    spawn_warm_start,
                    payload,
                    cancel,
                    &wake_tx,
                )));
            }

            // Nothing left running -> nothing more to schedule; we're done. (Checked before the
            // `select!` below because an empty `FuturesUnordered` reports itself terminated, so
            // `select!` would disable that arm and wait on the wake channel forever instead of
            // yielding `None`.)
            if workers.is_empty() {
                break;
            }
            // Wait for the next worker to finish OR a mid-run incumbent to wake the scheduler.
            let end = select! {
                end = workers.next() => end,
                _ = wake_rx.next() => None,
            };
            let Some(end) = end else {
                // Woken by a new incumbent (never a drained pool: the `is_empty` guard above and
                // the single-threaded await point guarantee `workers.next()` yields `Some` here).
                // Drain any coalesced wakes and re-run the loop top, which tops up idle slots with
                // fuzzy exploration around the fresh incumbent.
                while let Ok(Some(())) = wake_rx.try_next() {}
                continue;
            };
            let worker_result = match end {
                // A superseded Default was killed; its replacement is already queued at the front,
                // so just free the slot and let the loop head refill it (Default first).
                WorkerEnd::Cancelled { worker_num } => {
                    slots.free(worker_num as usize);
                    continue;
                }
                WorkerEnd::Finished(wr) => wr,
            };
            let slot = worker_result.worker_num as usize;
            let outcome = worker_result.outcome?;
            // Capture the incumbent this worker produced before `resolve_worker_outcome` consumes
            // it; used below to decide whether to restart the Default worker.
            let incumbent_obj = outcome.objective;

            let resolution =
                resolve_worker_outcome(&worker_result.kind, outcome, &status, sense, on_progress);

            // A feasible incumbent whose cost meets the best proven bound is optimal — whichever
            // workers produced the incumbent and the bound. If the gap is now closed, finish with
            // the conductor's best solution regardless of this worker's own (possibly `Stopped`)
            // status. On return, `workers`' drop kills every still-live worker subprocess and
            // `slots`' drop reports every still-busy slot (this one included) idle.
            let proven = {
                let st = status.lock().expect("conductor status mutex");
                optimum_reached(&st, sense).then(|| conductor_outcome(&st))
            };
            if let Some(outcome) = proven {
                return Ok(outcome);
            }

            match resolution {
                WorkerResolution::Definitive(outcome) => return Ok(outcome),
                WorkerResolution::Update => {}
            }

            // Not terminal: the worker's slot is free again.
            slots.free(slot);

            // A far-better incumbent arrived while Default grinds on a stale one: restart Default
            // from it. Queue the replacement first (so the freed slot is refilled with the new
            // Default), then kill the old one. Default finishing on its own is always `Definitive`
            // and returns above, so it never reaches here — only `NoObjective`/`Fuzzy` do.
            if let (Some(new_obj), Some(_)) = (incumbent_obj, default_slot) {
                let d = *default_obj.lock().expect("default obj mutex");
                let b = status.lock().expect("conductor status mutex").best_bound;
                if should_restart_default(new_obj, d, b, sense) {
                    queue.push_front(StrategyKind::Default(DefaultStrategy::default()));
                    default_slot = None;
                    if let Some(tx) = default_cancel.take() {
                        // Wake the old Default's cancel branch → drops its future → kills its
                        // subprocess. Its slot frees when the `Cancelled` result surfaces; the new
                        // Default re-establishes the trace and re-seeds `default_obj` when launched.
                        let _ = tx.send(());
                    }
                }
            }

            // Report the freed slot idle now only if nothing is queued to refill it (a live "went
            // idle while the conductor keeps working" signal, distinct from the terminal cleanup
            // that `WorkerSlots::drop` handles on return). A queued Default restart skips this.
            if queue.is_empty() {
                on_progress(ConductorProgress::WorkerAssigned {
                    worker_num: slot as u32,
                    strategy: None,
                });
            }
        }

        let status = status.lock().expect("conductor status mutex");
        Ok(conductor_outcome(&status))
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

        let inner = ConductorProgressData::WorkerProgress {
            worker_num: 0,
            progress: Box::new(StrategyProgressData::Default(SolveProgressData {
                best_obj: None,
                best_bound: 0.5,
                node_count: 7,
                solutions_found: 2,
                incumbent: None,
            })),
        };
        let json = serde_json::to_string(&inner).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, inner);

        let echo = ConductorProgressData::WorkerEcho {
            worker_num: 0,
            echo: "solving...".to_owned(),
        };
        let json = serde_json::to_string(&echo).unwrap();
        let restored: ConductorProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, echo);
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
            ConductorProgress::WorkerProgress { .. } => "worker",
            ConductorProgress::WorkerAssigned { .. } => "assigned",
            ConductorProgress::WorkerEcho { .. } => "echo",
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
            best_obj: Some(3.0),
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

    /// Mirror the wake decision the `run_one_worker` progress closure makes: fold an update and
    /// signal `wake_tx` iff it installed a new/better incumbent. Returns whether a wake was sent.
    fn fold_and_maybe_wake<P>(
        status: &Mutex<ConductorStatus<usize>>,
        wake_tx: &mpsc::UnboundedSender<()>,
        progress: P,
    ) -> bool
    where
        P: Into<StrategyProgress<usize>>,
    {
        let noop = |_p: ConductorProgress<usize>| true;
        let before = status
            .lock()
            .unwrap()
            .best_solution
            .as_ref()
            .map(|s| s.objective);
        report_worker_progress(0, progress, status, ObjectiveSense::Minimize, &noop);
        let after = status
            .lock()
            .unwrap()
            .best_solution
            .as_ref()
            .map(|s| s.objective);
        if after != before {
            let _ = wake_tx.unbounded_send(());
            true
        } else {
            false
        }
    }

    #[test]
    fn wake_fires_on_incumbent_improvement_only() {
        let status = empty_status();
        let (wake_tx, mut wake_rx) = mpsc::unbounded::<()>();

        // First incumbent (None -> Some): wakes.
        let improved = fold_and_maybe_wake(
            &status,
            &wake_tx,
            SolveProgress {
                best_obj: Some(5.0),
                best_bound: 1.0,
                node_count: 1,
                solutions_found: 1,
                incumbent: Some(config(&[(0, 1.0)])),
            },
        );
        assert!(improved);
        assert!(matches!(wake_rx.try_next(), Ok(Some(()))));

        // A dominated update (worse objective) does not improve the incumbent: no wake.
        let improved = fold_and_maybe_wake(
            &status,
            &wake_tx,
            SolveProgress {
                best_obj: Some(9.0),
                best_bound: 2.0,
                node_count: 2,
                solutions_found: 2,
                incumbent: Some(config(&[(0, 0.0)])),
            },
        );
        assert!(!improved);
        // Empty channel -> `try_next` reports the disconnected-but-empty `Err` case.
        assert!(wake_rx.try_next().is_err());

        // A better incumbent wakes again.
        let improved = fold_and_maybe_wake(
            &status,
            &wake_tx,
            SolveProgress {
                best_obj: Some(3.0),
                best_bound: 2.0,
                node_count: 3,
                solutions_found: 3,
                incumbent: Some(config(&[(0, 1.0)])),
            },
        );
        assert!(improved);
        assert!(matches!(wake_rx.try_next(), Ok(Some(()))));
    }

    #[test]
    fn drain_empties_coalesced_wakes() {
        let (wake_tx, mut wake_rx) = mpsc::unbounded::<()>();
        for _ in 0..3 {
            wake_tx.unbounded_send(()).unwrap();
        }
        let mut drained = 0;
        while let Ok(Some(())) = wake_rx.try_next() {
            drained += 1;
        }
        assert_eq!(drained, 3);
        assert!(wake_rx.try_next().is_err());
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
            best_obj: Some(3.0),
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

    #[test]
    fn warm_start_prefers_best_solution_then_falls_back() {
        // No solution yet: fall back to the original hint (or to None when there is none).
        let status = empty_status();
        let fallback = Some(config(&[(0, 1.0)]));
        assert_eq!(warm_start_for(&status, &fallback), fallback);
        assert_eq!(warm_start_for(&status, &None), None);

        // Once a solution exists, prefer it over the fallback hint.
        let best = config(&[(0, 0.0), (1, 1.0)]);
        let status = Mutex::new(ConductorStatus {
            best_solution: Some(Solution {
                config: best.clone(),
                objective: 2.0,
            }),
            best_bound: None,
        });
        assert_eq!(warm_start_for(&status, &fallback), Some(best.clone()));
        assert_eq!(warm_start_for(&status, &None), Some(best));
    }

    fn no_objective_strategy() -> NoObjectiveStrategy {
        NoObjectiveStrategy {
            checker_time_limit: collomatique_time::TimeLimit::none(),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
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

    fn status_with(best_obj: Option<f64>, best_bound: Option<f64>) -> ConductorStatus<usize> {
        ConductorStatus {
            best_solution: best_obj.map(|objective| Solution {
                config: config(&[(0, 1.0)]),
                objective,
            }),
            best_bound,
        }
    }

    #[test]
    fn optimum_reached_needs_both_solution_and_bound() {
        assert!(!optimum_reached(
            &status_with(None, None),
            ObjectiveSense::Minimize
        ));
        assert!(!optimum_reached(
            &status_with(Some(3.0), None),
            ObjectiveSense::Minimize
        ));
        assert!(!optimum_reached(
            &status_with(None, Some(3.0)),
            ObjectiveSense::Minimize
        ));
    }

    #[test]
    fn optimum_reached_closes_gap_per_sense() {
        // Minimize: the bound is a lower bound; proven optimal once it reaches the incumbent.
        assert!(!optimum_reached(
            &status_with(Some(3.0), Some(1.0)),
            ObjectiveSense::Minimize
        ));
        assert!(optimum_reached(
            &status_with(Some(3.0), Some(3.0)),
            ObjectiveSense::Minimize
        ));
        // A bound at or above the incumbent (within epsilon) counts as closed.
        assert!(optimum_reached(
            &status_with(Some(3.0), Some(3.0 - OPTIMALITY_GAP_EPS / 2.0)),
            ObjectiveSense::Minimize
        ));

        // Maximize: the bound is an upper bound; proven optimal once it drops to the incumbent.
        assert!(!optimum_reached(
            &status_with(Some(3.0), Some(5.0)),
            ObjectiveSense::Maximize
        ));
        assert!(optimum_reached(
            &status_with(Some(3.0), Some(3.0)),
            ObjectiveSense::Maximize
        ));
    }

    fn kinds(queue: &VecDeque<StrategyKind>) -> Vec<&'static str> {
        queue
            .iter()
            .map(|k| match k {
                StrategyKind::NoObjective(_) => "no_objective",
                StrategyKind::Default(_) => "default",
                StrategyKind::Incremental(_) => "incremental",
                StrategyKind::Fuzzy(_) => "fuzzy",
                _ => "other",
            })
            .collect()
    }

    fn conductor(worker_count: u32, d: bool, w: bool, f: bool) -> ConductorStrategy {
        ConductorStrategy {
            worker_count: NonZeroU32::new(worker_count).expect("non-zero worker count"),
            enable_default: d,
            enable_warm_start: w,
            incremental_config: None,
            fuzzy_config: f.then(FuzzyConfig::default),
        }
    }

    #[test]
    fn warnings_flag_no_strategy_and_no_seed() {
        // Nothing enabled at all.
        assert!(
            conductor(1, false, false, false)
                .warnings()
                .contains(&ConductorWarning::NoStrategyEnabled)
        );
        // Fuzzy only: enabled but nothing produces an incumbent to seed it.
        let w = conductor(4, false, false, true).warnings();
        assert!(w.contains(&ConductorWarning::NoSeed));
        assert!(!w.contains(&ConductorWarning::NoStrategyEnabled));
        // NoSeed and ColdFuzzy are mutually exclusive (ColdFuzzy requires default).
        assert!(!w.contains(&ConductorWarning::ColdFuzzy));
        // Nothing runs: NoOptimizing does not pile on top of NoStrategyEnabled (it needs warm-start).
        assert!(
            !conductor(1, false, false, false)
                .warnings()
                .contains(&ConductorWarning::NoOptimizing)
        );
    }

    #[test]
    fn warnings_flag_no_optimizing() {
        // Warm-start only: a feasible search runs but nothing optimises it.
        let w = conductor(1, false, true, false).warnings();
        assert!(w.contains(&ConductorWarning::NoOptimizing));
        assert!(!w.contains(&ConductorWarning::NoStrategyEnabled));
        // Default (branch-and-bound) does optimise, so NoOptimizing must not fire.
        assert!(
            !conductor(1, true, true, false)
                .warnings()
                .contains(&ConductorWarning::NoOptimizing)
        );
    }

    #[test]
    fn warnings_flag_starved_fuzzy_on_single_worker() {
        // One slot, default + fuzzy: default hogs it, fuzzy never gets an idle slot.
        assert!(
            conductor(1, true, true, true)
                .warnings()
                .contains(&ConductorWarning::StarvedFuzzy)
        );
        // Two slots leaves room for fuzzy: not starved.
        assert!(
            !conductor(2, true, true, true)
                .warnings()
                .contains(&ConductorWarning::StarvedFuzzy)
        );
    }

    #[test]
    fn warnings_flag_wont_finish_and_cold_fuzzy() {
        // Warm-start seeds fuzzy but no default => no bound => never terminates.
        assert!(
            conductor(4, false, true, true)
                .warnings()
                .contains(&ConductorWarning::WontFinish)
        );
        // Fuzzy with default but no warm start => fuzzy only fires once default has gone far.
        assert!(
            conductor(4, true, false, true)
                .warnings()
                .contains(&ConductorWarning::ColdFuzzy)
        );
    }

    #[test]
    fn warnings_treat_incremental_as_a_seed_like_warm_start() {
        // Incremental provides the initial incumbent, so fuzzy + default with incremental (no warm
        // start) is not "cold" — the same config without incremental would flag ColdFuzzy.
        let with_incremental = ConductorStrategy {
            incremental_config: Some(IncrementalConfig::default()),
            ..conductor(4, true, false, true)
        };
        assert!(
            !with_incremental
                .warnings()
                .contains(&ConductorWarning::ColdFuzzy)
        );
        // Incremental-only: something runs (so not NoStrategyEnabled) but nothing optimises it.
        let only_incremental = ConductorStrategy {
            incremental_config: Some(IncrementalConfig::default()),
            ..conductor(1, false, false, false)
        };
        let w = only_incremental.warnings();
        assert!(!w.contains(&ConductorWarning::NoStrategyEnabled));
        assert!(w.contains(&ConductorWarning::NoOptimizing));
    }

    #[test]
    fn warnings_flag_redundant_warm_start_with_incremental() {
        // Warm-start and incremental both provide the initial incumbent, so enabling both is redundant.
        let both = ConductorStrategy {
            incremental_config: Some(IncrementalConfig::default()),
            ..conductor(4, true, true, false)
        };
        assert!(
            both.warnings()
                .contains(&ConductorWarning::RedundantWarmStart)
        );
        // Either seeding provider alone is fine.
        assert!(
            !conductor(4, true, true, false)
                .warnings()
                .contains(&ConductorWarning::RedundantWarmStart)
        );
        let only_incremental = ConductorStrategy {
            incremental_config: Some(IncrementalConfig::default()),
            ..conductor(4, true, false, false)
        };
        assert!(
            !only_incremental
                .warnings()
                .contains(&ConductorWarning::RedundantWarmStart)
        );
    }

    #[test]
    fn warnings_are_clean_for_healthy_configs() {
        // Default only, or default + warm start, on a single worker: nothing to flag (barring an
        // impossibly small reported core count, which `OverwhelmedCpu` guards against separately).
        let plain = conductor(1, true, false, false).warnings();
        assert!(!plain.contains(&ConductorWarning::NoStrategyEnabled));
        assert!(!plain.contains(&ConductorWarning::StarvedFuzzy));
        assert!(!plain.contains(&ConductorWarning::WontFinish));
        assert!(!plain.contains(&ConductorWarning::ColdFuzzy));
        assert!(!plain.contains(&ConductorWarning::NoSeed));
    }

    #[test]
    fn warnings_flag_overwhelmed_cpu() {
        // Only assert when the platform reports its parallelism; otherwise the check is skipped.
        if let Ok(cores) = std::thread::available_parallelism() {
            let over = u32::try_from(cores.get() + 1).expect("core count + 1 fits in u32");
            assert!(
                conductor(over, true, true, false)
                    .warnings()
                    .contains(&ConductorWarning::OverwhelmedCpu)
            );
            // One worker never oversubscribes a machine that reports at least one core.
            assert!(
                !conductor(1, true, false, false)
                    .warnings()
                    .contains(&ConductorWarning::OverwhelmedCpu)
            );
        }
    }

    #[test]
    fn seed_queue_orders_warm_start_before_default() {
        let conductor = |warm, default| ConductorStrategy {
            enable_warm_start: warm,
            enable_default: default,
            ..ConductorStrategy::default()
        };

        assert_eq!(
            kinds(&conductor(true, true).seed_queue(None)),
            vec!["no_objective", "default"]
        );
        assert_eq!(
            kinds(&conductor(false, true).seed_queue(None)),
            vec!["default"]
        );
        assert_eq!(
            kinds(&conductor(true, false).seed_queue(None)),
            vec!["no_objective"]
        );
        assert!(conductor(false, false).seed_queue(None).is_empty());
        // Fuzzy is never seeded up front; it is only added dynamically once an incumbent exists.
        assert!(!kinds(&conductor(true, true).seed_queue(None)).contains(&"fuzzy"));
    }

    #[test]
    fn seed_queue_inserts_incremental_between_warm_start_and_default() {
        let conductor = ConductorStrategy {
            enable_warm_start: true,
            enable_default: true,
            ..ConductorStrategy::default()
        };
        assert_eq!(
            kinds(&conductor.seed_queue(Some(&IncrementalConfig::default()))),
            vec!["no_objective", "incremental", "default"]
        );
        // Incremental slots in even without a warm-start, still ahead of default.
        let no_warm = ConductorStrategy {
            enable_warm_start: false,
            enable_default: true,
            ..ConductorStrategy::default()
        };
        assert_eq!(
            kinds(&no_warm.seed_queue(Some(&IncrementalConfig::default()))),
            vec!["incremental", "default"]
        );
    }

    #[test]
    fn progress_closing_gap_asks_worker_to_stop() {
        // A worker already knows a bound of 3.0; folding an incumbent whose cost meets it closes
        // the gap, so the worker is asked to stop (callback returns false).
        let status = Mutex::new(ConductorStatus {
            best_solution: None,
            best_bound: Some(3.0),
        });
        let progress = SolveProgress {
            best_obj: Some(3.0),
            best_bound: 3.0,
            node_count: 5,
            solutions_found: 1,
            incumbent: Some(config(&[(0, 1.0)])),
        };
        let cont = report_worker_progress(
            0,
            progress,
            &status,
            ObjectiveSense::Minimize,
            &|_p: ConductorProgress<usize>| true,
        );
        assert!(!cont, "closing the gap should ask the worker to stop");
        let st = status.lock().unwrap();
        assert_eq!(st.best_solution.as_ref().unwrap().objective, 3.0);

        // Mirror case: a still-open gap keeps the worker running.
        let status = Mutex::new(ConductorStatus {
            best_solution: None,
            best_bound: Some(1.0),
        });
        let progress = SolveProgress {
            best_obj: Some(3.0),
            best_bound: 1.0,
            node_count: 5,
            solutions_found: 1,
            incumbent: Some(config(&[(0, 1.0)])),
        };
        let cont = report_worker_progress(
            0,
            progress,
            &status,
            ObjectiveSense::Minimize,
            &|_p: ConductorProgress<usize>| true,
        );
        assert!(cont, "an open gap should keep the worker running");
    }

    #[test]
    fn conductor_outcome_labels_optimal_with_incumbent() {
        let outcome = conductor_outcome(&status_with(Some(3.0), Some(3.0)));
        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(3.0));
        assert_eq!(outcome.best_bound, Some(3.0));
        assert!(outcome.solution.is_some());

        let outcome = conductor_outcome(&status_with(None, Some(2.0)));
        assert_eq!(outcome.status, SolveStatus::Stopped);
        assert_eq!(outcome.objective, None);
        assert_eq!(outcome.best_bound, Some(2.0));
        assert!(outcome.solution.is_none());
    }

    #[test]
    fn worker_slots_drop_idles_still_busy_slots() {
        let events: Mutex<Vec<(u32, bool)>> = Mutex::new(Vec::new());
        let on_progress = |p: ConductorProgress<usize>| {
            if let ConductorProgress::WorkerAssigned {
                worker_num,
                strategy,
            } = p
            {
                events
                    .lock()
                    .unwrap()
                    .push((worker_num, strategy.is_some()));
            }
            true
        };

        {
            let mut slots = WorkerSlots::new(3, &on_progress);
            // `assign` hands out ascending free slots and reports `None` once the pool is full.
            assert_eq!(slots.assign(), Some(0));
            assert_eq!(slots.assign(), Some(1));
            assert_eq!(slots.assign(), Some(2));
            assert_eq!(slots.assign(), None);
            // Slot 1 is recycled; 0 and 2 are still busy when the pool is dropped below.
            slots.free(1);
        }

        // Drop reports exactly the still-busy slots as idle (`strategy: None`).
        let mut got = events.lock().unwrap().clone();
        got.sort();
        assert_eq!(got, vec![(0, false), (2, false)]);
    }

    #[test]
    fn merge_default_obj_takes_the_better_per_sense() {
        // With nothing tracked yet, the candidate is adopted verbatim.
        assert_eq!(merge_default_obj(None, 5.0, ObjectiveSense::Minimize), 5.0);
        assert_eq!(merge_default_obj(None, 5.0, ObjectiveSense::Maximize), 5.0);
        // Minimize keeps the smaller; Maximize keeps the larger.
        assert_eq!(
            merge_default_obj(Some(5.0), 3.0, ObjectiveSense::Minimize),
            3.0
        );
        assert_eq!(
            merge_default_obj(Some(5.0), 7.0, ObjectiveSense::Minimize),
            5.0
        );
        assert_eq!(
            merge_default_obj(Some(5.0), 7.0, ObjectiveSense::Maximize),
            7.0
        );
        assert_eq!(
            merge_default_obj(Some(5.0), 3.0, ObjectiveSense::Maximize),
            5.0
        );
    }

    #[test]
    fn should_restart_default_reboots_once_when_untracked() {
        // No tracked Default objective (cold boot): any incumbent triggers the one-time reboot,
        // regardless of the bound.
        assert!(should_restart_default(
            1000.0,
            None,
            Some(1200.0),
            ObjectiveSense::Minimize
        ));
        assert!(should_restart_default(
            9999.0,
            None,
            None,
            ObjectiveSense::Minimize
        ));
    }

    #[test]
    fn should_restart_default_needs_a_bound_to_gate() {
        // Tracked objective but no bound: no midpoint, so never restart.
        assert!(!should_restart_default(
            0.0,
            Some(2800.0),
            None,
            ObjectiveSense::Minimize
        ));
    }

    #[test]
    fn should_restart_default_crosses_midpoint_per_sense() {
        // Minimize: D=2800, B=1200 -> midpoint 2000. At-or-below triggers; above does not.
        let d = Some(2800.0);
        let b = Some(1200.0);
        assert!(should_restart_default(
            2000.0,
            d,
            b,
            ObjectiveSense::Minimize
        ));
        assert!(should_restart_default(
            1000.0,
            d,
            b,
            ObjectiveSense::Minimize
        ));
        assert!(!should_restart_default(
            2001.0,
            d,
            b,
            ObjectiveSense::Minimize
        ));

        // Maximize: D=1200, B=2800 -> midpoint 2000. At-or-above triggers; below does not.
        let d = Some(1200.0);
        let b = Some(2800.0);
        assert!(should_restart_default(
            2000.0,
            d,
            b,
            ObjectiveSense::Maximize
        ));
        assert!(!should_restart_default(
            1999.0,
            d,
            b,
            ObjectiveSense::Maximize
        ));
    }
}
