mod strategies;

pub use strategies::conductor::{
    ConductorPayload, ConductorPayloadData, ConductorProgress, ConductorProgressData,
    ConductorStatus, ConductorStatusData, ConductorStrategy, ConductorWarning, DefaultConfig,
    FuzzyConfig, IncrementalConfig, OPTIMALITY_GAP_EPS, Solution, SolutionData, WarmStartConfig,
    update_best_bound, update_best_solution,
};
pub use strategies::default::{DefaultPayload, DefaultStrategy};
pub use strategies::find_closest::{
    FindClosestPayload, FindClosestPayloadData, FindClosestProgressData, FindClosestStrategy,
};
pub use strategies::fuzzy::{FuzzyPayload, FuzzyPayloadData, FuzzyProgressData, FuzzyStrategy};
pub use strategies::incremental::{
    IncrementalPayload, IncrementalPayloadData, IncrementalProgressData, IncrementalStrategy,
};
pub use strategies::no_objective::{
    NoObjectivePayload, NoObjectiveProgressData, NoObjectiveSolveProgress, NoObjectiveStrategy,
};
pub use strategies::no_objective_starter::{
    NoObjectiveStarterPayload, NoObjectiveStarterProgress, NoObjectiveStarterProgressData,
    NoObjectiveStarterStrategy,
};

use std::convert::Infallible;
use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::mat_repr::ProblemRepr;
use collomatique_ilp::{ConfigData, Problem, ProblemDesc, UsableData};

/// Re-exported so consumers can name the payload of [SolveStatus::Stopped]
/// without depending on `collomatique-ilp` directly.
pub use collomatique_ilp::solvers::StopReason;
use collomatique_ilp_modeler::model_desc::ModelDesc;
use collomatique_ilp_modeler::{InternalVar, Model};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveStatus {
    Optimal,
    Infeasible,
    Stopped(StopReason),
    Error,
}

pub struct SolveConfig {
    pub warm_start: Option<Vec<f64>>,
    pub time_limit: collomatique_time::TimeLimit,
    /// Time limit counted from the first feasible incumbent, independent of
    /// [SolveConfig::time_limit]: the solve stops at whichever comes first.
    pub incumbent_time_limit: collomatique_time::TimeLimit,
    pub disable_logging: bool,
}

pub struct SolveProblemOpts<V: UsableData> {
    pub warm_start: Option<ConfigData<V>>,
    pub time_limit: collomatique_time::TimeLimit,
    /// Time limit counted from the first feasible incumbent, independent of
    /// [SolveProblemOpts::time_limit]: the solve stops at whichever comes first.
    pub incumbent_time_limit: collomatique_time::TimeLimit,
    pub disable_logging: bool,
}

#[derive(Debug, Clone)]
pub struct RawSolveOutcome {
    pub status: SolveStatus,
    pub objective: Option<f64>,
    pub best_bound: Option<f64>,
    pub solution: Option<Vec<f64>>,
}

impl RawSolveOutcome {
    pub fn into_typed<V: UsableData>(self, var_order: &[V]) -> StrategyOutcome<V> {
        let solution = self
            .solution
            .as_ref()
            .map(|sol| collomatique_ilp::solution_to_config_data(sol, var_order));
        StrategyOutcome {
            status: self.status,
            objective: self.objective,
            best_bound: self.best_bound,
            solution,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StrategyOutcome<V: UsableData> {
    pub status: SolveStatus,
    pub objective: Option<f64>,
    pub best_bound: Option<f64>,
    pub solution: Option<ConfigData<V>>,
}

/// Raw, serializable progress emitted by the solve backend.
///
/// The incumbent's variable assignment is carried as a column-indexed `Vec<f64>`
/// (the implicit `ProblemDesc`/`ModelDesc` ordering), so the type stays serializable
/// across the IPC barrier. Use [`SolveProgressData::into_typed`] to reconstruct the
/// typed [`SolveProgress`] once a `var_order` is available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveProgressData {
    /// Objective of the current incumbent, or `None` if none has been found yet.
    pub best_obj: Option<f64>,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent: Option<Vec<f64>>,
}

impl fmt::Display for SolveProgressData {
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
            if self.incumbent.is_some() {
                "yes"
            } else {
                "no"
            },
        )
    }
}

impl SolveProgressData {
    /// Reconstruct the typed progress, turning the raw incumbent vector into a
    /// [`ConfigData`] keyed by the supplied `var_order`.
    pub fn into_typed<V: UsableData>(self, var_order: &[V]) -> SolveProgress<V> {
        SolveProgress {
            best_obj: self.best_obj,
            best_bound: self.best_bound,
            node_count: self.node_count,
            solutions_found: self.solutions_found,
            incumbent: self
                .incumbent
                .as_ref()
                .map(|sol| collomatique_ilp::solution_to_config_data(sol, var_order)),
        }
    }
}

/// Typed progress with the incumbent exposed as a [`ConfigData<V>`].
///
/// Not serializable by design: `ConfigData<V>` is only serializable when `V` is, which
/// is not guaranteed. The serializable counterpart is [`SolveProgressData`].
#[derive(Debug, Clone)]
pub struct SolveProgress<V: UsableData> {
    /// Objective of the current incumbent, or `None` if none has been found yet.
    pub best_obj: Option<f64>,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
    pub incumbent: Option<ConfigData<V>>,
}

impl<V: UsableData> SolveProgress<V> {
    /// Serialize back to the raw form, encoding the incumbent against `var_order`.
    pub fn into_data(self, var_order: &[V]) -> SolveProgressData {
        SolveProgressData {
            best_obj: self.best_obj,
            best_bound: self.best_bound,
            node_count: self.node_count,
            solutions_found: self.solutions_found,
            incumbent: self
                .incumbent
                .as_ref()
                .map(|cfg| collomatique_ilp::config_data_to_hint(cfg, var_order)),
        }
    }
}

impl<V: UsableData> fmt::Display for SolveProgress<V> {
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
            if self.incumbent.is_some() {
                "yes"
            } else {
                "no"
            },
        )
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StrategyError {
    #[error("solve error: {0}")]
    SolveError(String),
    #[error("{0}")]
    Other(String),
}

