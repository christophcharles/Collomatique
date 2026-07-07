use std::convert::Infallible;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::{
    SolveProblemOpts, SolveProgress, Strategy, StrategyContext, StrategyError, StrategyOutcome,
    VarOrderSerializable,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DefaultStrategy {
    pub time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

/// Per-run payload for [`DefaultStrategy`]. Empty for now; carries no
/// problem-specific data.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DefaultPayload;

impl<V: UsableData + Send> VarOrderSerializable<V> for DefaultPayload {
    type Data = DefaultPayload;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<DefaultPayload, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &DefaultPayload, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

#[async_trait]
impl Strategy for DefaultStrategy {
    type Progress<V: UsableData + Send> = SolveProgress<V>;
    type Payload<V: UsableData + Send> = DefaultPayload;

    fn name(&self) -> &'static str {
        "default"
    }

    fn ui_name(&self) -> &'static str {
        "Stratégie par défaut"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        _payload: DefaultPayload,
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
            &|line| Some(format!("[solver] {line}")),
        )
        .await
    }
}
