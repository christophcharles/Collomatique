//! What a finished solve amounts to, in the terms a person is shown.
//!
//! [SolveStatus] is what a solver reports about the problem it was handed. It
//! is not the answer to « how did it go »: the conductor calls a run `Optimal`
//! as soon as it holds any incumbent (`conductor_outcome`), and never reports
//! `Infeasible` or `Error` at all. The verdict below is that answer, computed
//! once here so the graphical interface and the scripting api give the same
//! one.

use collomatique_ilp::UsableData;

use crate::{OPTIMALITY_GAP_EPS, SolveStatus, StrategyOutcome};

#[cfg(test)]
mod tests;

/// How a solve ended, in four words
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SolveVerdict {
    /// A solution, and the proof that none is better
    Optimal,
    /// A solution, with the question of a better one left open
    Feasible,
    /// Nothing in hand: stopped before finding one, or none exists
    NoSolution,
    /// The run broke down; it may still carry the best it had found
    Error,
}

/// The verdict one finished outcome earns
///
/// Only meaningful on a *final* outcome. The gap is `|objective − bound|` and
/// so needs no `ObjectiveSense`: the bound brackets the optimum, and the
/// mid-solve sign flip that `optimum_reached` has to allow for is over by the
/// time an outcome exists.
pub fn verdict<V: UsableData>(outcome: &StrategyOutcome<V>) -> SolveVerdict {
    match outcome.status {
        SolveStatus::Error => SolveVerdict::Error,
        SolveStatus::Infeasible => SolveVerdict::NoSolution,
        SolveStatus::Optimal | SolveStatus::Stopped(_) => {
            if outcome.solution.is_none() {
                return SolveVerdict::NoSolution;
            }

            let proven = match (outcome.objective, outcome.best_bound) {
                (Some(objective), Some(bound)) => (objective - bound).abs() <= OPTIMALITY_GAP_EPS,
                // No bound is no proof.
                _ => false,
            };

            if proven {
                SolveVerdict::Optimal
            } else {
                SolveVerdict::Feasible
            }
        }
    }
}
