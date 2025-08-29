pub mod a_star;
pub mod backtracking;
pub mod dijkstra;

use super::{Config, FeasableConfig};

use super::linexpr::VariableName;

pub trait FeasabilitySolver<V: VariableName> {
    fn restore_feasability_with_origin<'a>(
        &self,
        config: &Config<'a, V>,
        origin: Option<&FeasableConfig<'a, V>>,
    ) -> Option<FeasableConfig<'a, V>>;

    fn restore_feasability<'a>(&self, config: &Config<'a, V>) -> Option<FeasableConfig<'a, V>> {
        self.restore_feasability_with_origin(config, None)
    }
}
