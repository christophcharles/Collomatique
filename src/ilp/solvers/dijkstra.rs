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
    fn restore_feasability_with_origin_and_max_steps<'a>(
        &self,
        config: &Config<'a, V>,
        origin: Option<&FeasableConfig<'a, V>>,
        mut max_steps: Option<usize>,
    ) -> Option<FeasableConfig<'a, V>> {
        use std::collections::VecDeque;

        let forbidden_config = origin.map(|x| x.inner().clone());
        let mut explored_configs: BTreeSet<Config<'_, V>> =
            forbidden_config.iter().cloned().collect();
        let mut config_queue = VecDeque::new();
        config_queue.push_back(config.clone());

        while let Some(candidate) = config_queue.pop_front() {
            if let Some(ms) = max_steps {
                if ms == 0 {
                    return None;
                } else {
                    max_steps = Some(ms - 1);
                }
            }
            if candidate.is_feasable()
                && !forbidden_config.as_ref().is_some_and(|x| *x == candidate)
            {
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
