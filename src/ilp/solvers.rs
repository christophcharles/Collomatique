pub mod astar;
pub mod dijkstra;

use super::{Config, FeasableConfig};

use std::collections::BTreeSet;

pub trait FeasabilitySolver {
    fn restore_feasability_exclude<'a>(
        &self,
        config: &Config<'a>,
        exclude_list: &BTreeSet<&FeasableConfig<'a>>,
    ) -> Option<FeasableConfig<'a>>;

    fn restore_feasability<'a>(&self, config: &Config<'a>) -> Option<FeasableConfig<'a>> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
