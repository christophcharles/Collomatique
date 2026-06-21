use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::UsableData;
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::{
    SolveProblemOpts, SolveProgress, Strategy, StrategyContext, StrategyError, StrategyOutcome,
};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DefaultStrategy {
    pub time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

#[async_trait]
impl Strategy for DefaultStrategy {
    type Progress<V: UsableData + Send> = SolveProgress;

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        ctx.solve_model_with_progress(
            model,
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
