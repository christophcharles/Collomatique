use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::mat_repr::ProblemRepr;
use collomatique_ilp::{Problem, UsableData};

use crate::{
    SolveProblemOpts, SolveProgress, Strategy, StrategyContext, StrategyError, StrategyOutcome,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConductorStrategy {
    pub time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

#[async_trait]
impl Strategy for ConductorStrategy {
    type Progress = SolveProgress;

    async fn run_with_callback<V, C, P>(
        &self,
        ctx: &StrategyContext,
        problem: &Problem<V, C, P>,
        on_progress: &(dyn Fn(Self::Progress) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<V>, StrategyError>
    where
        V: UsableData + Send,
        C: UsableData + Send,
        P: ProblemRepr<V> + Send + Sync,
    {
        ctx.solve_problem_with_progress(
            problem,
            SolveProblemOpts {
                warm_start: None,
                time_limit_seconds: self.time_limit_seconds,
                disable_logging: self.disable_logging,
            },
            on_progress,
        )
        .await
    }
}