#[async_trait]
pub trait SolveBackend: Send + Sync {
    async fn solve_with_progress(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
        on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
        on_echo: &(dyn Fn(String) + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError>;

    async fn solve(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
    ) -> Result<RawSolveOutcome, StrategyError> {
        self.solve_with_progress(desc, opts, &|_| true, &|_| {})
            .await
    }

    async fn run_strategy_subprocess(
        &self,
        model_desc: &ModelDesc,
        strategy: &StrategyKind,
        warm_start: Option<Vec<f64>>,
        payload: StrategyPayloadData,
        on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
        on_echo: &(dyn Fn(String) + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError>;
}

pub struct StrategyContext {
    backend: Arc<dyn SolveBackend>,
    on_echo: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

impl StrategyContext {
    pub fn new(backend: Arc<dyn SolveBackend>) -> Self {
        Self {
            backend,
            on_echo: None,
        }
    }

    pub fn with_echo(
        backend: Arc<dyn SolveBackend>,
        on_echo: Arc<dyn Fn(String) + Send + Sync>,
    ) -> Self {
        Self {
            backend,
            on_echo: Some(on_echo),
        }
    }

    pub async fn solve(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
    ) -> Result<RawSolveOutcome, StrategyError> {
        self.backend.solve(desc, opts).await
    }

    pub async fn solve_with_progress(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
        on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError> {
        let noop_echo = |_: String| {};
        let echo_fn: &(dyn Fn(String) + Send + Sync) = match &self.on_echo {
            Some(f) => f.as_ref(),
            None => &noop_echo,
        };
        self.backend
            .solve_with_progress(desc, opts, on_progress, echo_fn)
            .await
    }

    pub async fn solve_with_progress_and_echo(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
        on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
        handle_echo: &(dyn Fn(String) -> Option<String> + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError> {
        let echo_impl: Box<dyn Fn(String) + Send + Sync + '_> = match &self.on_echo {
            Some(ctx_echo) => Box::new(move |line| {
                if let Some(out) = handle_echo(line) {
                    ctx_echo(out);
                }
            }),
            // No parent sink: still call handle_echo so its side effects/routing run.
            None => Box::new(move |line| {
                let _ = handle_echo(line);
            }),
        };
        self.backend
            .solve_with_progress(desc, opts, on_progress, &*echo_impl)
            .await
    }

    pub async fn solve_problem<V, C, P>(
        &self,
        problem: &Problem<V, C, P>,
        opts: SolveProblemOpts<V>,
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData,
        C: UsableData,
        P: ProblemRepr<V>,
    {
        self.solve_problem_with_progress(problem, opts, &|_| true)
            .await
    }

    pub async fn solve_problem_with_progress<V, C, P>(
        &self,
        problem: &Problem<V, C, P>,
        opts: SolveProblemOpts<V>,
        on_progress: &(dyn Fn(SolveProgress<V>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData,
        C: UsableData,
        P: ProblemRepr<V>,
    {
        let (desc, var_order) = problem.get_desc();

        let warm_start = opts
            .warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));

        let solve_config = SolveConfig {
            warm_start,
            time_limit: opts.time_limit,
            incumbent_time_limit: opts.incumbent_time_limit,
            disable_logging: opts.disable_logging,
        };

        let typed_on_progress =
            |data: SolveProgressData| -> bool { on_progress(data.into_typed(&var_order)) };

        let raw = self
            .solve_with_progress(&desc, solve_config, &typed_on_progress)
            .await?;

        Ok(raw.into_typed(&var_order))
    }

    pub async fn solve_problem_with_echo<V, C, P>(
        &self,
        problem: &Problem<V, C, P>,
        opts: SolveProblemOpts<V>,
        on_progress: &(dyn Fn(SolveProgress<V>) -> bool + Send + Sync),
        handle_echo: &(dyn Fn(String) -> Option<String> + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData,
        C: UsableData,
        P: ProblemRepr<V>,
    {
        let (desc, var_order) = problem.get_desc();

        let warm_start = opts
            .warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));

        let solve_config = SolveConfig {
            warm_start,
            time_limit: opts.time_limit,
            incumbent_time_limit: opts.incumbent_time_limit,
            disable_logging: opts.disable_logging,
        };

        let typed_on_progress =
            |data: SolveProgressData| -> bool { on_progress(data.into_typed(&var_order)) };

        let raw = self
            .solve_with_progress_and_echo(&desc, solve_config, &typed_on_progress, handle_echo)
            .await?;

        Ok(raw.into_typed(&var_order))
    }
}

#[async_trait]
pub trait Strategy: Send + Sync {
    type Progress<V: UsableData + Send>: Send + Sync + Clone;
    /// Per-run payload carrying data specific to *this* problem instance (as opposed to the
    /// strategy's own configuration). Empty (`*Payload`) for strategies that need none.
    type Payload<V: UsableData + Send>: Send + Sync + Clone;

    fn name(&self) -> &'static str;

    /// Human-facing French name, shown in the UI.
    fn ui_name(&self) -> &'static str;

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: Self::Payload<InternalVar<B, E>>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send;

    async fn run<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        payload: Self::Payload<InternalVar<B, E>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        self.run_with_callback(ctx, model, None, payload, &|_| true)
            .await
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyKind {
    Default(DefaultStrategy),
    NoObjective(NoObjectiveStrategy),
    NoObjectiveStarter(NoObjectiveStarterStrategy),
    FindClosest(FindClosestStrategy),
    Fuzzy(FuzzyStrategy),
    Incremental(IncrementalStrategy),
    Conductor(ConductorStrategy),
}

/// Conversion between a typed progress and its serializable (`Data`) counterpart,
/// parameterized by the model's `var_order` (used to encode/decode incumbents as
/// column-indexed `Vec<f64>`). Implemented by every progress type; [`StrategyProgress<V>`]
/// implements it by delegating to its sub-progress types.
pub trait VarOrderSerializable<V: UsableData + Send>: Sized {
    type Data: serde::Serialize + serde::de::DeserializeOwned;
    type Error;

    /// Erase the typed progress into its serializable form.
    fn into_data(&self, var_order: &[V]) -> Result<Self::Data, Self::Error>;

    /// Reconstruct the typed progress from its serializable form.
    fn from_data(data: &Self::Data, var_order: &[V]) -> Result<Self, Self::Error>;
}

/// A typed progress that is one variant of [`StrategyProgress<V>`]. Used to project the
/// typed union down to the specific progress a [`SpawnableStrategy`] expects.
pub trait StrategyProgressVariant<V: UsableData + Send>: Sized {
    /// Extract this variant from the union, handing the union back unchanged on mismatch.
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>>;
}

/// Typed union of every strategy's progress, carrying real incumbents as `ConfigData<V>`.
///
/// Not serializable by design (mirrors the per-strategy typed/erased split). The
/// serializable counterpart is [`StrategyProgressData`]; convert via the
/// [`VarOrderSerializable`] impl.
#[derive(Debug, Clone)]
pub enum StrategyProgress<V: UsableData + Send> {
    Default(SolveProgress<V>),
    NoObjective(NoObjectiveProgressData),
    NoObjectiveStarter(NoObjectiveStarterProgress<V>),
    FindClosest(FindClosestProgressData),
    Fuzzy(FuzzyProgressData),
    Incremental(IncrementalProgressData),
    Conductor(ConductorProgress<V>),
}

impl<V: UsableData + Send> fmt::Display for StrategyProgress<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyProgress::Default(p) => write!(f, "{p}"),
            StrategyProgress::NoObjective(p) => write!(f, "{p}"),
            StrategyProgress::NoObjectiveStarter(p) => write!(f, "{p}"),
            StrategyProgress::FindClosest(p) => write!(f, "{p}"),
            StrategyProgress::Fuzzy(p) => write!(f, "{p}"),
            StrategyProgress::Incremental(p) => write!(f, "{p}"),
            StrategyProgress::Conductor(p) => write!(f, "{p}"),
        }
    }
}

// Lift each strategy's own progress into the typed union, so helpers can accept any
// worker's progress via `Into<StrategyProgress<V>>`.
impl<V: UsableData + Send> From<SolveProgress<V>> for StrategyProgress<V> {
    fn from(p: SolveProgress<V>) -> Self {
        StrategyProgress::Default(p)
    }
}

impl<V: UsableData + Send> From<NoObjectiveProgressData> for StrategyProgress<V> {
    fn from(p: NoObjectiveProgressData) -> Self {
        StrategyProgress::NoObjective(p)
    }
}

impl<V: UsableData + Send> From<NoObjectiveStarterProgress<V>> for StrategyProgress<V> {
    fn from(p: NoObjectiveStarterProgress<V>) -> Self {
        StrategyProgress::NoObjectiveStarter(p)
    }
}

impl<V: UsableData + Send> From<FindClosestProgressData> for StrategyProgress<V> {
    fn from(p: FindClosestProgressData) -> Self {
        StrategyProgress::FindClosest(p)
    }
}

impl<V: UsableData + Send> From<FuzzyProgressData> for StrategyProgress<V> {
    fn from(p: FuzzyProgressData) -> Self {
        StrategyProgress::Fuzzy(p)
    }
}

impl<V: UsableData + Send> From<IncrementalProgressData> for StrategyProgress<V> {
    fn from(p: IncrementalProgressData) -> Self {
        StrategyProgress::Incremental(p)
    }
}

impl<V: UsableData + Send> From<ConductorProgress<V>> for StrategyProgress<V> {
    fn from(p: ConductorProgress<V>) -> Self {
        StrategyProgress::Conductor(p)
    }
}

/// Serializable, type-erased union of every strategy's progress. This is the only progress
/// form that crosses the IPC barrier; reconstruct the typed [`StrategyProgress<V>`] with
/// [`VarOrderSerializable::from_data`] once a `var_order` is available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyProgressData {
    Default(SolveProgressData),
    NoObjective(NoObjectiveProgressData),
    NoObjectiveStarter(NoObjectiveStarterProgressData),
    FindClosest(FindClosestProgressData),
    Fuzzy(FuzzyProgressData),
    Incremental(IncrementalProgressData),
    Conductor(ConductorProgressData),
}

impl fmt::Display for StrategyProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyProgressData::Default(p) => write!(f, "{p}"),
            StrategyProgressData::NoObjective(p) => write!(f, "{p}"),
            StrategyProgressData::NoObjectiveStarter(p) => write!(f, "{p}"),
            StrategyProgressData::FindClosest(p) => write!(f, "{p}"),
            StrategyProgressData::Fuzzy(p) => write!(f, "{p}"),
            StrategyProgressData::Incremental(p) => write!(f, "{p}"),
            StrategyProgressData::Conductor(p) => write!(f, "{p}"),
        }
    }
}

impl StrategyProgressData {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self)
            .expect("Serialization of StrategyProgressData should never fail")
    }

    pub fn deserialize(s: &str) -> Result<Self, StrategyError> {
        serde_json::from_str(s).map_err(|e| {
            StrategyError::Other(format!("failed to deserialize StrategyProgressData: {e}"))
        })
    }
}

/// Typed union of every strategy's per-run payload, carrying real target configs as
/// `ConfigData<V>`.
///
/// Mirrors [`StrategyProgress<V>`] but flows parent → subprocess. Not serializable by design
/// (the target configs are only serializable when `V` is); the serializable counterpart is
/// [`StrategyPayloadData`], reached via the [`VarOrderSerializable`] impl.
#[derive(Debug, Clone)]
pub enum StrategyPayload<V: UsableData + Send> {
    Default(DefaultPayload),
    NoObjective(NoObjectivePayload),
    NoObjectiveStarter(NoObjectiveStarterPayload),
    FindClosest(FindClosestPayload<V>),
    Fuzzy(FuzzyPayload<V>),
    Incremental(IncrementalPayload<V>),
    Conductor(ConductorPayload<V>),
}

/// Serializable, type-erased union of every strategy's per-run payload — the only payload form
/// that crosses the IPC barrier. Target configs are erased to column-indexed `Vec<f64>`;
/// reconstruct the typed [`StrategyPayload<V>`] with [`VarOrderSerializable::from_data`] once a
/// `var_order` is available.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyPayloadData {
    Default(DefaultPayload),
    NoObjective(NoObjectivePayload),
    NoObjectiveStarter(NoObjectiveStarterPayload),
    FindClosest(FindClosestPayloadData),
    Fuzzy(FuzzyPayloadData),
    Incremental(IncrementalPayloadData),
    Conductor(ConductorPayloadData),
}

