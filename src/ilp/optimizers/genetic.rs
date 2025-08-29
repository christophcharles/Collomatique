use crate::ilp::initializers::ConfigInitializer;
use crate::ilp::linexpr::VariableName;
use crate::ilp::mat_repr::ProblemRepr;
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, FeasableConfig, Problem};

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
    #[error("Some init config were failed. Only {0} individuals generated")]
    InitializerFailed(usize),
    #[error("Solver failed on initial population. Only {0} individuals were solved")]
    SolverFailed(usize),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Optimizer<'a, V: VariableName, P: ProblemRepr<V>> {
    problem: &'a Problem<V, P>,
    max_steps: Option<usize>,
    init_max_steps: Option<usize>,
    population_size: usize,
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Optimizer<'a, V, P> {
    pub fn new(problem: &'a Problem<V, P>) -> Self {
        Optimizer {
            problem,
            max_steps: None,
            init_max_steps: None,
            population_size: 1000,
        }
    }

    pub fn set_max_steps(&mut self, max_steps: Option<usize>) {
        self.max_steps = max_steps;
    }

    pub fn set_init_max_steps(&mut self, init_max_steps: Option<usize>) {
        self.init_max_steps = init_max_steps;
    }

    pub fn set_population_size(&mut self, population_size: usize) {
        self.population_size = population_size;
    }

    pub fn iterate<'b, S: FeasabilitySolver<V, P>, I: ConfigInitializer<V, P>>(
        &'b self,
        initializer: I,
        solver: S,
    ) -> Result<OptimizerIterator<'a, 'b, V, P, S>> {
        let population = self.generate_init_population(initializer, &solver)?;
        Ok(OptimizerIterator {
            optimizer: self,
            solver,
            population,
        })
    }
}

impl<'a, V: VariableName, P: ProblemRepr<V>> Optimizer<'a, V, P> {
    fn generate_init_population<I: ConfigInitializer<V, P>, S: FeasabilitySolver<V, P>>(
        &self,
        initializer: I,
        solver: &S,
    ) -> Result<Vec<FeasableConfig<'a, V, P>>> {
        use rayon::prelude::*;
        let non_feasable_configs: Vec<_> = (0..self.population_size)
            .into_par_iter()
            .map(|_i| initializer.build_init_config(&self.problem))
            .while_some()
            .collect();

        let number_init = non_feasable_configs.len();
        if number_init != self.population_size {
            return Err(Error::InitializerFailed(number_init));
        }

        let feasable_configs: Vec<_> = non_feasable_configs
            .into_par_iter()
            .map(|c| solver.restore_feasability_with_max_steps(&c, self.max_steps))
            .while_some()
            .collect();

        let number_solved = feasable_configs.len();
        if number_solved != self.population_size {
            return Err(Error::SolverFailed(number_solved));
        }

        Ok(feasable_configs)
    }
}

#[derive(Debug)]
pub struct OptimizerIterator<
    'b,
    'a: 'b,
    V: VariableName,
    P: ProblemRepr<V>,
    S: FeasabilitySolver<V, P>,
> {
    optimizer: &'b Optimizer<'a, V, P>,
    solver: S,
    population: Vec<FeasableConfig<'a, V, P>>,
}
