#[cfg(test)]
mod tests;

use crate::ilp::ndtools::{ConfigRepr, MatRepr};
use crate::ilp::{Config, FeasableConfig, Problem};

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

impl<'a> FeasabilitySolver<'a> for Solver<'a> {
    fn restore_feasability_exclude<'b>(
        &'b self,
        config: &Config,
        exclude_list: &BTreeSet<&FeasableConfig>,
    ) -> Option<FeasableConfig<'a>>
    where
        'a: 'b,
    {
        let config_repr = self.mat_repr.config(config)?;

        use std::collections::VecDeque;

        let exclude_configs: BTreeSet<ConfigRepr<'_, '_>> = exclude_list
            .iter()
            .map(|x| self.mat_repr.config(x.inner()).unwrap())
            .collect();
        let mut explored_configs = exclude_configs.clone();
        let mut config_queue = VecDeque::new();
        config_queue.push_back(config_repr);

        while let Some(candidate) = config_queue.pop_front() {
            if candidate.is_feasable() && !exclude_configs.contains(&candidate) {
                return Some(unsafe { Config::from(candidate).into_feasable_unchecked() });
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
