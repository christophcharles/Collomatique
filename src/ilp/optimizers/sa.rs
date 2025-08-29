#[cfg(test)]
mod tests;

use crate::ilp::random::RandomGen;
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, FeasableConfig, Problem};

#[derive(Debug, Clone)]
pub struct Optimizer<'a> {
    problem: &'a Problem,
    init_config: Config,
}

impl<'a> Optimizer<'a> {
    pub fn new(problem: &'a Problem) -> Self {
        let init_config = Config::new();

        Optimizer {
            problem,
            init_config,
        }
    }

    pub fn set_init_config(&mut self, init_config: Config) {
        self.init_config = init_config;
    }

    pub fn iterate<'b, 'c, R: RandomGen, S: FeasabilitySolver<'a>>(
        &'b self,
        solver: S,
        random_gen: &'c mut R,
    ) -> OptimizerIterator<'a, 'b, 'c, R, S> {
        OptimizerIterator {
            optimizer: self,
            random_gen,
            solver,
            previous_config: None,
            current_config: self.init_config.clone(),
            k: 0,
        }
    }
}

#[derive(Debug)]
pub struct OptimizerIterator<'b, 'a: 'b, 'c, R: RandomGen, S: FeasabilitySolver<'a>> {
    optimizer: &'b Optimizer<'a>,
    solver: S,
    random_gen: &'c mut R,

    previous_config: Option<(FeasableConfig<'a>, f64)>,
    current_config: Config,

    k: usize,
}

impl<'b, 'a: 'b, 'c, R: RandomGen, S: FeasabilitySolver<'a>> Iterator
    for OptimizerIterator<'b, 'a, 'c, R, S>
{
    type Item = (FeasableConfig<'a>, f64);

    fn next(&mut self) -> Option<Self::Item> {
        use std::collections::BTreeSet;
        let exclude_list = match &self.previous_config {
            Some((c, _)) => BTreeSet::from([c]),
            None => BTreeSet::new(),
        };

        // If we can't restore then the iterator stops
        // So "None" should be propagated upwards
        let config = self
            .solver
            .restore_feasability_exclude(&self.current_config, &exclude_list)?;

        let config_cost = (self.optimizer.problem.eval_fn)(&config);

        let acceptance = match self.previous_config {
            Some((_, old_cost)) => {
                let temp = Self::temp_profile(1000, self.k);
                (-(config_cost - old_cost) / temp).exp()
            }
            None => 1.0,
        };

        self.current_config = self
            .optimizer
            .problem
            .random_neighbour(&Config::from(&config), self.random_gen);
        self.k += 1;
        if acceptance >= self.random_gen.random() {
            self.previous_config = Some((config, config_cost));
        }
        self.previous_config.clone()
    }
}

impl<'b, 'a: 'b, 'c, R: RandomGen, S: FeasabilitySolver<'a>> OptimizerIterator<'b, 'a, 'c, R, S> {
    fn temp_profile(max_iter: usize, k: usize) -> f64 {
        f64::max(
            1000.0 * ((max_iter as f64) - (k as f64)) / (max_iter as f64),
            0.,
        )
    }

    pub fn best_in(self, max_iter: usize) -> Option<(FeasableConfig<'a>, f64)> {
        self.take(max_iter)
            .min_by(|x, y| x.1.partial_cmp(&y.1).expect("Non NaN"))
    }
}
