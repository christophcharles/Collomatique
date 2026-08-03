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
#[cfg(feature = "collo_cbc")]
pub mod collo_cbc;
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
pub trait SolverModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send {
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
    /// Solve the model, optionally bounded by `time_limit`.
    ///
    /// If the time limit is reached, the best solution found
    /// so far is returned (which may be `None`).
    ///
    /// A [TimeLimit::none](collomatique_time::TimeLimit::none) solves without
    /// any bound — equivalent to [SolverModel::solve], but returning a
    /// [TimeLimitSolution] (whose [time_limit_reached](TimeLimitSolution::time_limit_reached)
    /// is then always `false`).
    ///
    /// You can check whether the limit was reached by inspecting
    /// [TimeLimitSolution::time_limit_reached].
    fn solve_with_time_limit(
        self,
        time_limit: collomatique_time::TimeLimit,
    ) -> TimeLimitSolution<'a, V, C, P>;
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

/// Result of [CallbackSolverModel::solve_with_callback].
///
/// Similar to [TimeLimitSolution] but does not interpret *why* the solve
/// stopped — only whether the callback caused it.
pub struct CallbackSolution<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    /// The best feasible solution found, if any.
    pub config: Option<FeasibleConfig<'a, V, C, P>>,
    /// Whether the solve was stopped by the callback returning `false`.
    /// If `false`, the solve completed normally (optimal or infeasible).
    pub stopped_by_callback: bool,
}

/// A model that supports solving with a progress callback.
///
/// The callback is called periodically during solving and when new solutions
/// are found. It returns `true` to continue solving, `false` to stop.
///
/// The associated type [CallbackSolverModel::Progress] is solver-specific:
/// each backend exposes whatever progress data it can actually provide
/// (e.g. best objective, bound, node count, incumbent solution).
pub trait CallbackSolverModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>:
    SolverModel<'a, V, C, P>
{
    /// Solver-specific progress information passed to the callback.
    type Progress;

    /// Solve the model with a progress callback.
    ///
    /// The callback receives a solver-specific [Self::Progress] reference
    /// and returns `true` to continue, `false` to stop.
    fn solve_with_callback<F>(self, callback: F) -> CallbackSolution<'a, V, C, P>
    where
        F: FnMut(&Self::Progress) -> bool;
}

/// Why a solve was cut short.
///
/// Shared by every layer that reports a stopped solve: the per-layer status
/// enums carry it as the payload of their `Stopped` variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StopReason {
    /// The progress callback (or, downstream, the control channel) asked to stop.
    Callback,
    /// The time limit counted from the start of the solve ran out.
    TimeLimit,
    /// The time limit counted from the first feasible incumbent ran out.
    IncumbentTimeLimit,
}

/// Result of [IncumbentTimeLimitSolverModel::solve_with_time_limits].
pub struct TimeLimitsSolution<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>> {
    /// The best feasible solution found, if any.
    pub config: Option<FeasibleConfig<'a, V, C, P>>,

    /// `None` if the solve ran to completion (optimal or infeasible),
    /// otherwise why it was cut short.
    pub stopped: Option<StopReason>,
}

/// A model that supports a time limit counted from the *first incumbent*,
/// alongside the usual limit counted from the start of the solve.
///
/// The two limits are independent and compose: the solve stops at
/// `min(start + time_limit, first_incumbent + incumbent_time_limit)`.
/// Either limit can be [TimeLimit::none](collomatique_time::TimeLimit::none),
/// which disables that side. With both none, this behaves exactly like
/// [CallbackSolverModel::solve_with_callback].
pub trait IncumbentTimeLimitSolverModel<'a, V: UsableData, C: UsableData, P: ProblemRepr<V>>:
    CallbackSolverModel<'a, V, C, P>
{
    /// Solve the model, bounded by both time limits and by `callback`.
    ///
    /// The callback behaves as in [CallbackSolverModel::solve_with_callback]:
    /// it receives a solver-specific [Progress](CallbackSolverModel::Progress)
    /// reference and returns `true` to continue, `false` to stop.
    ///
    /// Both deadlines are cooperative: they are checked when the solver fires
    /// a progress event, exactly like the callback itself. If the backend goes
    /// silent, the stop happens at the next event.
    ///
    /// [TimeLimitsSolution::stopped] tells whether — and why — the solve was
    /// cut short. A stop asked for by `callback` is reported as
    /// [StopReason::Callback], even if a deadline passed at the same event.
    fn solve_with_time_limits<F>(
        self,
        time_limit: collomatique_time::TimeLimit,
        incumbent_time_limit: collomatique_time::TimeLimit,
        callback: F,
    ) -> TimeLimitsSolution<'a, V, C, P>
    where
        F: FnMut(&Self::Progress) -> bool;
}

pub trait ProgressBounds {
    fn best_bound(&self) -> f64;
    /// The objective of the current incumbent, or `None` if no incumbent has
    /// been found yet. An objective only exists as a property of an incumbent.
    fn best_objective(&self) -> Option<f64>;
}

pub trait ProgressStats {
    fn nodes(&self) -> u64;
    fn solutions(&self) -> u64;
}

pub struct IncumbentInfo {
    pub objective: f64,
    pub feasible: bool,
}

pub trait ProgressIncumbentInfo {
    fn incumbent_info(&self) -> Option<&IncumbentInfo>;
}

/// Progress data that can expose the current incumbent's variable assignment.
pub trait ProgressIncumbentData<V: UsableData> {
    /// The variable assignment of the most recent incumbent, if any.
    fn incumbent_data(&self) -> Option<&ConfigData<V>>;
}
