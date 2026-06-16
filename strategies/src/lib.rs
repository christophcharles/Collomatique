use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

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
    pub solution: Option<ConfigData<V>>,
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
    async fn solve(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
    ) -> Result<RawSolveOutcome, StrategyError>;
}

pub struct StrategyContext {
    backend: Arc<dyn SolveBackend>,
}

impl StrategyContext {
    pub fn new(backend: Arc<dyn SolveBackend>) -> Self {
        Self { backend }
    }

    pub async fn solve(
        &self,
        desc: &ProblemDesc,
        opts: SolveConfig,
    ) -> Result<RawSolveOutcome, StrategyError> {
        self.backend.solve(desc, opts).await
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

        let raw = self.solve(&desc, solve_config).await?;

        let solution = raw
            .solution
            .as_ref()
            .map(|sol| collomatique_ilp::solution_to_config_data(sol, &var_order));

        Ok(StrategyOutcome {
            status: raw.status,
            objective: raw.objective,
            solution,
        })
    }
}

#[async_trait]
pub trait Strategy: Send + Sync {
    async fn run<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync;
}
