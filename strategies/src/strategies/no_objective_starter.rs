use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::strategies::default::DefaultStrategy;
use crate::strategies::no_objective::NoObjectiveStrategy;
use crate::{
    NoObjectiveProgressData, SolveProgress, SolveProgressData, SolveStatus, Strategy,
    StrategyContext, StrategyError, StrategyOutcome,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoObjectiveStarterStrategy {
    pub no_objective: NoObjectiveStrategy,
    pub default: DefaultStrategy,
}

#[async_trait]
impl Strategy for NoObjectiveStarterStrategy {
    type Progress<V: UsableData + Send> = NoObjectiveStarterProgress<V>;

    fn name(&self) -> &'static str {
        "no-obj-starter"
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
        let no_obj_outcome = self
            .no_objective
            .run_with_callback(ctx, model, warm_start, &|p| {
                on_progress(NoObjectiveStarterProgress::Starter(p))
            })
            .await?;

        match no_obj_outcome.status {
            SolveStatus::Infeasible => {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Infeasible,
                    objective: None,
                    best_bound: None,
                    solution: None,
                });
            }
            SolveStatus::Error => {
                return Err(StrategyError::SolveError(
                    "no-objective solve returned error".into(),
                ));
            }
            SolveStatus::Stopped => {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Stopped,
                    objective: no_obj_outcome.objective,
                    best_bound: no_obj_outcome.best_bound,
                    solution: no_obj_outcome.solution,
                });
            }
            SolveStatus::Optimal => {}
        }

        let hint = no_obj_outcome.solution.ok_or_else(|| {
            StrategyError::SolveError("no-objective optimal but no solution returned".into())
        })?;

        let should_continue = on_progress(NoObjectiveStarterProgress::HintFound(hint.clone()));
        if !should_continue {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: no_obj_outcome.objective,
                best_bound: no_obj_outcome.best_bound,
                solution: Some(hint),
            });
        }

        self.default
            .run_with_callback(ctx, model, Some(hint), &|p| {
                on_progress(NoObjectiveStarterProgress::Default(p))
            })
            .await
    }
}

#[derive(Debug, Clone)]
pub enum NoObjectiveStarterProgress<V: UsableData + Send> {
    Starter(NoObjectiveProgressData),
    HintFound(ConfigData<V>),
    Default(SolveProgress<V>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoObjectiveStarterProgressData {
    Starter(NoObjectiveProgressData),
    HintFound(Vec<f64>),
    Default(SolveProgressData),
}

impl fmt::Display for NoObjectiveStarterProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoObjectiveStarterProgressData::Starter(p) => write!(f, "[starter] {p}"),
            NoObjectiveStarterProgressData::HintFound(_) => {
                write!(
                    f,
                    "Hint found! Starting default strategy with warm start..."
                )
            }
            NoObjectiveStarterProgressData::Default(p) => {
                write!(f, "[default solver progress] {p}")
            }
        }
    }
}
