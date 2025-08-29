#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;

use super::{Problem, FeasableConfig};

use super::UsableData;
use super::mat_repr::ProblemRepr;

pub trait Solver<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
    fn solve<'a>(
        &self,
        problem: &'a Problem<V, C, P>,
    ) -> Option<FeasableConfig<'a, V, C, P>>;
}

pub trait SolverWithTimeLimit<V: UsableData, C: UsableData, P: ProblemRepr<V>>: Send + Sync {
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
