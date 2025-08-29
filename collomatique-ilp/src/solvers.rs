//! Solvers module
//! 
//! This module defines the main trait for solvers: [Solver].
//! 
//! It also contains the implementations of different solvers as submodules.
//! The default solver for collomatique is [coin_cbc].

#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;

use super::{Problem, FeasableConfig};

use super::UsableData;
use super::mat_repr::ProblemRepr;

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
    fn solve<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
    ) -> Option<FeasableConfig<'a, V, C, P>>;
}

/// Solver with time limit trait
/// 
/// A solver implements this trait if it supports having a time limit for solving.
/// If a solver implements this trait, a blanket implementation of the [Solver]
/// trait is automatically implemented.
pub trait SolverWithTimeLimit<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
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
    fn solve_with_time_limit<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, C, P>>;
}

impl<V: UsableData, C: UsableData, P: ProblemRepr<V>, T: SolverWithTimeLimit<V, C, P>> Solver<V, C, P> for T {
    fn solve<'a>(
            &self,
            problem: &'a Problem<V, C, P>,
        ) -> Option<FeasableConfig<'a, V, C, P>> {
        self.solve_with_time_limit(problem, None)
    }
}
