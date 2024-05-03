#[cfg(test)]
mod tests;

use crate::ilp::{Config, FeasableConfig};

#[derive(Debug, Clone, Default)]
pub struct Solver {}

impl Solver {
    pub fn new() -> Self {
        Solver {}
    }
}

use super::{FeasabilitySolver, VariableName};

use std::collections::BTreeSet;

impl<V: VariableName> FeasabilitySolver<V> for Solver {
    fn restore_feasability_exclude<'a>(
        &self,
        config: &Config<'a, V>,
        exclude_list: &BTreeSet<&FeasableConfig<'a, V>>,
    ) -> Option<FeasableConfig<'a, V>> {
        use std::collections::VecDeque;

        let exclude_configs: BTreeSet<Config<'a, V>> =
            exclude_list.iter().map(|x| x.inner().clone()).collect();
        let mut explored_configs = exclude_configs.clone();
        let mut config_queue = VecDeque::new();
        config_queue.push_back(config.clone());

        while let Some(candidate) = config_queue.pop_front() {
            if candidate.is_feasable() && !exclude_configs.contains(&candidate) {
                return Some(unsafe { candidate.into_feasable_unchecked() });
            } else {
                config_queue.extend(
                    candidate
                        .neighbours()
                        .into_iter()
                        .filter(|x| !explored_configs.contains(x)),
                );
                explored_configs.insert(candidate.clone());
            }
        }

        None
    }
}
