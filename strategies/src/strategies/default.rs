use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData};
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
    type Progress<V: UsableData + Send> = SolveProgress<V>;

    fn name(&self) -> &'static str {
        "default"
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
        ctx.solve_model_with_echo(
            model,
            SolveProblemOpts {
                warm_start,
                time_limit_seconds: self.time_limit_seconds,
                disable_logging: self.disable_logging,
            },
            on_progress,
            &|line| format!("[solver] {line}"),
        )
        .await
    }
}
