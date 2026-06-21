mod strategies;

pub use strategies::conductor::{
    ConductorProgress, ConductorStatus, ConductorStrategy, Solution, update_best_bound,
    update_best_solution,
};
pub use strategies::default::DefaultStrategy;
pub use strategies::no_objective::{NoObjectiveProgressData, NoObjectiveStrategy};

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::mat_repr::ProblemRepr;
use collomatique_ilp::{ConfigData, Problem, ProblemDesc, UsableData};
use collomatique_ilp_modeler::model_desc::ModelDesc;
use collomatique_ilp_modeler::{InternalVar, Model};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolveStatus {
    Optimal,
    Infeasible,
    Stopped,
    Error,
}

pub struct SolveConfig {
    pub warm_start: Option<Vec<f64>>,
    pub time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

pub struct SolveProblemOpts<V: UsableData> {
    pub warm_start: Option<ConfigData<V>>,
    pub time_limit_seconds: Option<u32>,
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolveProgress {
    pub best_obj: f64,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
}

impl fmt::Display for SolveProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "obj={:.4} bound={:.4} nodes={} solutions={}",
            self.best_obj, self.best_bound, self.node_count, self.solutions_found
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
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
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
        tag_echo: &(dyn Fn(String) -> String + Send + Sync),
    ) -> Result<RawSolveOutcome, StrategyError> {
        let echo_impl: Box<dyn Fn(String) + Send + Sync + '_> = match &self.on_echo {
            Some(ctx_echo) => Box::new(move |line| ctx_echo(tag_echo(line))),
            None => Box::new(move |line| {
                let _ = tag_echo(line);
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
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
            time_limit_seconds: opts.time_limit_seconds,
            disable_logging: opts.disable_logging,
        };

        let raw = self
            .solve_with_progress(&desc, solve_config, on_progress)
            .await?;

        Ok(raw.into_typed(&var_order))
    }

    pub async fn solve_problem_with_echo<V, C, P>(
        &self,
        problem: &Problem<V, C, P>,
        opts: SolveProblemOpts<V>,
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
        tag_echo: &(dyn Fn(String) -> String + Send + Sync),
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
            time_limit_seconds: opts.time_limit_seconds,
            disable_logging: opts.disable_logging,
        };

        let raw = self
            .solve_with_progress_and_echo(&desc, solve_config, on_progress, tag_echo)
            .await?;

        Ok(raw.into_typed(&var_order))
    }
}

#[async_trait]
pub trait Strategy: Send + Sync {
    type Progress<V: UsableData + Send>: Send + Sync + Clone;

    fn name(&self) -> &'static str;

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
        C: UsableData + Send;

    async fn run<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        self.run_with_callback(ctx, model, None, &|_| true).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyKind {
    Default(DefaultStrategy),
    NoObjective(NoObjectiveStrategy),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyProgress {
    Default(SolveProgress),
    NoObjective(NoObjectiveProgressData),
}

impl fmt::Display for StrategyProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StrategyProgress::Default(p) => write!(f, "{p}"),
            StrategyProgress::NoObjective(p) => write!(f, "{p}"),
        }
    }
}

impl StrategyProgress {
    pub fn serialize(&self) -> String {
        serde_json::to_string(self).expect("Serialization of StrategyProgress should never fail")
    }

    pub fn deserialize(s: &str) -> Result<Self, StrategyError> {
        serde_json::from_str(s).map_err(|e| {
            StrategyError::Other(format!("failed to deserialize StrategyProgress: {e}"))
        })
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

impl TryFrom<StrategyProgress> for SolveProgress {
    type Error = StrategyProgress;
    fn try_from(sp: StrategyProgress) -> Result<Self, StrategyProgress> {
        match sp {
            StrategyProgress::Default(p) => Ok(p),
            other => Err(other),
        }
    }
}

impl TryFrom<StrategyProgress> for NoObjectiveProgressData {
    type Error = StrategyProgress;
    fn try_from(sp: StrategyProgress) -> Result<Self, StrategyProgress> {
        match sp {
            StrategyProgress::NoObjective(p) => Ok(p),
            other => Err(other),
        }
    }
}

pub trait SpawnableStrategy {
    type Progress;
    fn to_strategy_kind(&self) -> StrategyKind;
    fn convert_progress(sp: StrategyProgress) -> Result<Self::Progress, StrategyProgress>;
}

impl SpawnableStrategy for DefaultStrategy {
    type Progress = SolveProgress;
    fn to_strategy_kind(&self) -> StrategyKind {
        self.clone().into()
    }
    fn convert_progress(sp: StrategyProgress) -> Result<SolveProgress, StrategyProgress> {
        sp.try_into()
    }
}

impl SpawnableStrategy for NoObjectiveStrategy {
    type Progress = NoObjectiveProgressData;
    fn to_strategy_kind(&self) -> StrategyKind {
        self.clone().into()
    }
    fn convert_progress(sp: StrategyProgress) -> Result<NoObjectiveProgressData, StrategyProgress> {
        sp.try_into()
    }
}

impl SpawnableStrategy for StrategyKind {
    type Progress = StrategyProgress;
    fn to_strategy_kind(&self) -> StrategyKind {
        self.clone()
    }
    fn convert_progress(sp: StrategyProgress) -> Result<StrategyProgress, StrategyProgress> {
        Ok(sp)
    }
}

impl StrategyKind {
    pub fn name(&self) -> &'static str {
        match self {
            StrategyKind::Default(s) => s.name(),
            StrategyKind::NoObjective(s) => s.name(),
        }
    }

    pub async fn run<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        self.run_with_callback(ctx, model, warm_start, &|_| true)
            .await
    }

