#[cfg(test)]
mod tests;

use crate::ilp::tools::{ConfigRepr, MatRepr};
use crate::ilp::{Config, Problem};

#[derive(Debug, Clone)]
pub struct Solver<'a> {
    mat_repr: MatRepr<'a>,
}

impl<'a> Solver<'a> {
    pub fn new(problem: &'a Problem) -> Self {
        let mat_repr = MatRepr::new(problem);
        Solver { mat_repr }
    }
}

use super::FeasabilitySolver;
use std::collections::BTreeSet;

impl<'a> FeasabilitySolver for Solver<'a> {
    fn restore_feasability_exclude(
        &self,
        config: &Config,
        exclude_list: &BTreeSet<&Config>,
    ) -> Option<Config> {
        let config_repr = self.mat_repr.config(config);

        use std::collections::VecDeque;

        let mut explored_configs: BTreeSet<ConfigRepr<'_, '_>> = exclude_list
            .iter()
            .map(|x| self.mat_repr.config(x))
            .collect();
        let mut config_queue = VecDeque::new();
        config_queue.push_back(config_repr);

        while let Some(candidate) = config_queue.pop_front() {
            if candidate.is_feasable() {
                return Some(candidate.into());
            } else {
                config_queue.extend(
                    candidate
                        .neighbours()
                        .into_iter()
                        .filter(|x| !explored_configs.contains(x)),
                );
                explored_configs.insert(candidate);
            }
        }

        None
    }
}
