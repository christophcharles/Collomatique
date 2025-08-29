pub mod a_star;
pub mod dijkstra;

use super::{Config, FeasableConfig};

use std::collections::BTreeSet;

use super::linexpr::VariableName;

pub trait FeasabilitySolver<V: VariableName> {
    fn restore_feasability_exclude<'a>(
        &self,
        config: &Config<'a, V>,
        exclude_list: &BTreeSet<&FeasableConfig<'a, V>>,
    ) -> Option<FeasableConfig<'a, V>>;

    fn restore_feasability<'a>(&self, config: &Config<'a, V>) -> Option<FeasableConfig<'a, V>> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
