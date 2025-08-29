//! Solvers module
//!
//! This module defines the main trait for solvers: [Solver].
//!
//! It also contains the implementations of different solvers as submodules.
//! The default solver for collomatique is [coin_cbc].

#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;

use super::{FeasableConfig, Problem};

use super::mat_repr::ProblemRepr;
use super::UsableData;

/// Solver trait
///
/// Any solver should implement this trait. It provides a single method
/// [Solver::solve] that takes a problem and returns either a solution
/// or, if the problem is not solvable, `None`.
pub trait Solver<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
    /// Solves the problem.
    ///
    /// This is the main function of the [Solver] trait.
    ///
    /// Any solver should this function.
    /// It takes a problem and returns either a solution
    /// or, if the problem is not solvable, `None`.
    ///
    /// If you want to solve with a time limit, use the [SolverWithTimeLimit] trait.
    fn solve<'a>(&self, problem: &'a Problem<V, C, P>) -> Option<FeasableConfig<'a, V, C, P>>;
}

/// Result of [SolverWithTimeLimit::solve_with_time_limit].
///
/// It contains the solution if one was found but also the reason
/// for returning.
///
/// If [TimeLimitSolution::time_limit_reached] is `true`, this means
/// the time limit was reached and the solution might not be optimal.
///
/// If [TimeLimitSolution::time_limit_reached] is `false`, this means
/// the time limit was not reached and therefore the solution is indeed optimal.
pub struct TimeLimitSolution<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    /// The actual solution found by the solver
    pub config: FeasableConfig<'a, V, C, P>,

    /// Whether the time limit was reached.
    ///
    /// If the time limit is reached, the solution might not be optimal.
    pub time_limit_reached: bool,
}

/// Solver with time limit trait
///
/// A solver implements this trait if it supports having a time limit for solving.
/// If a solver implements this trait, a blanket implementation of the [Solver]
/// trait is automatically implemented.
pub trait SolverWithTimeLimit<V: UsableData, C: UsableData, P: ProblemRepr<V>>:
    Send + Sync
{
    /// Solves the problem with the time problem.
    ///
    /// This is the main function of the [SolverWithTimeLimit] trait.
    ///
    /// It takes a problem and returns either a solution
    /// or, if the problem is not solvable, `None`.
    ///
    /// If the time limit is reached, the best solution *so far* is returned.
    /// So if no solution was found, it still returns `None`. If a solution is found
    /// it is returned. However, it might not be optimal.
    ///
    /// You can check this by inspecting [TimeLimitSolution::time_limit_reached].
    fn solve_with_time_limit<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<TimeLimitSolution<'a, V, C, P>>;
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>, T: SolverWithTimeLimit<V, C, P>>
    Solver<V, C, P> for T
{
    fn solve<'a>(&self, problem: &'a Problem<V, C, P>) -> Option<FeasableConfig<'a, V, C, P>> {
        self.solve_with_time_limit(problem, None).map(|x| x.config)
    }
}
