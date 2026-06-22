use std::collections::HashMap;
use std::fmt;

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

    fn name(&self) -> &'static str {
        "no-obj"
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
        // Phase 1: Solve checker problem (no objective, fast)
        let checker_outcome = ctx
            .solve_problem_with_echo(
                model.checker_problem(),
                SolveProblemOpts {
                    warm_start,
                    time_limit_seconds: self.checker_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
                &|p| on_progress(NoObjectiveProgressData::CheckerSolve((&p).into())),
                &|line| format!("[checker solver] {line}"),
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

        let should_continue = on_progress(NoObjectiveProgressData::SolutionFound);
        if !should_continue {
            return Ok(StrategyOutcome {
                status: SolveStatus::Stopped,
                objective: checker_outcome.objective,
                best_bound: checker_outcome.best_bound,
                solution: None,
            });
        }

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

        let recon_outcome = ctx
            .solve_problem_with_echo(
                &recon_problem,
                SolveProblemOpts {
                    warm_start: None,
                    time_limit_seconds: self.reconstruction_time_limit_seconds,
                    disable_logging: self.disable_logging,
                },
                &move |p| {
                    on_progress(NoObjectiveProgressData::ObjectiveReconstruction(
                        (&p).into(),
                    ))
                },
                &|line| format!("[reconstruction solver] {line}"),
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

/// Scalar solve statistics for a NoObjective sub-solve.
///
/// Unlike [`SolveProgressData`](crate::SolveProgressData), this carries no incumbent: the
/// incumbents produced by the checker and reconstruction sub-solves are not meaningful to the
/// caller (a checker incumbent covers only base+checker variables; a reconstruction incumbent is
/// expressed in the sub-problem's coordinate system), so they are pruned here by construction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NoObjectiveSolveProgress {
    pub best_obj: f64,
    pub best_bound: f64,
    pub node_count: u64,
    pub solutions_found: u64,
}

impl<V: UsableData> From<&SolveProgress<V>> for NoObjectiveSolveProgress {
    fn from(p: &SolveProgress<V>) -> Self {
        Self {
            best_obj: p.best_obj,
            best_bound: p.best_bound,
            node_count: p.node_count,
            solutions_found: p.solutions_found,
        }
    }
}

impl fmt::Display for NoObjectiveSolveProgress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "obj={:.4} bound={:.4} nodes={} solutions={}",
            self.best_obj, self.best_bound, self.node_count, self.solutions_found
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum NoObjectiveProgressData {
    CheckerSolve(NoObjectiveSolveProgress),
    SolutionFound,
    ObjectiveReconstruction(NoObjectiveSolveProgress),
}

impl fmt::Display for NoObjectiveProgressData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NoObjectiveProgressData::CheckerSolve(p) => write!(f, "[checker solver progress] {p}"),
            NoObjectiveProgressData::SolutionFound => {
                write!(f, "Solution found! Reconstructing full variable set...")
            }
            NoObjectiveProgressData::ObjectiveReconstruction(p) => {
                write!(f, "[reconstruction solver progress] {p}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checker_progress_round_trips_without_incumbent() {
        let progress = NoObjectiveProgressData::CheckerSolve(NoObjectiveSolveProgress {
            best_obj: 1.5,
            best_bound: 0.5,
            node_count: 7,
            solutions_found: 2,
        });

        let json = serde_json::to_string(&progress).unwrap();
        // The dedicated type has no incumbent concept at all.
        assert!(!json.contains("incumbent"));

        let restored: NoObjectiveProgressData = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, progress);
    }
}
