pub mod a_star;
pub mod backtracking;
pub mod coin_cbc;
pub mod dijkstra;

use super::{Config, FeasableConfig};

use super::linexpr::VariableName;
use super::mat_repr::ProblemRepr;

pub trait FeasabilitySolver<V: VariableName, P: ProblemRepr<V>> {
    fn restore_feasability_with_origin_and_max_steps<'a>(
        &self,
        config: &Config<'a, V, P>,
        origin: Option<&FeasableConfig<'a, V, P>>,
        max_steps: Option<usize>,
    ) -> Option<FeasableConfig<'a, V, P>>;

    fn restore_feasability_with_origin<'a>(
        &self,
        config: &Config<'a, V, P>,
        origin: Option<&FeasableConfig<'a, V, P>>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.restore_feasability_with_origin_and_max_steps(config, origin, None)
    }

    fn restore_feasability_with_max_steps<'a>(
        &self,
        config: &Config<'a, V, P>,
        max_steps: Option<usize>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.restore_feasability_with_origin_and_max_steps(config, None, max_steps)
    }

    fn restore_feasability<'a>(
        &self,
        config: &Config<'a, V, P>,
    ) -> Option<FeasableConfig<'a, V, P>> {
        self.restore_feasability_with_origin_and_max_steps(config, None, None)
    }
}