    pub async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        match self {
            StrategyKind::Default(s) => {
                s.run_with_callback(ctx, model, warm_start, &|p| {
                    on_progress(StrategyProgress::Default(p))
                })
                .await
            }
            StrategyKind::NoObjective(s) => {
                s.run_with_callback(ctx, model, warm_start, &|p| {
                    on_progress(StrategyProgress::NoObjective(p))
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
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
        on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
        tag_echo: &(dyn Fn(String) -> String + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        self.solve_problem_with_echo(model.problem(), opts, on_progress, tag_echo)
            .await
    }

    pub async fn run_strategy<B, E, C>(
        &self,
        strategy: &StrategyKind,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        strategy.run(self, model, warm_start).await
    }

    pub async fn run_strategy_with_callback<B, E, C>(
        &self,
        strategy: &StrategyKind,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        strategy
            .run_with_callback(self, model, warm_start, on_progress)
            .await
    }

    pub async fn spawn_strategy<B, E, C, S: SpawnableStrategy>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        self.spawn_strategy_with_callback(strategy, model, warm_start, &|_| true)
            .await
    }

    pub async fn spawn_strategy_with_callback<B, E, C, S: SpawnableStrategy>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(Result<S::Progress, StrategyProgress>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        let (model_desc, var_order) = model.to_desc();
        let strategy_kind = strategy.to_strategy_kind();

        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));

        let noop_echo = |_: String| {};
        let echo_fn: &(dyn Fn(String) + Send + Sync) = match &self.on_echo {
            Some(f) => f.as_ref(),
            None => &noop_echo,
        };

        let raw_on_progress =
            |sp: StrategyProgress| -> bool { on_progress(S::convert_progress(sp)) };

        let raw = self
            .backend
            .run_strategy_subprocess(
                &model_desc,
                &strategy_kind,
                raw_warm_start,
                &raw_on_progress,
                echo_fn,
            )
            .await?;

        Ok(raw.into_typed(&var_order))
    }

    pub async fn spawn_strategy_with_echo<B, E, C, S: SpawnableStrategy>(
        &self,
        strategy: &S,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        on_progress: &(dyn Fn(Result<S::Progress, StrategyProgress>) -> bool + Send + Sync),
        tag_echo: &(dyn Fn(String) -> String + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData,
        E: UsableData,
        C: UsableData,
    {
        let (model_desc, var_order) = model.to_desc();
        let strategy_kind = strategy.to_strategy_kind();

        let raw_warm_start = warm_start
            .as_ref()
            .map(|hint| collomatique_ilp::config_data_to_hint(hint, &var_order));

        let echo_impl: Box<dyn Fn(String) + Send + Sync + '_> = match &self.on_echo {
            Some(ctx_echo) => Box::new(move |line| ctx_echo(tag_echo(line))),
            None => Box::new(move |line| {
                let _ = tag_echo(line);
            }),
        };

        let raw_on_progress =
            |sp: StrategyProgress| -> bool { on_progress(S::convert_progress(sp)) };

        let raw = self
            .backend
            .run_strategy_subprocess(
                &model_desc,
                &strategy_kind,
                raw_warm_start,
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
    #[serde(default)]
    pub warm_start: Option<Vec<f64>>,
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
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    struct MockBackend {
        outcome: RawSolveOutcome,
    }

    #[async_trait]
    impl SolveBackend for MockBackend {
        async fn solve_with_progress(
            &self,
            _desc: &ProblemDesc,
            _opts: SolveConfig,
            _on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
            _on_echo: &(dyn Fn(String) + Send + Sync),
        ) -> Result<RawSolveOutcome, StrategyError> {
            Ok(self.outcome.clone())
        }

        async fn run_strategy_subprocess(
            &self,
            _model_desc: &ModelDesc,
            _strategy: &StrategyKind,
            _warm_start: Option<Vec<f64>>,
            _on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
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
            _on_progress: &(dyn Fn(SolveProgress) -> bool + Send + Sync),
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
            _on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
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
        let outcome = strategy.run(&ctx, &model).await.unwrap();

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
        let outcome = ctx.run_strategy(&kind, &model, None).await.unwrap();

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
            let outcome = ctx.spawn_strategy(&kind, &model, None).await.unwrap();

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
            checker_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: false,
        };

        let progress_log: Mutex<Vec<NoObjectiveProgressData>> = Mutex::new(Vec::new());
        let outcome = strategy
            .run_with_callback(&ctx, &model, None, &|p| {
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
            checker_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: false,
        };

        let outcome = strategy.run(&ctx, &model).await.unwrap();
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
            checker_time_limit_seconds: None,
            reconstruction_time_limit_seconds: None,
            disable_logging: false,
        });
        let outcome = ctx.run_strategy(&kind, &model, None).await.unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(3.0));
    }
}
