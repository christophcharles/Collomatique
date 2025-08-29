#[cfg(test)]
mod tests;

use crate::ilp::tools::{ConfigRepr, MatRepr};
use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Solver<'a> {
    problem: &'a Problem,
    mat_repr: MatRepr<'a>,
}

impl<'a> Solver<'a> {
    pub fn new(problem: &'a Problem) -> Self {
        let mat_repr = MatRepr::new(problem);
        Solver { problem, mat_repr }
    }
}

use super::FeasabilitySolver;
use std::collections::BTreeSet;

impl<'a> FeasabilitySolver<'a> for Solver<'a> {
    fn restore_feasability_exclude(
        &self,
        config: &Config,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig<'a>> {
        let config_repr = self.mat_repr.config(config);

        use std::collections::VecDeque;

        let exclude_configs: BTreeSet<ConfigRepr<'_, '_>> = exclude_list
            .iter()
            .map(|x| self.mat_repr.config(&Config::from(*x)))
            .collect();
        let mut explored_configs = exclude_configs.clone();
        let mut config_queue = VecDeque::new();
        config_queue.push_back(config_repr);

        while let Some(candidate) = config_queue.pop_front() {
            if candidate.is_feasable() && !exclude_configs.contains(&candidate) {
                return Some(
                    self.problem
                        .into_feasable(&candidate.into())
                        .expect("Solution should be feasable"),
                );
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
