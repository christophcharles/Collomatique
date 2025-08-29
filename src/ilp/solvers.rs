#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;
#[cfg(feature = "highs")]
pub mod highs;

use super::{Config, FeasableConfig, Problem};

use super::linexpr::VariableName;
use super::mat_repr::ProblemRepr;

pub trait FeasabilitySolver<V: VariableName, P: ProblemRepr<V>>: Send + Sync {
    fn find_closest_solution<'a>(
        &self,
        config: &Config<'a, V, P>,
    ) -> Option<FeasableConfig<'a, V, P>>;

    fn solve<'a>(
        &self,
        problem: &'a Problem<V, P>,
        minimize_objective: bool,
    ) -> Option<FeasableConfig<'a, V, P>>;
}
