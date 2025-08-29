#[cfg(test)]
mod tests;

use crate::ilp::random::RandomGen;
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Optimizer<'a, S: FeasabilitySolver> {
    problem: &'a Problem,
    init_config: Config,
    solver: S,
    max_iter: u32,
}

impl<'a, S: FeasabilitySolver> Optimizer<'a, S> {
    pub fn new(problem: &'a Problem, solver: S) -> Self {
        let init_config = Config::new();

        Optimizer {
            problem,
            init_config,
            solver,
            max_iter: 10,
        }
    }

    pub fn set_init_config(&mut self, init_config: Config) {
        self.init_config = init_config;
    }

    pub fn set_max_iter(&mut self, max_iter: u32) {
        self.max_iter = max_iter;
    }

    pub fn optimize<R: RandomGen>(&self, random_gen: &mut R) -> Option<FeasableConfig> {
        use std::collections::BTreeSet;

        let mut config = self.solver.restore_feasability(&self.init_config)?;

        let eval_fn = match &self.problem.eval_fn {
            Some(e) => e,
            None => {
                return Some(config);
            }
        };
        let mut config_cost = eval_fn(&config);

        let mut lowest_config = config.clone();
        let mut lowest_cost = config_cost;

        for k in 0..self.max_iter {
            let temp = self.temp_profile(k);

            let neighbour_candidate = self
                .problem
                .random_neighbour(&Config::from(&config), random_gen);
            let exclude_list = BTreeSet::from([&config]);
            let neighbour = match self
                .solver
                .restore_feasability_exclude(&neighbour_candidate, &exclude_list)
            {
                Some(n) => n,
                None => return Some(config), // No other feasable solution? We found the optimal one!
            };

            let neighbour_cost = eval_fn(&neighbour);

            if neighbour_cost < lowest_cost {
                lowest_cost = neighbour_cost;
                lowest_config = neighbour.clone();
            }

            let acceptance = (-(neighbour_cost - config_cost) / temp).exp();

            if acceptance > random_gen.random() {
                config = neighbour;
                config_cost = neighbour_cost;
            }
        }

        Some(lowest_config)
    }
}

impl<'a, S: FeasabilitySolver> Optimizer<'a, S> {
    fn temp_profile(&self, k: u32) -> f64 {
        1000.0 * ((self.max_iter - k) as f64) / (self.max_iter as f64)
    }
}
