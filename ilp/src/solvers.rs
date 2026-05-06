//! Solvers module
//!
//! This module defines the main traits for solvers:
//! [Solver] builds a [SolverModel] from a [Problem],
//! and [SolverModel::solve] finds an optimal solution.
//!
//! It also contains the implementations of different solvers as submodules.
//! The default solver for collomatique is [coin_cbc].

#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;
#[cfg(feature = "good_lp")]
pub mod good_lp;

use super::{ConfigData, FeasibleConfig, Problem};

use super::UsableData;
use super::mat_repr::ProblemRepr;

/// Solver trait
///
/// A solver translates a [Problem] into a backend-specific [SolverModel]
/// via [Solver::build_model]. The model can then be solved
/// via [SolverModel::solve].
pub trait Solver<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
    /// The backend-specific model type.
    type Model<'a>: SolverModel<'a, V, C, P>
    where
        V: 'a,
        C: 'a,
        P: 'a;

    /// Build a model from a problem.
    ///
    /// This translates the problem's variables, constraints, and
    /// objective into the backend's internal representation.
    /// The returned model is ready to be solved.
    fn build_model<'a>(&self, problem: &'a Problem<V, C, P>) -> Self::Model<'a>;
}

/// A model ready to be solved.
///
/// Produced by [Solver::build_model]. Call [SolverModel::solve]
/// to find an optimal solution. The model is consumed in the process.
pub trait SolverModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
    /// Solve the model without any time limit.
    ///
    /// Returns `None` if the problem is infeasible.
    fn solve(self) -> Option<FeasibleConfig<'a, V, C, P>>;
}

/// Result of [TimeLimitSolverModel::solve_with_time_limit].
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
    pub config: Option<FeasibleConfig<'a, V, C, P>>,

    /// Whether the time limit was reached.
    ///
    /// If the time limit is reached, the solution might not be optimal.
    pub time_limit_reached: bool,
}

/// A model that supports solving with a time limit.
///
/// This is a supertrait of [SolverModel]: any model that
/// supports time limits can also be solved without one.
pub trait TimeLimitSolverModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>:
    SolverModel<'a, V, C, P>
{
    /// Solve the model with a time limit.
    ///
    /// If the time limit is reached, the best solution found
    /// so far is returned (which may be `None`).
    ///
    /// You can check this by inspecting [TimeLimitSolution::time_limit_reached].
    fn solve_with_time_limit(self, time_limit_in_seconds: u32) -> TimeLimitSolution<'a, V, C, P>;
}

/// A solver that supports warm starting from an initial solution hint.
///
/// Warm starting provides the solver with a known (or candidate) solution
/// to use as a starting point. This can significantly speed up MIP solving
/// by giving the solver a good incumbent early in the search.
///
/// The hint is a [ConfigData] rather than a [Config](super::Config) so that
/// solutions from different (but related) problem instances can be reused
/// — for example, when re-solving after modifying constraints.
///
/// The hint is best-effort: variables missing from the hint are
/// solver-dependent (typically defaulting to 0), and the solver may
/// ignore the hint entirely if it is not useful.
///
/// This trait is on the [Solver] rather than the [SolverModel] to
/// guarantee the hint is applied at construction time, before any
/// solving occurs.
pub trait WarmSolver<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Solver<V, C, P> {
    /// Build a model from a problem, seeded with an initial solution hint.
    ///
    /// Behaves like [Solver::build_model] but additionally injects the
    /// hint as an initial solution for the solver backend.
    fn build_warm_model<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
        hint: &ConfigData<V>,
    ) -> Self::Model<'a>;
}
