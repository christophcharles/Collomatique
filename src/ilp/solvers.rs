pub mod dijkstra;

use super::{Config, FeasableConfig};

use std::collections::BTreeSet;

pub trait FeasabilitySolver<'a> {
    fn restore_feasability_exclude(
        &self,
        config: &Config,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig<'a>>;

    fn restore_feasability(&self, config: &Config) -> Option<FeasableConfig<'a>> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
