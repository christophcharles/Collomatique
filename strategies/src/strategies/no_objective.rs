use std::collections::HashMap;
use std::sync::Mutex;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use collomatique_ilp::{ConfigData, UsableData};
use collomatique_ilp_modeler::{InternalVar, Model};

use crate::{
    SolveProblemOpts, SolveProgress, SolveStatus, Strategy, StrategyContext, StrategyError,
    StrategyOutcome,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoObjectiveStrategy {
    pub checker_time_limit_seconds: Option<u32>,
    pub reconstruction_time_limit_seconds: Option<u32>,
    pub disable_logging: bool,
}

#[async_trait]
impl Strategy for NoObjectiveStrategy {
    type Progress<V: UsableData + Send> = NoObjectiveProgressData;

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
        // Phase 1: Solve checker problem (no objective, fast)
        let last_progress: Mutex<Option<SolveProgress>> = Mutex::new(None);

        let checker_outcome = ctx
            .solve_problem_with_progress(
                model.checker_problem(),
                SolveProblemOpts {
                    warm_start: None,
                    time_limit_seconds: self.checker_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
                &|p: SolveProgress| {
                    *last_progress.lock().unwrap() = Some(p.clone());
                    on_progress(NoObjectiveProgressData::CheckerSolve(p))
                },
            )
            .await?;

        match checker_outcome.status {
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
                    "checker solve returned error".into(),
                ));
            }
            SolveStatus::Stopped => {
                return Ok(StrategyOutcome {
                    status: SolveStatus::Stopped,
                    objective: None,
                    best_bound: None,
                    solution: None,
                });
            }
            SolveStatus::Optimal => {}
        }

        let checker_solution = checker_outcome.solution.ok_or_else(|| {
            StrategyError::SolveError("checker optimal but no solution returned".into())
        })?;

        // Phase 2: Reconstruct all variables + objective
        let base_values: HashMap<B, f64> = checker_solution
            .get_values()
            .into_iter()
            .filter_map(|(v, val)| match v {
                InternalVar::Base(b) => Some((b, val)),
                _ => None,
            })
            .collect();

        let recon_problem = model.reconstruction_problem(&base_values).map_err(|e| {
            StrategyError::SolveError(format!("failed to build reconstruction problem: {e}"))
        })?;

        let should_continue = on_progress(NoObjectiveProgressData::ObjectiveReconstruction {
            last_solve_progress: last_progress.into_inner().unwrap(),
        });
        if !should_continue {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: None,
                best_bound: None,
                solution: None,
            });
        }

        let recon_outcome = ctx
            .solve_problem(
                &recon_problem,
                SolveProblemOpts {
                    warm_start: None,
                    time_limit_seconds: self.reconstruction_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
            )
            .await?;

        let recon_solution = match recon_outcome.status {
            SolveStatus::Optimal | SolveStatus::Stopped => {
                recon_outcome.solution.ok_or_else(|| {
                    StrategyError::SolveError("reconstruction produced no solution".into())
                })?
            }
            SolveStatus::Infeasible => {
                return Err(StrategyError::SolveError(
                    "reconstruction problem is infeasible".into(),
                ));
            }
            SolveStatus::Error => {
                return Err(StrategyError::SolveError(
                    "reconstruction solve returned error".into(),
                ));
            }
        };

        // Combine base + reconstruction into complete solution
        let mut complete_values: HashMap<InternalVar<B, E>, f64> = base_values
            .into_iter()
            .map(|(b, v)| (InternalVar::Base(b), v))
            .collect();
        complete_values.extend(recon_solution.get_values());
        let complete_config = ConfigData::from(complete_values);

        Ok(StrategyOutcome {
            status: SolveStatus::Optimal,
            objective: recon_outcome.objective,
            best_bound: recon_outcome.best_bound,
            solution: Some(complete_config),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoObjectiveProgressData {
    CheckerSolve(SolveProgress),
    ObjectiveReconstruction {
        last_solve_progress: Option<SolveProgress>,
    },
}