impl StrategyPayloadData {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("Serialization of StrategyPayloadData should never fail")
    }

    pub fn deserialize(s: &str) -> Result<Self, StrategyError> {
        serde_json::from_str(s).map_err(|e| {
            StrategyError::Other(format!("failed to deserialize StrategyPayloadData: {e}"))
        })
    }
}

// Lift each strategy's own payload into the typed union, so the spawnable-strategy machinery can
// erase any strategy's payload via `Into<StrategyPayload>` (mirrors the progress `From` impls).
impl<V: UsableData + Send> From<DefaultPayload> for StrategyPayload<V> {
    fn from(p: DefaultPayload) -> Self {
        StrategyPayload::Default(p)
    }
}
impl<V: UsableData + Send> From<NoObjectivePayload> for StrategyPayload<V> {
    fn from(p: NoObjectivePayload) -> Self {
        StrategyPayload::NoObjective(p)
    }
}
impl<V: UsableData + Send> From<NoObjectiveStarterPayload> for StrategyPayload<V> {
    fn from(p: NoObjectiveStarterPayload) -> Self {
        StrategyPayload::NoObjectiveStarter(p)
    }
}
impl<V: UsableData + Send> From<FindClosestPayload<V>> for StrategyPayload<V> {
    fn from(p: FindClosestPayload<V>) -> Self {
        StrategyPayload::FindClosest(p)
    }
}
impl<V: UsableData + Send> From<FuzzyPayload<V>> for StrategyPayload<V> {
    fn from(p: FuzzyPayload<V>) -> Self {
        StrategyPayload::Fuzzy(p)
    }
}
impl<V: UsableData + Send> From<IncrementalPayload<V>> for StrategyPayload<V> {
    fn from(p: IncrementalPayload<V>) -> Self {
        StrategyPayload::Incremental(p)
    }
}
impl<V: UsableData + Send> From<ConductorPayload<V>> for StrategyPayload<V> {
    fn from(p: ConductorPayload<V>) -> Self {
        StrategyPayload::Conductor(p)
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for StrategyPayload<V> {
    type Data = StrategyPayloadData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<StrategyPayloadData, Infallible> {
        Ok(match self {
            StrategyPayload::Default(p) => {
                StrategyPayloadData::Default(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyPayload::NoObjective(p) => {
                StrategyPayloadData::NoObjective(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyPayload::NoObjectiveStarter(p) => StrategyPayloadData::NoObjectiveStarter(
                VarOrderSerializable::into_data(p, var_order)?,
            ),
            StrategyPayload::FindClosest(p) => {
                StrategyPayloadData::FindClosest(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyPayload::Fuzzy(p) => {
                StrategyPayloadData::Fuzzy(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyPayload::Incremental(p) => {
                StrategyPayloadData::Incremental(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyPayload::Conductor(p) => {
                StrategyPayloadData::Conductor(VarOrderSerializable::into_data(p, var_order)?)
            }
        })
    }
    fn from_data(data: &StrategyPayloadData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(match data {
            StrategyPayloadData::Default(d) => StrategyPayload::Default(
                <DefaultPayload as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::NoObjective(d) => StrategyPayload::NoObjective(
                <NoObjectivePayload as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::NoObjectiveStarter(d) => StrategyPayload::NoObjectiveStarter(
                <NoObjectiveStarterPayload as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::FindClosest(d) => StrategyPayload::FindClosest(
                <FindClosestPayload<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::Fuzzy(d) => StrategyPayload::Fuzzy(
                <FuzzyPayload<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::Incremental(d) => StrategyPayload::Incremental(
                <IncrementalPayload<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyPayloadData::Conductor(d) => StrategyPayload::Conductor(
                <ConductorPayload<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
        })
    }
}

impl StrategyKind {
    /// The empty payload for strategies that need none (`Default`, `NoObjective`,
    /// `NoObjectiveStarter`, `Conductor`). Returns `None` for `Fuzzy`/`FindClosest`/`Incremental`,
    /// which require a target config or an epoch assignment and must have their payload built
    /// explicitly.
    pub fn empty_payload<V: UsableData + Send>(&self) -> Option<StrategyPayload<V>> {
        match self {
            StrategyKind::Default(_) => Some(StrategyPayload::Default(DefaultPayload)),
            StrategyKind::NoObjective(_) => Some(StrategyPayload::NoObjective(NoObjectivePayload)),
            StrategyKind::NoObjectiveStarter(_) => Some(StrategyPayload::NoObjectiveStarter(
                NoObjectiveStarterPayload,
            )),
            StrategyKind::Conductor(_) => {
                Some(StrategyPayload::Conductor(ConductorPayload::default()))
            }
            StrategyKind::FindClosest(_)
            | StrategyKind::Fuzzy(_)
            | StrategyKind::Incremental(_) => None,
        }
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for SolveProgress<V> {
    type Data = SolveProgressData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<SolveProgressData, Infallible> {
        Ok(SolveProgress::into_data(self.clone(), var_order))
    }
    fn from_data(data: &SolveProgressData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(SolveProgressData::into_typed(data.clone(), var_order))
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for NoObjectiveProgressData {
    type Data = NoObjectiveProgressData;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<NoObjectiveProgressData, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &NoObjectiveProgressData, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for FindClosestProgressData {
    type Data = FindClosestProgressData;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<FindClosestProgressData, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &FindClosestProgressData, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for FuzzyProgressData {
    type Data = FuzzyProgressData;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<FuzzyProgressData, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &FuzzyProgressData, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for IncrementalProgressData {
    type Data = IncrementalProgressData;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<IncrementalProgressData, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &IncrementalProgressData, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for StrategyProgress<V> {
    type Data = StrategyProgressData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<StrategyProgressData, Infallible> {
        Ok(match self {
            StrategyProgress::Default(p) => {
                StrategyProgressData::Default(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyProgress::NoObjective(p) => {
                StrategyProgressData::NoObjective(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyProgress::NoObjectiveStarter(p) => StrategyProgressData::NoObjectiveStarter(
                VarOrderSerializable::into_data(p, var_order)?,
            ),
            StrategyProgress::FindClosest(p) => {
                StrategyProgressData::FindClosest(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyProgress::Fuzzy(p) => {
                StrategyProgressData::Fuzzy(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyProgress::Incremental(p) => {
                StrategyProgressData::Incremental(VarOrderSerializable::into_data(p, var_order)?)
            }
            StrategyProgress::Conductor(p) => {
                StrategyProgressData::Conductor(VarOrderSerializable::into_data(p, var_order)?)
            }
        })
    }
    fn from_data(data: &StrategyProgressData, var_order: &[V]) -> Result<Self, Infallible> {
        Ok(match data {
            StrategyProgressData::Default(d) => StrategyProgress::Default(
                <SolveProgress<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyProgressData::NoObjective(d) => {
                StrategyProgress::NoObjective(<NoObjectiveProgressData as VarOrderSerializable<
                    V,
                >>::from_data(d, var_order)?)
            }
            StrategyProgressData::NoObjectiveStarter(d) => StrategyProgress::NoObjectiveStarter(
                <NoObjectiveStarterProgress<V> as VarOrderSerializable<V>>::from_data(
                    d, var_order,
                )?,
            ),
            StrategyProgressData::FindClosest(d) => {
                StrategyProgress::FindClosest(<FindClosestProgressData as VarOrderSerializable<
                    V,
                >>::from_data(d, var_order)?)
            }
            StrategyProgressData::Fuzzy(d) => StrategyProgress::Fuzzy(
                <FuzzyProgressData as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
            StrategyProgressData::Incremental(d) => {
                StrategyProgress::Incremental(<IncrementalProgressData as VarOrderSerializable<
                    V,
                >>::from_data(d, var_order)?)
            }
            StrategyProgressData::Conductor(d) => StrategyProgress::Conductor(
                <ConductorProgress<V> as VarOrderSerializable<V>>::from_data(d, var_order)?,
            ),
        })
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for StrategyProgress<V> {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        Ok(progress)
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for SolveProgress<V> {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::Default(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for NoObjectiveProgressData {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::NoObjective(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for NoObjectiveStarterProgress<V> {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::NoObjectiveStarter(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for FindClosestProgressData {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::FindClosest(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for FuzzyProgressData {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::Fuzzy(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for IncrementalProgressData {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::Incremental(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl<V: UsableData + Send> StrategyProgressVariant<V> for ConductorProgress<V> {
    fn from_strategy_progress(progress: StrategyProgress<V>) -> Result<Self, StrategyProgress<V>> {
        match progress {
            StrategyProgress::Conductor(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl From<DefaultStrategy> for StrategyKind {
    fn from(s: DefaultStrategy) -> Self {
        StrategyKind::Default(s)
    }
}

impl From<NoObjectiveStrategy> for StrategyKind {
    fn from(s: NoObjectiveStrategy) -> Self {
        StrategyKind::NoObjective(s)
    }
}

impl From<NoObjectiveStarterStrategy> for StrategyKind {
    fn from(s: NoObjectiveStarterStrategy) -> Self {
        StrategyKind::NoObjectiveStarter(s)
    }
}

impl From<FindClosestStrategy> for StrategyKind {
    fn from(s: FindClosestStrategy) -> Self {
        StrategyKind::FindClosest(s)
    }
}

impl From<FuzzyStrategy> for StrategyKind {
    fn from(s: FuzzyStrategy) -> Self {
        StrategyKind::Fuzzy(s)
    }
}

impl From<IncrementalStrategy> for StrategyKind {
    fn from(s: IncrementalStrategy) -> Self {
        StrategyKind::Incremental(s)
    }
}

impl From<ConductorStrategy> for StrategyKind {
    fn from(s: ConductorStrategy) -> Self {
        StrategyKind::Conductor(s)
    }
}

/// A strategy that can be spawned as a subprocess for a given variable type `V`.
///
/// `V` is a trait parameter (not a GAT) so the blanket impl below can bound
/// `Self::Progress: StrategyProgressVariant<V>` as an ordinary per-instantiation
/// `where` clause — a GAT would require an (inexpressible) `for<V>` bound.
pub trait SpawnableStrategy<V: UsableData + Send> {
    type Progress: Send;
    type Payload: Send;
    fn to_strategy_kind(&self) -> StrategyKind;
    /// Reconstruct the typed progress from the erased form received over IPC, returning
    /// the typed union unchanged if it carries a variant this strategy never emits.
    fn convert_progress(
        data: StrategyProgressData,
        var_order: &[V],
    ) -> Result<Self::Progress, StrategyProgress<V>>;
    /// Erase this strategy's typed payload into the serializable form sent over IPC, encoding
    /// any variable-keyed data against `var_order` (the inverse of [`Self::convert_progress`]).
    fn payload_into_data(payload: Self::Payload, var_order: &[V]) -> StrategyPayloadData;
}

/// Every `Strategy` that can be turned into a `StrategyKind`, whose progress is a variant of
/// the typed union and whose payload lifts into [`StrategyPayload`], is spawnable:
/// deserialize-then-project for progress, lift-then-erase for the payload.
impl<V, S> SpawnableStrategy<V> for S
where
    V: UsableData + Send,
    S: Strategy + Clone,
    StrategyKind: From<S>,
    <S as Strategy>::Progress<V>: StrategyProgressVariant<V>,
    StrategyPayload<V>: From<<S as Strategy>::Payload<V>>,
{
    type Progress = <S as Strategy>::Progress<V>;
    type Payload = <S as Strategy>::Payload<V>;
    fn to_strategy_kind(&self) -> StrategyKind {
        StrategyKind::from(self.clone())
    }
    fn convert_progress(
        data: StrategyProgressData,
        var_order: &[V],
    ) -> Result<Self::Progress, StrategyProgress<V>> {
        let typed = <StrategyProgress<V> as VarOrderSerializable<V>>::from_data(&data, var_order)
            .unwrap_or_else(|e| match e {});
        <Self::Progress as StrategyProgressVariant<V>>::from_strategy_progress(typed)
    }
    fn payload_into_data(payload: Self::Payload, var_order: &[V]) -> StrategyPayloadData {
        let union: StrategyPayload<V> = StrategyPayload::from(payload);
        <StrategyPayload<V> as VarOrderSerializable<V>>::into_data(&union, var_order)
            .unwrap_or_else(|e| match e {})
    }
}

#[async_trait]
impl Strategy for StrategyKind {
    type Progress<V: UsableData + Send> = StrategyProgress<V>;
    type Payload<V: UsableData + Send> = StrategyPayload<V>;

    fn name(&self) -> &'static str {
        match self {
            StrategyKind::Default(s) => s.name(),
            StrategyKind::NoObjective(s) => s.name(),
            StrategyKind::NoObjectiveStarter(s) => s.name(),
            StrategyKind::FindClosest(s) => s.name(),
            StrategyKind::Fuzzy(s) => s.name(),
            StrategyKind::Incremental(s) => s.name(),
            StrategyKind::Conductor(s) => s.name(),
        }
    }

    fn ui_name(&self) -> &'static str {
        match self {
            StrategyKind::Default(s) => s.ui_name(),
            StrategyKind::NoObjective(s) => s.ui_name(),
            StrategyKind::NoObjectiveStarter(s) => s.ui_name(),
            StrategyKind::FindClosest(s) => s.ui_name(),
            StrategyKind::Fuzzy(s) => s.ui_name(),
            StrategyKind::Incremental(s) => s.ui_name(),
            StrategyKind::Conductor(s) => s.ui_name(),
        }
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: StrategyPayload<InternalVar<B, E>>,
        on_progress: &(dyn Fn(StrategyProgress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        // Project the payload union down to the variant this strategy expects. A mismatch means
        // the caller built a payload for a different `StrategyKind` and is a programming error.
        let mismatch = || {
            StrategyError::Other(format!(
                "payload variant does not match strategy `{}`",
                self.name()
            ))
        };
        match self {
            StrategyKind::Default(s) => {
                let StrategyPayload::Default(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::Default(p))
                })
                .await
            }
            StrategyKind::NoObjective(s) => {
                let StrategyPayload::NoObjective(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::NoObjective(p))
                })
                .await
            }
            StrategyKind::NoObjectiveStarter(s) => {
                let StrategyPayload::NoObjectiveStarter(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::NoObjectiveStarter(p))
                })
                .await
            }
            StrategyKind::FindClosest(s) => {
                let StrategyPayload::FindClosest(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::FindClosest(p))
                })
                .await
            }
            StrategyKind::Fuzzy(s) => {
                let StrategyPayload::Fuzzy(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::Fuzzy(p))
                })
                .await
            }
            StrategyKind::Incremental(s) => {
                let StrategyPayload::Incremental(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::Incremental(p))
                })
                .await
            }
            StrategyKind::Conductor(s) => {
                let StrategyPayload::Conductor(payload) = payload else {
                    return Err(mismatch());
                };
                s.run_with_callback(ctx, model, warm_start, payload, &|p| {
                    on_progress(StrategyProgress::Conductor(p))
                })
                .await
            }
        }
    }
}

impl StrategyContext {
    pub async fn solve_model<B, E, C>(
        &self,
        model: &Model<B, E, C>,
        opts: SolveProblemOpts<InternalVar<B, E>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        self.solve_model_with_progress(model, opts, &|_| true).await
    }

    pub async fn solve_model_with_progress<B, E, C>(
        &self,
        model: &Model<B, E, C>,
        opts: SolveProblemOpts<InternalVar<B, E>>,
        on_progress: &(dyn Fn(SolveProgress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        self.solve_problem_with_progress(model.problem(), opts, on_progress)
            .await
    }

    pub async fn solve_model_with_echo<B, E, C>(
        &self,
        model: &Model<B, E, C>,
        opts: SolveProblemOpts<InternalVar<B, E>>,
        on_progress: &(dyn Fn(SolveProgress<InternalVar<B, E>>) -> bool + Send + Sync),
        handle_echo: &(dyn Fn(String) -> Option<String> + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        self.solve_problem_with_echo(model.problem(), opts, on_progress, handle_echo)
            .await
    }

    pub async fn run_strategy<B, E, C>(
        &self,
        strategy: &StrategyKind,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: StrategyPayload<InternalVar<B, E>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        strategy
            .run_with_callback(self, model, warm_start, payload, &|_| true)
            .await
    }

    pub async fn run_strategy_with_callback<B, E, C>(
        &self,
        strategy: &StrategyKind,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: StrategyPayload<InternalVar<B, E>>,
        on_progress: &(dyn Fn(StrategyProgress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        strategy
            .run_with_callback(self, model, warm_start, payload, on_progress)
            .await
    }

    pub async fn spawn_strategy<B, E, C, S>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: S::Payload,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
        S: SpawnableStrategy<InternalVar<B, E>>,
    {
        self.spawn_strategy_with_callback(strategy, model, warm_start, payload, &|_| true)
            .await
    }

    pub async fn spawn_strategy_with_callback<B, E, C, S>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: S::Payload,
        on_progress: &(dyn Fn(S::Progress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
        S: SpawnableStrategy<InternalVar<B, E>>,
    {
        let (model_desc, var_order) = model.to_desc();
        let strategy_kind = strategy.to_strategy_kind();

        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));
        let payload_data = S::payload_into_data(payload, &var_order);

        let noop_echo = |_: String| {};
        let echo_fn: &(dyn Fn(String) + Send + Sync) = match &self.on_echo {
            Some(f) => f.as_ref(),
            None => &noop_echo,
        };

        let raw_on_progress = |data: StrategyProgressData| -> bool {
            match S::convert_progress(data, &var_order) {
                Ok(typed) => on_progress(typed),
                Err(leftover) => {
                    echo_fn(format!("unexpected progress variant: {leftover}"));
                    true
                }
            }
        };

        let raw = self
            .backend
            .run_strategy_subprocess(
                &model_desc,
                &strategy_kind,
                raw_warm_start,
                payload_data,
                &raw_on_progress,
                echo_fn,
            )
            .await?;

        Ok(raw.into_typed(&var_order))
    }

    pub async fn spawn_strategy_with_echo<B, E, C, S>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        payload: S::Payload,
        on_progress: &(dyn Fn(S::Progress) -> bool + Send + Sync),
        handle_echo: &(dyn Fn(String) -> Option<String> + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
        S: SpawnableStrategy<InternalVar<B, E>>,
    {
        let (model_desc, var_order) = model.to_desc();
        let strategy_kind = strategy.to_strategy_kind();

        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));
        let payload_data = S::payload_into_data(payload, &var_order);

        let echo_impl: Box<dyn Fn(String) + Send + Sync + '_> = match &self.on_echo {
            Some(ctx_echo) => Box::new(move |line| {
                if let Some(out) = handle_echo(line) {
                    ctx_echo(out);
                }
            }),
            // No parent sink: still call handle_echo so its side effects/routing run.
            None => Box::new(move |line| {
                let _ = handle_echo(line);
            }),
        };

        let raw_on_progress = |data: StrategyProgressData| -> bool {
            match S::convert_progress(data, &var_order) {
                Ok(typed) => on_progress(typed),
                Err(leftover) => {
                    echo_impl(format!("unexpected progress variant: {leftover}"));
                    true
                }
            }
        };

        let raw = self
            .backend
            .run_strategy_subprocess(
                &model_desc,
                &strategy_kind,
                raw_warm_start,
                payload_data,
                &raw_on_progress,
                &*echo_impl,
            )
            .await?;

        Ok(raw.into_typed(&var_order))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRequest {
    pub model_desc: ModelDesc,
    pub strategy: StrategyKind,
    pub warm_start: Option<Vec<f64>>,
    /// Erased per-run payload, reconstructed to the typed [`StrategyPayload`] in the subprocess
    /// against the model's `var_order`.
    pub payload: StrategyPayloadData,
}

impl StrategyRequest {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("Serialization of StrategyRequest should never fail")
    }

    pub fn deserialize(s: &str) -> Result<Self, StrategyError> {
        serde_json::from_str(s).map_err(|e| {
            StrategyError::Other(format!("failed to deserialize StrategyRequest: {e}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use collomatique_ilp::Variable;
    use collomatique_ilp_modeler::Modeler;
    use futures::channel::oneshot;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    #[test]
    fn solve_progress_data_display_signals_incumbent_presence() {
        let base = SolveProgressData {
            best_obj: None,
            best_bound: 0.5,
            node_count: 7,
            solutions_found: 2,
            incumbent: None,
        };
        assert!(format!("{base}").ends_with("incumbent=no"));

        let with_incumbent = SolveProgressData {
            incumbent: Some(vec![0.0, 1.0]),
            ..base
        };
        assert!(format!("{with_incumbent}").ends_with("incumbent=yes"));
    }

    #[test]
    fn strategy_payload_fuzzy_target_survives_erase_and_reconstruct() {
        // A Fuzzy payload's target lives in the model's variable space. Erasing it to a
        // StrategyPayloadData (Vec<f64>) and reconstructing against the same var_order must
        // recover the exact config, mirroring how an incumbent crosses the subprocess barrier.
        let var_order: Vec<usize> = vec![0, 1, 2];
        let raw = vec![1.0, 0.0, 1.0];
        let target = collomatique_ilp::solution_to_config_data(&raw, &var_order);

        let payload: StrategyPayload<usize> = StrategyPayload::Fuzzy(FuzzyPayload { target });
        let data = VarOrderSerializable::into_data(&payload, &var_order).unwrap();
        assert_eq!(
            data,
            StrategyPayloadData::Fuzzy(FuzzyPayloadData {
                target: raw.clone()
            })
        );

        let restored =
            <StrategyPayload<usize> as VarOrderSerializable<usize>>::from_data(&data, &var_order)
                .unwrap();
        let StrategyPayload::Fuzzy(FuzzyPayload { target }) = restored else {
            panic!("expected a Fuzzy payload");
        };
        assert_eq!(
            collomatique_ilp::config_data_to_hint(&target, &var_order),
            raw
        );
    }

    #[test]
    fn incremental_kind_and_payload_survive_json_round_trip() {
        // The whole IPC hop for the incremental strategy: the StrategyKind (with its f64
        // weight) and the erased payload must serialize to JSON and back unchanged.
        let kind = StrategyKind::Incremental(IncrementalStrategy {
            l1_weight: 1e6,
            distance_tolerance: 5.0,
            epoch_time_limit: collomatique_time::TimeLimit::seconds(
                std::num::NonZeroU32::new(30).unwrap(),
            ),
            epoch_incumbent_time_limit: collomatique_time::TimeLimit::seconds(
                std::num::NonZeroU32::new(10).unwrap(),
            ),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: false,
        });
        let json = serde_json::to_string(&kind).unwrap();
        assert_eq!(serde_json::from_str::<StrategyKind>(&json).unwrap(), kind);

        // An epoch payload erases against var_order to a Vec<Option<u32>> and reconstructs.
        let var_order: Vec<usize> = vec![0, 1, 2];
        let payload: StrategyPayload<usize> = StrategyPayload::Incremental(IncrementalPayload {
            epochs: HashMap::from([(0usize, 0u32), (2usize, 1u32)]),
        });
        let data = VarOrderSerializable::into_data(&payload, &var_order).unwrap();
        assert_eq!(
            data,
            StrategyPayloadData::Incremental(IncrementalPayloadData {
                epochs: vec![Some(0), None, Some(1)]
            })
        );

        let json = serde_json::to_string(&data).unwrap();
        let back: StrategyPayloadData = serde_json::from_str(&json).unwrap();
        assert_eq!(back, data);

        let restored =
            <StrategyPayload<usize> as VarOrderSerializable<usize>>::from_data(&back, &var_order)
                .unwrap();
        let StrategyPayload::Incremental(IncrementalPayload { epochs }) = restored else {
            panic!("expected an Incremental payload");
        };
        assert_eq!(epochs, HashMap::from([(0usize, 0u32), (2usize, 1u32)]));
    }

    struct MockBackend {
        outcome: RawSolveOutcome,
    }

    #[async_trait]
    impl SolveBackend for MockBackend {
        async fn solve_with_progress(
            &self,
            _desc: &ProblemDesc,
            _opts: SolveConfig,
            _on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            Ok(self.outcome.clone())
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            _strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _payload: StrategyPayloadData,
            _on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            Ok(self.outcome.clone())
        }
    }

    struct SequentialMockBackend {
        outcomes: Mutex<VecDeque<RawSolveOutcome>>,
    }

    impl SequentialMockBackend {
        fn new(outcomes: Vec<RawSolveOutcome>) -> Self {
            Self {
                outcomes: Mutex::new(outcomes.into()),
            }
        }
    }

    #[async_trait]
    impl SolveBackend for SequentialMockBackend {
        async fn solve_with_progress(
            &self,
            _desc: &ProblemDesc,
            _opts: SolveConfig,
            _on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            let outcome = self
                .outcomes
                .lock()
                .unwrap()
                .pop_front()
                .expect("SequentialMockBackend: no more outcomes");
            Ok(outcome)
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            _strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _payload: StrategyPayloadData,
            _on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            unreachable!()
        }
    }

    /// Backend that emits a single progress update (carrying an incumbent) before
    /// returning its canned outcome.
    struct ProgressMockBackend {
        progress: SolveProgressData,
        outcome: RawSolveOutcome,
    }

    #[async_trait]
    impl SolveBackend for ProgressMockBackend {
        async fn solve_with_progress(
            &self,
            _desc: &ProblemDesc,
            _opts: SolveConfig,
            on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            on_progress(self.progress.clone());
            Ok(self.outcome.clone())
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            _strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _payload: StrategyPayloadData,
            _on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            unreachable!()
        }
    }

    fn make_model(
        base_vars: Vec<(usize, Variable)>,
    ) -> collomatique_ilp_modeler::Model<usize, (), ()> {
        let vars: HashMap<usize, Variable> = base_vars.into_iter().collect();
        let modeler: Modeler<'_, usize, (), (), (), ()> = Modeler::new(vars);
        modeler.build(&()).unwrap()
    }

    #[tokio::test]
    async fn solve_model_with_progress_reports_typed_incumbent() {
        // The raw incumbent crosses the backend boundary as a column-indexed Vec<f64>;
        // solve_model_with_progress must reconstruct it into a typed ConfigData<V> keyed
        // by the exact var_order it uses internally. Build the raw vector against that same
        // ordering so the test is insensitive to HashMap iteration order.
        let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);
        let (_, var_order) = model.problem().get_desc();

        let incumbent: Vec<f64> = var_order
            .iter()
            .map(|iv| match iv {
                InternalVar::Base(0) => 1.0,
                InternalVar::Base(1) => 0.0,
                _ => 0.0,
            })
            .collect();

        let backend = Arc::new(ProgressMockBackend {
            progress: SolveProgressData {
                best_obj: Some(1.0),
                best_bound: 2.0,
                node_count: 3,
                solutions_found: 1,
                incumbent: Some(incumbent),
            },
            outcome: RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(1.0),
                best_bound: Some(1.0),
                solution: Some(vec![1.0, 0.0]),
            },
        });

        let ctx = StrategyContext::new(backend);
        let captured: Mutex<Option<ConfigData<InternalVar<usize, ()>>>> = Mutex::new(None);

        let outcome = ctx
            .solve_model_with_progress(
                &model,
                SolveProblemOpts {
                    warm_start: None,
                    time_limit: collomatique_time::TimeLimit::none(),
                    incumbent_time_limit: collomatique_time::TimeLimit::none(),
                    disable_logging: false,
                },
                &|p: SolveProgress<InternalVar<usize, ()>>| {
                    *captured.lock().unwrap() = p.incumbent;
                    true
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);

        let incumbent = captured
            .into_inner()
            .unwrap()
            .expect("a typed incumbent should be reported");
        assert_eq!(incumbent.get(InternalVar::Base(0)), Some(1.0));
        assert_eq!(incumbent.get(InternalVar::Base(1)), Some(0.0));
    }

    #[tokio::test]
    async fn default_strategy_returns_mock_result() {
        let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);

        let backend = Arc::new(MockBackend {
            outcome: RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(42.0),
                best_bound: Some(42.0),
                solution: Some(vec![1.0, 1.0]),
            },
        });

        let ctx = StrategyContext::new(backend);
        let strategy = DefaultStrategy::default();
        let outcome = strategy
            .run(&ctx, &model, DefaultPayload::default())
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(42.0));
        let solution = outcome.solution.unwrap();
        assert_eq!(solution.get(InternalVar::Base(0usize)), Some(1.0));
        assert_eq!(solution.get(InternalVar::Base(1usize)), Some(1.0));
    }

    #[tokio::test]
    async fn strategy_kind_dispatches_to_default() {
        let model = make_model(vec![(0, Variable::binary())]);

        let backend = Arc::new(MockBackend {
            outcome: RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(1.0),
                best_bound: Some(1.0),
                solution: Some(vec![1.0]),
            },
        });

        let ctx = StrategyContext::new(backend);
        let kind = StrategyKind::Default(DefaultStrategy::default());
        let outcome = ctx
            .run_strategy(&kind, &model, None, kind.empty_payload().unwrap())
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(
            outcome.solution.unwrap().get(InternalVar::Base(0usize)),
            Some(1.0)
        );
    }

    #[tokio::test]
    async fn spawn_strategy_returns_mock_result() {
        // Failure if it happens is non-deterministic (because of the HashMap seed)
        // In case of bad code, odds of failures are about 50-50, so we repeat
        // a couple of hundreds time. It should be enough to catch it.
        for _ in 0..200 {
            let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);
            let (_, var_order) = model.to_desc();

            let mut solution_vec = vec![0.0; var_order.len()];
            for (i, iv) in var_order.iter().enumerate() {
                solution_vec[i] = match iv {
                    InternalVar::Base(0) => 1.0,
                    InternalVar::Base(1) => 0.0,
                    _ => panic!("unexpected var"),
                };
            }

            let backend = Arc::new(MockBackend {
                outcome: RawSolveOutcome {
                    status: SolveStatus::Optimal,
                    objective: Some(7.0),
                    best_bound: Some(7.0),
                    solution: Some(solution_vec),
                },
            });

            let ctx = StrategyContext::new(backend);
            let kind = StrategyKind::Default(DefaultStrategy::default());
            let outcome = ctx
                .spawn_strategy(&kind, &model, None, kind.empty_payload().unwrap())
                .await
                .unwrap();

            assert_eq!(outcome.status, SolveStatus::Optimal);
            assert_eq!(outcome.objective, Some(7.0));
            let solution = outcome.solution.unwrap();
            assert_eq!(solution.get(InternalVar::Base(0)), Some(1.0));
            assert_eq!(solution.get(InternalVar::Base(1)), Some(0.0));
        }
    }

    #[tokio::test]
    async fn no_objective_strategy_happy_path() {
        let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![
            // Checker solve (both vars = 1.0 to avoid var_order sensitivity)
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(0.0),
                best_bound: Some(0.0),
                solution: Some(vec![1.0, 1.0]),
            },
            // Reconstruction solve (no extras in trivial model)
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(5.0),
                best_bound: Some(5.0),
                solution: Some(vec![]),
            },
        ]));

        let ctx = StrategyContext::new(backend);
        let strategy = NoObjectiveStrategy {
            checker_time_limit: collomatique_time::TimeLimit::none(),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: false,
        };

        let progress_log: Mutex<Vec<NoObjectiveProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy
            .run_with_callback(&ctx, &model, None, NoObjectivePayload::default(), &|p| {
                progress_log.lock().unwrap().push(p);
                true
            })
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(5.0));
        let solution = outcome.solution.unwrap();
        assert_eq!(solution.get(InternalVar::Base(0usize)), Some(1.0));
        assert_eq!(solution.get(InternalVar::Base(1usize)), Some(1.0));

        let log = progress_log.into_inner().unwrap();
        assert!(
            log.iter()
                .any(|p| matches!(p, NoObjectiveProgressData::SolutionFound { .. }))
        );
    }

    #[tokio::test]
    async fn no_objective_strategy_infeasible() {
        let model = make_model(vec![(0, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![RawSolveOutcome {
            status: SolveStatus::Infeasible,
            objective: None,
            best_bound: None,
            solution: None,
        }]));

        let ctx = StrategyContext::new(backend);
        let strategy = NoObjectiveStrategy {
            checker_time_limit: collomatique_time::TimeLimit::none(),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: false,
        };

        let outcome = strategy
            .run(&ctx, &model, NoObjectivePayload::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, SolveStatus::Infeasible);
        assert!(outcome.solution.is_none());
    }

    #[tokio::test]
    async fn no_objective_strategy_kind_dispatch() {
        let model = make_model(vec![(0, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(0.0),
                best_bound: Some(0.0),
                solution: Some(vec![1.0]),
            },
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(3.0),
                best_bound: Some(3.0),
                solution: Some(vec![]),
            },
        ]));

        let ctx = StrategyContext::new(backend);
        let kind = StrategyKind::NoObjective(NoObjectiveStrategy {
            checker_time_limit: collomatique_time::TimeLimit::none(),
            reconstruction_time_limit: collomatique_time::TimeLimit::none(),
            disable_logging: false,
        });
        let outcome = ctx
            .run_strategy(&kind, &model, None, kind.empty_payload().unwrap())
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(3.0));
    }

    #[tokio::test]
    async fn no_objective_starter_happy_path() {
        let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![
            // Checker solve (both vars = 1.0 to avoid var_order sensitivity)
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(0.0),
                best_bound: Some(0.0),
                solution: Some(vec![1.0, 1.0]),
            },
            // Reconstruction solve
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(5.0),
                best_bound: Some(5.0),
                solution: Some(vec![]),
            },
            // Default solve (with warm start from no-objective)
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(3.0),
                best_bound: Some(3.0),
                solution: Some(vec![1.0, 1.0]),
            },
        ]));

        let ctx = StrategyContext::new(backend);
        let strategy = NoObjectiveStarterStrategy {
            no_objective: NoObjectiveStrategy {
                checker_time_limit: collomatique_time::TimeLimit::none(),
                reconstruction_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
            },
            default: DefaultStrategy {
                time_limit: collomatique_time::TimeLimit::none(),
                incumbent_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
            },
        };

        let progress_log: Mutex<Vec<String>> = Mutex::new(Vec::new());
        let outcome = strategy
            .run_with_callback(
                &ctx,
                &model,
                None,
                NoObjectiveStarterPayload::default(),
                &|p: NoObjectiveStarterProgress<InternalVar<usize, ()>>| {
                    let tag = match &p {
                        NoObjectiveStarterProgress::Starter(_) => "starter",
                        NoObjectiveStarterProgress::HintFound { .. } => "hint",
                        NoObjectiveStarterProgress::Default(_) => "default",
                    };
                    progress_log.lock().unwrap().push(tag.to_owned());
                    true
                },
            )
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(3.0));
        let solution = outcome.solution.unwrap();
        assert_eq!(solution.get(InternalVar::Base(0usize)), Some(1.0));
        assert_eq!(solution.get(InternalVar::Base(1usize)), Some(1.0));

        let log = progress_log.into_inner().unwrap();
        assert!(log.contains(&"hint".to_owned()));
    }

    #[tokio::test]
    async fn no_objective_starter_infeasible() {
        let model = make_model(vec![(0, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![RawSolveOutcome {
            status: SolveStatus::Infeasible,
            objective: None,
            best_bound: None,
            solution: None,
        }]));

        let ctx = StrategyContext::new(backend);
        let strategy = NoObjectiveStarterStrategy {
            no_objective: NoObjectiveStrategy {
                checker_time_limit: collomatique_time::TimeLimit::none(),
                reconstruction_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
            },
            default: DefaultStrategy::default(),
        };

        let outcome = strategy
            .run(&ctx, &model, NoObjectiveStarterPayload::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, SolveStatus::Infeasible);
        assert!(outcome.solution.is_none());
    }

    #[tokio::test]
    async fn no_objective_starter_kind_dispatch() {
        let model = make_model(vec![(0, Variable::binary())]);

        let backend = Arc::new(SequentialMockBackend::new(vec![
            // Checker
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(0.0),
                best_bound: Some(0.0),
                solution: Some(vec![1.0]),
            },
            // Reconstruction
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(5.0),
                best_bound: Some(5.0),
                solution: Some(vec![]),
            },
            // Default
            RawSolveOutcome {
                status: SolveStatus::Optimal,
                objective: Some(2.0),
                best_bound: Some(2.0),
                solution: Some(vec![1.0]),
            },
        ]));

        let ctx = StrategyContext::new(backend);
        let kind = StrategyKind::NoObjectiveStarter(NoObjectiveStarterStrategy {
            no_objective: NoObjectiveStrategy {
                checker_time_limit: collomatique_time::TimeLimit::none(),
                reconstruction_time_limit: collomatique_time::TimeLimit::none(),
                disable_logging: false,
            },
            default: DefaultStrategy::default(),
        });
        let outcome = ctx
            .run_strategy(&kind, &model, None, kind.empty_payload().unwrap())
            .await
            .unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(2.0));
    }

    /// Backend for the conductor mid-run-wake regression test. The `Default` worker announces a
    /// feasible-but-unproven incumbent and then *keeps running* (blocks until released); the first
    /// `Fuzzy` worker releases it. So the whole run can only finish if that mid-run incumbent woke
    /// the scheduler and topped up the still-idle slot with a fuzzy worker — otherwise `Default`
    /// blocks forever and the test's watchdog timeout fires. Reproduces the bug where the loop only
    /// re-scheduled when a worker *ended*, so fuzzy never launched while `Default` ground on.
    struct MidRunIncumbentBackend {
        incumbent: Vec<f64>,
        // `Default` takes the receiver and awaits it; the first `Fuzzy` takes the sender and fires.
        release_rx: Mutex<Option<oneshot::Receiver<()>>>,
        release_tx: Mutex<Option<oneshot::Sender<()>>>,
    }

    #[async_trait]
    impl SolveBackend for MidRunIncumbentBackend {
        async fn solve_with_progress(
            &self,
            _desc: &ProblemDesc,
            _opts: SolveConfig,
            _on_progress: &(dyn Fn(SolveProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            unreachable!()
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _payload: StrategyPayloadData,
            on_progress: &(dyn Fn(StrategyProgressData) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            match strategy {
                StrategyKind::Default(_) => {
                    // Announce a feasible incumbent whose bound is strictly worse than its
                    // objective, so the optimality gap stays open and fuzzy remains eligible.
                    on_progress(StrategyProgressData::Default(SolveProgressData {
                        best_obj: Some(5.0),
                        best_bound: 1.0,
                        node_count: 1,
                        solutions_found: 1,
                        incumbent: Some(self.incumbent.clone()),
                    }));
                    // Keep grinding until a fuzzy worker releases us. Take the receiver out of the
                    // mutex in its own statement so the guard is not held across the await (which
                    // would make this future non-Send).
                    let rx = self.release_rx.lock().unwrap().take();
                    if let Some(rx) = rx {
                        let _ = rx.await;
                    }
                    Ok(RawSolveOutcome {
                        status: SolveStatus::Optimal,
                        objective: Some(5.0),
                        best_bound: Some(5.0),
                        solution: Some(self.incumbent.clone()),
                    })
                }
                StrategyKind::Fuzzy(_) => {
                    // Reaching here proves the mid-run incumbent woke the scheduler and the idle
                    // slot was topped up with fuzzy. Release the still-running Default worker.
                    let tx = self.release_tx.lock().unwrap().take();
                    if let Some(tx) = tx {
                        let _ = tx.send(());
                    }
                    // Yield so the now-ready Default worker gets polled instead of spinning on
                    // repeated fuzzy relaunches into the freed slot.
                    tokio::task::yield_now().await;
                    Ok(RawSolveOutcome {
                        status: SolveStatus::Stopped(StopReason::Callback),
                        objective: None,
                        best_bound: None,
                        solution: None,
                    })
                }
                _ => unreachable!("only Default and Fuzzy are enabled in this test"),
            }
        }
    }

    #[test]
    fn conductor_launches_fuzzy_on_midrun_incumbent() {
        use std::num::NonZeroU32;
        use std::sync::mpsc;
        use std::time::Duration;

        // Run the conductor on a dedicated current-thread runtime and watchdog it from here with a
        // real timeout (tokio's `time` feature is not enabled). Without the mid-run wake fix the
        // Default worker would block forever, so `recv_timeout` is what turns a regression into a
        // clean failure instead of a hang.
        let (done_tx, done_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .build()
                .expect("current-thread runtime");
            let result = rt.block_on(async {
                let model = make_model(vec![(0, Variable::binary()), (1, Variable::binary())]);
                let (release_tx, release_rx) = oneshot::channel::<()>();
                let backend = Arc::new(MidRunIncumbentBackend {
                    incumbent: vec![1.0, 1.0],
                    release_rx: Mutex::new(Some(release_rx)),
                    release_tx: Mutex::new(Some(release_tx)),
                });
                let ctx = StrategyContext::new(backend);
                let strategy = ConductorStrategy {
                    worker_count: NonZeroU32::new(2).unwrap(),
                    default_config: Some(DefaultConfig::default()),
                    warm_start_config: None,
                    incremental_config: None,
                    fuzzy_config: Some(FuzzyConfig::default()),
                };

                let saw_fuzzy = Arc::new(Mutex::new(false));
                let saw_fuzzy_cb = saw_fuzzy.clone();
                let on_progress = move |p: ConductorProgress<InternalVar<usize, ()>>| {
                    if let ConductorProgress::WorkerAssigned {
                        strategy: Some(kind),
                        ..
                    } = &p
                    {
                        if matches!(**kind, StrategyKind::Fuzzy(_)) {
                            *saw_fuzzy_cb.lock().unwrap() = true;
                        }
                    }
                    true
                };

                let outcome = strategy
                    .run_with_callback(
                        &ctx,
                        &model,
                        None,
                        ConductorPayload::default(),
                        &on_progress,
                    )
                    .await
                    .unwrap();
                (outcome.status, *saw_fuzzy.lock().unwrap())
            });
            let _ = done_tx.send(result);
        });

        let (status, saw_fuzzy) = done_rx.recv_timeout(Duration::from_secs(5)).expect(
            "conductor did not finish in time — the mid-run incumbent never launched fuzzy",
        );
        handle.join().unwrap();

        assert!(
            saw_fuzzy,
            "a fuzzy worker should have been assigned to the idle slot after the mid-run incumbent"
        );
        assert_eq!(status, SolveStatus::Optimal);
    }
}
