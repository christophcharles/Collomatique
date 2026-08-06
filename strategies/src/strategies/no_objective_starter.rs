use std::convert::Infallible;
use std::fmt;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::strategies::default::{DefaultPayload, DefaultStrategy};
use crate::strategies::no_objective::{NoObjectivePayload, NoObjectiveStrategy};
use crate::{
    NoObjectiveProgressData, SolveProgress, SolveProgressData, SolveStatus, StopReason, Strategy,
    StrategyContext, StrategyError, StrategyOutcome, VarOrderSerializable,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoObjectiveStarterStrategy {
    pub no_objective: NoObjectiveStrategy,
    pub default: DefaultStrategy,
}

/// Per-run payload for [`NoObjectiveStarterStrategy`]. Empty for now.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NoObjectiveStarterPayload;

impl<V: UsableData + Send> VarOrderSerializable<V> for NoObjectiveStarterPayload {
    type Data = NoObjectiveStarterPayload;
    type Error = Infallible;
    fn into_data(&self, _var_order: &[V]) -> Result<NoObjectiveStarterPayload, Infallible> {
        Ok(self.clone())
    }
    fn from_data(data: &NoObjectiveStarterPayload, _var_order: &[V]) -> Result<Self, Infallible> {
        Ok(data.clone())
    }
}

#[async_trait]
impl Strategy for NoObjectiveStarterStrategy {
    type Progress<V: UsableData + Send> = NoObjectiveStarterProgress<V>;
    type Payload<B: UsableData + Send, E: UsableData + Send> = NoObjectiveStarterPayload;

    fn name(&self) -> &'static str {
        "no-obj-starter"
    }

    fn ui_name(&self) -> &'static str {
        "Résolution avec préamorçage"
    }

    async fn run_with_callback<B, E, C>(
        &self,
        ctx: &StrategyContext,
        model: &Model<B, E, C>,
        warm_start: Option<ConfigData<InternalVar<B, E>>>,
        _payload: NoObjectiveStarterPayload,
        on_progress: &(dyn Fn(Self::Progress<InternalVar<B, E>>) -> bool + Send + Sync),
    ) -> Result<StrategyOutcome<InternalVar<B, E>>, StrategyError>
    where
        B: UsableData + Send,
        E: UsableData + Send,
        C: UsableData + Send,
    {
        let no_obj_outcome = self
            .no_objective
            .run_with_callback(
                ctx,
                model,
                warm_start,
                NoObjectivePayload::default(),
                &|p| on_progress(NoObjectiveStarterProgress::Starter(p)),
            )
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
            SolveStatus::Stopped(reason) => {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Stopped(reason),
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
        let hint_objective = no_obj_outcome.objective.ok_or_else(|| {
            StrategyError::SolveError("no-objective optimal but no objective returned".into())
        })?;

        let should_continue = on_progress(NoObjectiveStarterProgress::HintFound {
            config: hint.clone(),
            objective: hint_objective,
        });
        if !should_continue {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped(StopReason::Callback),
                objective: no_obj_outcome.objective,
                best_bound: no_obj_outcome.best_bound,
                solution: Some(hint),
            });
        }

        self.default
            .run_with_callback(ctx, model, Some(hint), DefaultPayload::default(), &|p| {
                on_progress(NoObjectiveStarterProgress::Default(p))
            })
            .await
    }
}

#[derive(Debug, Clone)]
pub enum NoObjectiveStarterProgress<V: UsableData + Send> {
    Starter(NoObjectiveProgressData),
    HintFound {
        config: ConfigData<V>,
        objective: f64,
    },
    Default(SolveProgress<V>),
}

impl<V: UsableData + Send> fmt::Display for NoObjectiveStarterProgress<V> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoObjectiveStarterProgress::Starter(p) => write!(f, "[starter] {p}"),
            NoObjectiveStarterProgress::HintFound { objective, .. } => {
                write!(
                    f,
                    "Hint found (objective={objective:.4})! Starting default strategy with warm start..."
                )
            }
            NoObjectiveStarterProgress::Default(p) => {
                write!(f, "[default solver progress] {p}")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoObjectiveStarterProgressData {
    Starter(NoObjectiveProgressData),
    HintFound { config: Vec<f64>, objective: f64 },
    Default(SolveProgressData),
}

impl fmt::Display for NoObjectiveStarterProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoObjectiveStarterProgressData::Starter(p) => write!(f, "[starter] {p}"),
            NoObjectiveStarterProgressData::HintFound { objective, .. } => {
                write!(
                    f,
                    "Hint found (objective={objective:.4})! Starting default strategy with warm start..."
                )
            }
            NoObjectiveStarterProgressData::Default(p) => {
                write!(f, "[default solver progress] {p}")
            }
        }
    }
}

impl<V: UsableData + Send> NoObjectiveStarterProgress<V> {
    /// Erase the typed progress into its serializable form, encoding the hint config
    /// against `var_order`.
    pub fn into_data(self, var_order: &[V]) -> NoObjectiveStarterProgressData {
        match self {
            NoObjectiveStarterProgress::Starter(d) => NoObjectiveStarterProgressData::Starter(d),
            NoObjectiveStarterProgress::HintFound { config, objective } => {
                let raw = collomatique_ilp::config_data_to_hint(&config, var_order);
                NoObjectiveStarterProgressData::HintFound {
                    config: raw,
                    objective,
                }
            }
            NoObjectiveStarterProgress::Default(p) => {
                NoObjectiveStarterProgressData::Default(p.into_data(var_order))
            }
        }
    }
}

impl NoObjectiveStarterProgressData {
    /// Reconstruct the typed progress, turning the raw hint vector back into a
    /// [`ConfigData<V>`] keyed by `var_order`.
    pub fn into_typed<V: UsableData + Send>(
        self,
        var_order: &[V],
    ) -> NoObjectiveStarterProgress<V> {
        match self {
            NoObjectiveStarterProgressData::Starter(d) => NoObjectiveStarterProgress::Starter(d),
            NoObjectiveStarterProgressData::HintFound { config, objective } => {
                let config = collomatique_ilp::solution_to_config_data(&config, var_order);
                NoObjectiveStarterProgress::HintFound { config, objective }
            }
            NoObjectiveStarterProgressData::Default(p) => {
                NoObjectiveStarterProgress::Default(p.into_typed(var_order))
            }
        }
    }
}

impl<V: UsableData + Send> VarOrderSerializable<V> for NoObjectiveStarterProgress<V> {
    type Data = NoObjectiveStarterProgressData;
    type Error = Infallible;
    fn into_data(&self, var_order: &[V]) -> Result<NoObjectiveStarterProgressData, Infallible> {
        Ok(NoObjectiveStarterProgress::into_data(
            self.clone(),
            var_order,
        ))
    }
    fn from_data(
        data: &NoObjectiveStarterProgressData,
        var_order: &[V],
    ) -> Result<Self, Infallible> {
        Ok(NoObjectiveStarterProgressData::into_typed(
            data.clone(),
            var_order,
        ))
    }
}
