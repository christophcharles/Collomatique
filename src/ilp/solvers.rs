pub mod dijkstra;

use super::{Config, FeasableConfig};

use std::collections::BTreeSet;

pub trait FeasabilitySolver {
    fn restore_feasability_exclude(
        &self,
        config: &Config,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig>;

    fn restore_feasability(&self, config: &Config) -> Option<FeasableConfig> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
