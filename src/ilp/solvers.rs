pub mod dijkstra;

use super::Config;

use std::collections::BTreeSet;

pub trait FeasabilitySolver {
    fn restore_feasability_exclude(
        &self,
        config: &Config,
        exclude_list: &BTreeSet<&Config>,
    ) -> Option<Config>;

    fn restore_feasability(&self, config: &Config) -> Option<Config> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
