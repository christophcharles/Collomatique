pub mod astar;
pub mod dijkstra;

use super::ndtools::{ConfigRepr, FeasableConfigRepr};

use std::collections::BTreeSet;

pub trait FeasabilitySolver {
    fn restore_feasability_exclude<'a>(
        &self,
        config: &ConfigRepr<'a>,
        exclude_list: &BTreeSet<&FeasableConfigRepr<'a>>,
    ) -> Option<FeasableConfigRepr<'a>>;

    fn restore_feasability<'a>(&self, config: &ConfigRepr<'a>) -> Option<FeasableConfigRepr<'a>> {
        let exclude_list = BTreeSet::new();
        self.restore_feasability_exclude(config, &exclude_list)
    }
}
