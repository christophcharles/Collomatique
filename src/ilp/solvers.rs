pub mod dijkstra;

use super::{Config, FeasableConfig};

use std::collections::BTreeSet;

pub trait FeasabilitySolver<'a> {
    fn restore_feasability_exclude<'b>(
        &'b self,
        config: &Config,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig<'a>>
    where
        'a: 'b;

    fn restore_feasability<'b>(&'b self, config: &Config) -> Option<FeasableConfig<'a>>
    where
        'a: 'b,
    {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
