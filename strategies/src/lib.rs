mod strategies;

pub use strategies::conductor::ConductorStrategy;
pub use strategies::default::DefaultStrategy;

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use collomatique_ilp::mat_repr::ProblemRepr;
use collomatique_ilp::{ConfigData, Problem, ProblemDesc, UsableData};

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

        let solution = raw
            .solution
            .as_ref()
            .map(|sol| collomatique_ilp::solution_to_config_data(sol, &var_order));

        Ok(StrategyOutcome {
            status: raw.status,
            objective: raw.objective,
            best_bound: raw.best_bound,
            solution,
        })
    }
}

#[async_trait]
pub trait Strategy: Send + Sync {
    type Progress: Serialize + DeserializeOwned + Send + Sync + Clone;

    async fn run_with_callback<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
        on_progress: &(dyn Fn(Self::Progress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync;

    async fn run<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        self.run_with_callback(ctx, problem, &|_| true).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StrategyKind {
    Default(DefaultStrategy),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StrategyProgress {
    Default(SolveProgress),
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

impl StrategyKind {
    pub async fn run<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        self.run_with_callback(ctx, problem, &|_| true).await
    }

    pub async fn run_with_callback<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        match self {
            StrategyKind::Default(s) => {
                s.run_with_callback(ctx, problem, &|p| on_progress(StrategyProgress::Default(p)))
                    .await
            }
        }
    }
}

impl StrategyContext {
    pub async fn run_strategy<V, C, P>(
        &self,
        strategy: &StrategyKind,
        problem: &Problem<V, C, P>,
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        strategy.run(self, problem).await
    }

    pub async fn run_strategy_with_callback<V, C, P>(
        &self,
        strategy: &StrategyKind,
        problem: &Problem<V, C, P>,
        on_progress: &(dyn Fn(StrategyProgress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        strategy.run_with_callback(self, problem, on_progress).await
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyRequest {
    pub problem_desc: ProblemDesc,
    pub strategy: StrategyKind,
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
    use collomatique_ilp::{DefaultRepr, ProblemBuilder, Variable};

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
    }

    #[tokio::test]
    async fn default_strategy_returns_mock_result() {
        let problem = ProblemBuilder::<usize, (), DefaultRepr<usize>>::new()
            .set_variable(0usize, Variable::binary())
            .set_variable(1usize, Variable::binary())
            .build()
            .unwrap();

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
        let outcome = strategy.run(&ctx, &problem).await.unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.objective, Some(42.0));
        let solution = outcome.solution.unwrap();
        assert_eq!(solution.get(0usize), Some(1.0));
        assert_eq!(solution.get(1usize), Some(1.0));
    }

    #[tokio::test]
    async fn strategy_kind_dispatches_to_default() {
        let problem = ProblemBuilder::<usize, (), DefaultRepr<usize>>::new()
            .set_variable(0usize, Variable::binary())
            .build()
            .unwrap();

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
        let outcome = ctx.run_strategy(&kind, &problem).await.unwrap();

        assert_eq!(outcome.status, SolveStatus::Optimal);
        assert_eq!(outcome.solution.unwrap().get(0usize), Some(1.0));
    }
}
