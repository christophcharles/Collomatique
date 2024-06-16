use crate::ilp::initializers::ConfigInitializer;
use crate::ilp::linexpr::VariableName;
use crate::ilp::mat_repr::ProblemRepr;
use crate::ilp::random::RandomGen;
use crate::ilp::solvers::FeasabilitySolver;
use crate::ilp::{Config, FeasableConfig, Problem};

use super::MutationPolicy;

use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum Error {
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

    pub fn iterate<
        'b,
        S: FeasabilitySolver<V, P>,
        I: ConfigInitializer<V, P>,
        R: RandomGen,
        C: CrossingPolicy<V, P>,
        M: MutationPolicy<V, P>,
    >(
        &'b self,
        initializer: I,
        solver: S,
        random_gen: R,
        crossing_policy: C,
        mutation_policy: M,
    ) -> Result<OptimizerIterator<'a, 'b, V, P, S, R, C, M>> {
        let population = self.generate_init_population(initializer, &solver)?;
        Ok(OptimizerIterator {
            optimizer: self,
            solver,
            population,
            random_gen,
            crossing_policy,
            mutation_policy,
            max_steps: self.max_steps,
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
            .collect();

        let feasable_configs: Vec<_> = non_feasable_configs
            .par_iter()
            .map(|c| solver.restore_feasability_with_max_steps(c, self.init_max_steps))
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
    R: RandomGen,
    C: CrossingPolicy<V, P>,
    M: MutationPolicy<V, P>,
> {
    optimizer: &'b Optimizer<'a, V, P>,
    solver: S,
    population: Vec<FeasableConfig<'a, V, P>>,
    random_gen: R,
    crossing_policy: C,
    mutation_policy: M,
    max_steps: Option<usize>,
}

pub struct Solution<'a, V: VariableName, P: ProblemRepr<V>> {
    pub config: FeasableConfig<'a, V, P>,
    pub cost: f64,
}

impl<
        'b,
        'a: 'b,
        V: VariableName,
        P: ProblemRepr<V>,
        S: FeasabilitySolver<V, P>,
        R: RandomGen,
        C: CrossingPolicy<V, P>,
        M: MutationPolicy<V, P>,
    > Iterator for OptimizerIterator<'b, 'a, V, P, S, R, C, M>
{
    type Item = Vec<Solution<'a, V, P>>;

    fn next(&mut self) -> Option<Self::Item> {
        use rayon::prelude::*;
        let mut pop_costs: Vec<_> = self
            .population
            .par_iter()
            .map(|config| {
                let cost = (self.optimizer.problem.eval_fn)(&config);
                Solution {
                    config: config.clone(),
                    cost,
                }
            })
            .collect();

        pop_costs.par_sort_unstable_by_key(|sol| ordered_float::OrderedFloat(-sol.cost));

        // The least fit is given 1 slot
        // The second least, 2
        // The third 3, etc up to the most fit individual which is given n slots
        //
        // Then we pick a number between 0 (inclusive) and 1 + 2 + 3 + ... + n (= n(n+1)/2) (exclusive)
        // If we get 0 the first individual is preserved
        // If we get 1 or 2, the second is preserved
        // etc
        // If we get anything from n(n+1)/2)-n up to n(n+1)/2)-1, the most fit is kept.
        let pop_size = pop_costs.len();
        let max_num = (pop_size * (pop_size + 1)) / 2;

        let pop_to_keep = (pop_size + 1) / 2;

        let mut new_pop: Vec<_> = (0..pop_to_keep)
            .into_par_iter()
            .map(|_i| {
                let num = self.random_gen.rand_in_range(0..max_num);
                let index = (((1. + (num as f64)).sqrt() - 1.) / 2.).floor() as usize;

                pop_costs[index].config.clone()
            })
            .collect();

        let children: Vec<_> = new_pop
            .par_chunks_exact(2)
            .filter_map(|chunk| {
                let p1 = &chunk[0];
                let p2 = &chunk[1];

                let child1 = self.crossing_policy.cross(p1, p2);
                let child2 = self.crossing_policy.cross(p1, p2);

                let child1 = match self.mutation_policy.mutate(&child1) {
                    Some(c) => c,
                    None => child1,
                };
                let child2 = match self.mutation_policy.mutate(&child2) {
                    Some(c) => c,
                    None => child2,
                };

                let feasable_child1 = self
                    .solver
                    .restore_feasability_with_max_steps(&child1, self.max_steps)?;
                let feasable_child2 = self
                    .solver
                    .restore_feasability_with_max_steps(&child2, self.max_steps)?;

                Some([feasable_child1, feasable_child2])
            })
            .flatten()
            .collect();

        new_pop.extend(children);

        if new_pop.len() < pop_size {
            new_pop.extend(
                pop_costs[new_pop.len()..pop_size]
                    .iter()
                    .map(|sol| sol.config.clone()),
            );
        }

        self.population = new_pop;

        Some(pop_costs)
    }
}

pub trait CrossingPolicy<V: VariableName, P: ProblemRepr<V>>: Sync + Send {
    fn cross<'a>(&self, config1: &Config<'a, V, P>, config2: &Config<'a, V, P>)
        -> Config<'a, V, P>;
}

#[derive(Clone, Debug)]
pub struct RandomCrossingPolicy<R: RandomGen> {
    random_gen: R,
}

impl<R: RandomGen> RandomCrossingPolicy<R> {
    pub fn new(random_gen: R) -> Self {
        RandomCrossingPolicy { random_gen }
    }
}

impl<V: VariableName, P: ProblemRepr<V>, R: RandomGen> CrossingPolicy<V, P>
    for RandomCrossingPolicy<R>
{
    fn cross<'a>(
        &self,
        config1: &Config<'a, V, P>,
        config2: &Config<'a, V, P>,
    ) -> Config<'a, V, P> {
        assert_eq!(
            config1.get_problem() as *const _,
            config2.get_problem() as *const _
        );

        let pb = config1.get_problem();

        use std::collections::BTreeSet;
        let mut vars = BTreeSet::new();
        for var in pb.get_variables() {
            let value = if self.random_gen.randbool() {
                config1.get(var).expect("Variable should be valid")
            } else {
                config2.get(var).expect("Variable should be valid")
            };

            if value {
                vars.insert(var.clone());
            }
        }

        pb.config_from(&vars).expect("Variables should be valid")
    }
}
