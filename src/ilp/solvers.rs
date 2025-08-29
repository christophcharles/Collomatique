#[cfg(feature = "coin_cbc")]
pub mod coin_cbc;
#[cfg(feature = "highs")]
pub mod highs;

use super::{Config, FeasableConfig};

use super::linexpr::VariableName;
use super::mat_repr::ProblemRepr;

pub trait FeasabilitySolver<V: VariableName, P: ProblemRepr<V>>: Send + Sync {
    fn find_closest_solution_with_time_limit<'a>(
        &self,
        config: &Config<'a, V, P>,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, P>>;

    fn find_closest_solution<'a>(
        &self,
        config: &Config<'a, V, P>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.find_closest_solution_with_time_limit(config, None)
    }

    fn solve<'a>(
        &self,
        config_hint: &Config<'a, V, P>,
        minimize_objective: bool,
        time_limit_in_seconds: Option<u32>,
    ) -> Option<FeasableConfig<'a, V, P>>;
}
