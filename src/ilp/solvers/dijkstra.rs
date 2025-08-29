use crate::ilp::tools::MatRepr;
use crate::ilp::{Config, Problem};

#[derive(Debug, Clone, PartialEq, Eq)]
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

impl<'a> FeasabilitySolver for Solver<'a> {
    fn restore_feasability(&self, config: &Config) -> Option<Config> {
        let config_repr = self.mat_repr.config(config);

        use std::collections::{BTreeSet, VecDeque};

        let mut explored_configs = BTreeSet::new();
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
                        .filter(|x| explored_configs.contains(x)),
                );
                explored_configs.insert(candidate);
            }
        }

        None
    }
}
